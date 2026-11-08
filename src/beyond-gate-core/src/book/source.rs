//! Cookie-only page reads gated by fresh library evidence and one session identity.

use std::{
    collections::{HashMap, HashSet},
    time::Duration,
};

use percent_encoding::{NON_ALPHANUMERIC, percent_decode_str, utf8_percent_encode};
use reqwest::Url;
use scraper::{ElementRef, Html, Selector};
use sha2::{Digest, Sha256};
use tokio::time::{Instant, timeout_at};
use zeroize::Zeroizing;

use super::{
    BookError, BookOutline, MAX_OUTLINE_ENTRIES, OutlineEntryRecord, OutlineRecord, SectionChunk,
    SectionRecord, SectionRequest, assemble_outline, read_section,
};
use crate::{
    auth::session::Session,
    http::HttpClient,
    library::{self, AccessState, BookAccess},
};

/// Exact website origin for source pages, never an origin supplied by a token.
const WEBSITE: &str = "https://www.dndbeyond.com";

/// Holds only freshly selected metadata and a bounded transport for this operation.
struct Source {
    root: Url,
    client: HttpClient,
    #[cfg(any(test, feature = "session-fixture"))]
    fixture_origin: Option<String>,
}

impl Source {
    /// Requires current positive account evidence without treating it as page access.
    async fn resolve(session: &Session, book_id: &str) -> Result<Self, BookError> {
        let snapshot = library::discover_library(session)
            .await
            .map_err(BookError::LibraryRead)?;
        let book = snapshot
            .books()
            .iter()
            .find(|book| book.id() == book_id)
            .ok_or(BookError::Unavailable)?;
        match book.access() {
            BookAccess::Purchased | BookAccess::Shared | BookAccess::PurchasedAndShared => {}
            BookAccess::Unavailable => return Err(BookError::AccessDenied),
            BookAccess::Unknown => return Err(BookError::AccessUnknown),
        }
        let root = book.source_url().clone();
        if root.origin().ascii_serialization() != WEBSITE
            || root.query().is_some()
            || root.fragment().is_some()
            || !root.username().is_empty()
            || root.password().is_some()
            || !root.path().starts_with("/sources/")
            || !safe_path(root.path())
        {
            return Err(BookError::UnsupportedSection);
        }
        #[cfg(any(test, feature = "session-fixture"))]
        if let Some(fixture) = &session.library_fixture {
            let origin = Url::parse(&fixture.config)
                .map_err(|_| BookError::MalformedSource)?
                .origin()
                .ascii_serialization();
            return Ok(Self {
                root,
                client: fixture.client.clone(),
                fixture_origin: Some(origin),
            });
        }
        Ok(Self {
            root,
            client: HttpClient::single_attempt(&[WEBSITE]).map_err(BookError::SourceRead)?,
            #[cfg(any(test, feature = "session-fixture"))]
            fixture_origin: None,
        })
    }

    /// Sends one page request after clearing its local fragment, not after reading a cursor.
    async fn page(
        &self,
        session: &Session,
        selected: &Url,
    ) -> Result<Zeroizing<Vec<u8>>, BookError> {
        let mut target = selected.clone();
        target.set_fragment(None);
        #[cfg(any(test, feature = "session-fixture"))]
        if let Some(origin) = &self.fixture_origin {
            return session
                .get_book_html(&self.client, &format!("{origin}{}", target.path()))
                .await
                .map_err(BookError::SessionRead);
        }
        session
            .get_book_html(&self.client, target.as_str())
            .await
            .map_err(BookError::SessionRead)
    }

    /// Validates the current source page before producing any navigation.
    async fn navigation(&self, session: &Session, book_id: &str) -> Result<BookOutline, BookError> {
        let bytes = self.page(session, &self.root).await?;
        parse_outline(&bytes, &self.root, book_id)
    }
}

/// Rejects noncanonical numeric library IDs before storage or network activity.
fn validate_book_id(id: &str) -> Result<(), BookError> {
    if id
        .parse::<u64>()
        .ok()
        .filter(|value| *value != 0)
        .is_none_or(|value| value.to_string() != id)
    {
        return Err(BookError::InvalidMetadata);
    }
    Ok(())
}

/// Bounds the entire fresh metadata and source-page operation to thirty seconds.
pub(super) async fn outline(session: &Session, id: &str) -> Result<BookOutline, BookError> {
    validate_book_id(id)?;
    let deadline = Instant::now() + Duration::from_secs(30);
    let session = session.read_scope();
    let result = timeout_at(deadline, async {
        Source::resolve(&session, id)
            .await?
            .navigation(&session, id)
            .await
    })
    .await
    .map_err(|_| BookError::TimedOut)?;
    if Instant::now() >= deadline {
        return Err(BookError::TimedOut);
    }
    result
}

/// Resolves only the explicit selection, then reuses the bounded offline chunk contract.
pub(super) async fn read(
    session: &Session,
    request: SectionRequest<'_>,
) -> Result<SectionChunk, BookError> {
    validate_book_id(request.book_id)?;
    if request.section_id.len() != 66
        || !request.section_id.starts_with("s-")
        || !request.section_id[2..]
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
    {
        return Err(BookError::InvalidMetadata);
    }
    if request.continuation.is_some_and(|cursor| {
        cursor.book_id != request.book_id || cursor.section_id != request.section_id
    }) {
        return Err(BookError::InvalidContinuation);
    }
    let deadline = Instant::now() + Duration::from_secs(30);
    let session = session.read_scope();
    let result = timeout_at(deadline, async {
        let source = Source::resolve(&session, request.book_id).await?;
        let outline = source.navigation(&session, request.book_id).await?;
        let selected = outline
            .entries()
            .iter()
            .find(|entry| entry.id() == request.section_id)
            .ok_or(if request.continuation.is_some() {
                BookError::StaleContinuation
            } else {
                BookError::UnsupportedSection
            })?;
        let bytes = source.page(&session, selected.source_url()).await?;
        let text = parse_section(&bytes, selected.source_url())?;
        let record = SectionRecord {
            book_id: request.book_id.into(),
            section_id: request.section_id.into(),
            revision: digest(&[
                request.book_id,
                request.section_id,
                selected.source_url().as_str(),
                &text,
            ]),
            source_url: selected.source_url().to_string(),
            text,
        };
        read_section(request, |_, _| Ok(AccessState::Granted), |_, _| Ok(record))
    })
    .await
    .map_err(|_| BookError::TimedOut)?;
    if Instant::now() >= deadline {
        return Err(BookError::TimedOut);
    }
    result
}

/// Parses fixed selectors without including source values in diagnostics.
fn selector(value: &str) -> Result<Selector, BookError> {
    Selector::parse(value).map_err(|_| BookError::MalformedSource)
}

/// Requires the exact canonical document, no base override, and no known challenge.
fn document(bytes: &[u8], expected: &Url) -> Result<Html, BookError> {
    if bytes.len() > 1024 * 1024 {
        return Err(BookError::LimitExceeded);
    }
    let text = std::str::from_utf8(bytes).map_err(|_| BookError::MalformedSource)?;
    let document = Html::parse_document(text);
    if document
        .select(&selector(
            "#challenge-form, #challenge-running, script[src*='/cdn-cgi/challenge-platform/']",
        )?)
        .next()
        .is_some()
    {
        return Err(BookError::AccessUnknown);
    }
    if document.select(&selector("base")?).next().is_some() {
        return Err(BookError::MalformedSource);
    }
    let canonical = selector("link[rel~='canonical']")?;
    let mut links = document.select(&canonical);
    let href = links
        .next()
        .and_then(|link| link.attr("href"))
        .ok_or(BookError::MalformedSource)?;
    if href.is_empty() || href.trim() != href || href.chars().any(char::is_control) {
        return Err(BookError::MalformedSource);
    }
    let location = expected
        .join(href)
        .map_err(|_| BookError::MalformedSource)?;
    let mut expected = expected.clone();
    expected.set_fragment(None);
    if links.next().is_some() || location != expected {
        return Err(BookError::MalformedSource);
    }
    Ok(document)
}

/// Rejects explicitly hidden content, including a hidden ancestor.
fn hidden(element: ElementRef<'_>) -> bool {
    std::iter::once(element)
        .chain(element.ancestors().filter_map(ElementRef::wrap))
        .any(|element| {
            element.attr("hidden").is_some()
                || element
                    .attr("aria-hidden")
                    .is_some_and(|value| value.eq_ignore_ascii_case("true"))
                || element.attr("style").is_some_and(|style| {
                    let style: String = style
                        .chars()
                        .filter(|value| !value.is_whitespace())
                        .collect();
                    let style = style.to_ascii_lowercase();
                    style.contains("display:none") || style.contains("visibility:hidden")
                })
        })
}

/// Requires one visible container rather than selecting the first ambiguous match.
fn container<'a>(document: &'a Html, query: &str) -> Result<ElementRef<'a>, BookError> {
    let query = selector(query)?;
    let mut roots = document.select(&query);
    let root = roots.next().ok_or(BookError::AccessUnknown)?;
    if roots.next().is_some() {
        return Err(BookError::MalformedSource);
    }
    if hidden(root) {
        return Err(BookError::AccessUnknown);
    }
    Ok(root)
}

/// Accepts only literal route segments; encoded separators and dot paths never select a fetch.
fn safe_path(path: &str) -> bool {
    path.starts_with('/')
        && path.split('/').skip(1).all(|part| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        })
}

/// Decodes a well-formed UTF-8 fragment once, preserving its exact DOM identifier.
fn fragment(value: &str) -> Result<String, BookError> {
    let bytes = value.as_bytes();
    for (index, byte) in bytes.iter().enumerate() {
        if *byte == b'%'
            && (index + 2 >= bytes.len()
                || !bytes[index + 1].is_ascii_hexdigit()
                || !bytes[index + 2].is_ascii_hexdigit())
        {
            return Err(BookError::MalformedSource);
        }
    }
    let value = percent_decode_str(value)
        .decode_utf8()
        .map_err(|_| BookError::MalformedSource)?;
    if !library::valid_text(&value) {
        return Err(BookError::MalformedSource);
    }
    Ok(value.into_owned())
}

/// Classifies same-book links and safe non-source auxiliary links without fetching either.
fn target(root: &Url, href: &str) -> Result<Option<Url>, BookError> {
    if href.is_empty()
        || href.trim() != href
        || href.chars().any(char::is_control)
        || href.contains('\\')
        || href
            .split(['/', '?', '#'])
            .any(|part| matches!(part, "." | ".."))
        || href
            .split('#')
            .next()
            .is_some_and(|path| path.contains('%'))
    {
        return Err(BookError::MalformedSource);
    }
    let mut target = root.join(href).map_err(|_| BookError::MalformedSource)?;
    if target.scheme() != "https"
        || !target.username().is_empty()
        || target.password().is_some()
        || target.port().is_some()
        || target.query().is_some()
    {
        return Err(BookError::MalformedSource);
    }
    let prefix = format!("{}/", root.path());
    if target.origin() == root.origin() {
        let root_parts: Vec<_> = root.path().split('/').collect();
        if root_parts.len() == 4 && root_parts[2] == "dnd" {
            let legacy = format!("/sources/{}/", root_parts[3]);
            if let Some(chapter) = target.path().strip_prefix(&legacy) {
                let canonical = format!("{prefix}{chapter}");
                target.set_path(&canonical);
            }
        }
        if target.path().starts_with(&prefix) {
            if !safe_path(target.path()) || target.path()[prefix.len()..].contains('/') {
                return Err(BookError::MalformedSource);
            }
            if let Some(value) = target.fragment() {
                let decoded = fragment(value)?;
                target.set_fragment(Some(
                    &utf8_percent_encode(&decoded, NON_ALPHANUMERIC).to_string(),
                ));
            }
            return Ok(Some(target));
        }
        if target.path() == root.path() && target.fragment().is_none() {
            return Ok(None);
        }
    }
    if target.path().starts_with("/sources/") {
        return Err(BookError::MalformedSource);
    }
    Ok(None)
}

/// Builds stable opaque identities from length-separated values without exposing a URL in an ID.
fn digest(parts: &[&str]) -> String {
    let mut digest = Sha256::new();
    for part in parts {
        digest.update((part.len() as u64).to_be_bytes());
        digest.update(part.as_bytes());
    }
    format!("{:x}", digest.finalize())
}

/// Counts all anchors before filtering, retaining repeated navigation roles with distinct IDs.
fn parse_outline(bytes: &[u8], root: &Url, book_id: &str) -> Result<BookOutline, BookError> {
    let document = document(bytes, root)?;
    let toc = container(&document, ".compendium-toc-full-text")?;
    let query = selector("a")?;
    let mut occurrences = HashMap::new();
    let mut anchors = HashMap::new();
    let mut entries = Vec::new();
    for (index, anchor) in toc.select(&query).enumerate() {
        if index >= MAX_OUTLINE_ENTRIES {
            return Err(BookError::LimitExceeded);
        }
        let href = anchor.attr("href").ok_or(BookError::MalformedSource)?;
        let Some(url) = target(root, href)? else {
            continue;
        };
        if hidden(anchor) {
            return Err(BookError::MalformedSource);
        }
        let label = anchor.text().collect::<String>();
        let title = label.split_whitespace().collect::<Vec<_>>().join(" ");
        if !library::valid_text(&title) {
            return Err(BookError::MalformedSource);
        }
        let count = occurrences.entry(url.to_string()).or_insert(0_usize);
        let id = format!("s-{}", digest(&[book_id, url.as_str(), &count.to_string()]));
        *count += 1;
        anchors.insert(anchor.id(), entries.len());
        entries.push(OutlineEntryRecord {
            id,
            title,
            parent_id: None,
            source_url: url.to_string(),
        });
    }
    if entries.is_empty() {
        return Err(BookError::MalformedSource);
    }
    let mut chapters = HashMap::new();
    for entry in &entries {
        let url = Url::parse(&entry.source_url).map_err(|_| BookError::MalformedSource)?;
        if url.fragment().is_none() {
            chapters
                .entry(url.path().to_owned())
                .or_insert(entry.id.clone());
        }
    }
    let mut latest = HashMap::new();
    for entry in &mut entries {
        let url = Url::parse(&entry.source_url).map_err(|_| BookError::MalformedSource)?;
        if url.fragment().is_none() {
            latest.insert(url.path().to_owned(), entry.id.clone());
        } else {
            entry.parent_id = Some(
                latest
                    .get(url.path())
                    .or_else(|| chapters.get(url.path()))
                    .ok_or(BookError::InvalidOutline)?
                    .clone(),
            );
        }
    }
    for anchor in toc.select(&query) {
        let Some(&child) = anchors.get(&anchor.id()) else {
            continue;
        };
        let parent = anchor
            .ancestors()
            .filter_map(ElementRef::wrap)
            .filter(|element| element.value().name() == "li")
            .nth(1);
        if let Some(parent) = parent {
            let direct: Vec<_> = parent
                .select(&query)
                .filter(|candidate| {
                    candidate
                        .ancestors()
                        .filter_map(ElementRef::wrap)
                        .find(|element| element.value().name() == "li")
                        .is_some_and(|element| element.id() == parent.id())
                })
                .filter_map(|candidate| anchors.get(&candidate.id()))
                .collect();
            if direct.len() > 1 {
                return Err(BookError::InvalidOutline);
            }
            if let Some(&&parent) = direct.first() {
                let id = entries[parent].id.clone();
                entries[child].parent_id = Some(id);
            }
        }
    }
    assemble_outline(
        book_id,
        |_| Ok(AccessState::Granted),
        |_| {
            Ok(OutlineRecord {
                book_id: book_id.into(),
                source_url: root.to_string(),
                entries,
            })
        },
    )
}

/// Bounds all article nodes and outer ancestors before repeated visibility checks.
fn validate_section_bounds(article: ElementRef<'_>) -> Result<(), BookError> {
    if article.ancestors().take(257).count() > 256 {
        return Err(BookError::LimitExceeded);
    }
    let mut stack = vec![(*article, 0_usize)];
    let mut elements = 0;
    while let Some((node, depth)) = stack.pop() {
        if depth > 256 {
            return Err(BookError::LimitExceeded);
        }
        if ElementRef::wrap(node).is_some() {
            elements += 1;
            if elements > 100_000 {
                return Err(BookError::LimitExceeded);
            }
        }
        for child in node.children().rev() {
            stack.push((child, depth + 1));
        }
    }
    Ok(())
}

/// Resolves an article link without fetching it and identifies first-party purchase destinations.
fn marketplace_link(selected: &Url, href: &str) -> bool {
    selected.join(href).is_ok_and(|target| {
        target.host_str() == Some("marketplace.dndbeyond.com")
            || (target.host_str() == Some("www.dndbeyond.com")
                && (target.path() == "/marketplace" || target.path().starts_with("/marketplace/")))
    })
}

/// Checks unique article IDs and selects only a heading range or its explicit chapter.
fn parse_section(bytes: &[u8], selected: &Url) -> Result<String, BookError> {
    let document = document(bytes, selected)?;
    let article = container(&document, ".p-article-content")?;
    validate_section_bounds(article)?;
    if article.select(&selector("h1")?).count() != 1 {
        return Err(BookError::MalformedSource);
    }
    let mut ids = HashSet::new();
    let mut selected_node = None;
    let selected_id = selected.fragment().map(fragment).transpose()?;
    for element in std::iter::once(article).chain(
        article
            .descendants()
            .filter_map(ElementRef::wrap)
            .filter(|element| element.id() != article.id()),
    ) {
        if super::text::heading(element).is_some() && hidden(element) {
            return Err(BookError::AccessUnknown);
        }
        if let Some(id) = element.attr("id") {
            if !library::valid_text(id) || !ids.insert(id) {
                return Err(BookError::MalformedSource);
            }
            if selected_id.as_deref() == Some(id) {
                selected_node = Some(element);
            }
        }
        if element.value().name() == "a"
            && element
                .attr("href")
                .is_some_and(|value| marketplace_link(selected, value))
        {
            return Err(BookError::AccessUnknown);
        }
    }
    let heading = if selected_id.is_some() {
        let node = selected_node.ok_or(BookError::UnsupportedSection)?;
        if hidden(node) {
            return Err(BookError::AccessUnknown);
        }
        Some(
            std::iter::once(node)
                .chain(node.ancestors().filter_map(ElementRef::wrap))
                .take_while(|element| element.id() != article.id())
                .find(|element| super::text::heading(*element).is_some())
                .ok_or(BookError::UnsupportedSection)?,
        )
    } else {
        None
    };
    super::text::render(article, heading)
}

#[cfg(test)]
mod tests;
