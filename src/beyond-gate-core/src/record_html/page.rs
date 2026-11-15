//! Checked content boundaries for observed background, feat, and class pages.

use super::HtmlError;
use scraper::{ElementRef, Html, Selector};

/// Selects an observed website layout, not an arbitrary URL or CSS selector.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum PageKind {
    /// Record-specific background container with description and source footer.
    Background,
    /// Record-specific feat container with description and source footer.
    Feat,
    /// Primary class content, excluding adjacent subclass records.
    Class,
    /// One exact fragment inside a bounded subclass record.
    Subclass,
}

/// Compiles only internal structural selectors.
fn selector(text: &str) -> Result<Selector, HtmlError> {
    Selector::parse(text).map_err(|_| HtmlError::InvalidResponse)
}

/// Requires one known container rather than accepting missing or duplicate structure.
fn one<'a>(mut nodes: impl Iterator<Item = ElementRef<'a>>) -> Result<ElementRef<'a>, HtmlError> {
    let result = nodes.next().ok_or(HtmlError::InvalidResponse)?;
    if nodes.next().is_some() {
        return Err(HtmlError::InvalidResponse);
    }
    Ok(result)
}

/// Bounds every DOM node before any HTML serialization or recursive text access.
fn bounds(document: &Html) -> Result<(), HtmlError> {
    if document.tree.nodes().count() > 100_000
        || document
            .tree
            .nodes()
            .any(|node| node.ancestors().take(257).count() > 256)
    {
        Err(HtmlError::LimitExceeded)
    } else {
        Ok(())
    }
}

/// Rejects active elements, event attributes, and unrelated account or purchase UI.
fn check_element(element: ElementRef<'_>) -> Result<(), HtmlError> {
    if matches!(
        element.value().name(),
        "script"
            | "form"
            | "iframe"
            | "object"
            | "embed"
            | "nav"
            | "svg"
            | "math"
            | "style"
            | "link"
            | "meta"
            | "base"
            | "input"
            | "button"
            | "textarea"
            | "select"
    ) {
        return Err(HtmlError::UnsupportedSource(vec![
            "active_content".to_owned(),
        ]));
    }
    for (name, value) in element.value().attrs() {
        if name.starts_with("on") || name == "srcdoc" {
            return Err(HtmlError::UnsupportedSource(vec![
                "active_attribute".to_owned(),
            ]));
        }
        if name == "style" {
            let compact: String = value
                .chars()
                .filter(|character| !character.is_whitespace())
                .collect();
            let compact = compact.to_ascii_lowercase();
            if !value.is_ascii()
                || value.contains(['\\', '@', '/', '<', '>'])
                || ["url(", "expression(", "javascript:"]
                    .iter()
                    .any(|marker| compact.contains(marker))
            {
                return Err(HtmlError::UnsupportedSource(vec![
                    "active_style".to_owned(),
                ]));
            }
        }
        if matches!(name, "id" | "class")
            && [
                "comment",
                "account",
                "purchase",
                "unlock",
                "login",
                "paywall",
                "challenge",
            ]
            .iter()
            .any(|marker| value.to_ascii_lowercase().contains(marker))
        {
            return Err(HtmlError::UnsupportedSource(vec![
                "unrelated_content".to_owned(),
            ]));
        }
        if matches!(name, "href" | "src") {
            let normalized = value.trim().to_ascii_lowercase();
            if normalized.chars().any(char::is_control)
                || !(normalized.starts_with("https://")
                    || normalized.starts_with("http://")
                    || normalized.starts_with('/')
                    || normalized.starts_with('#'))
            {
                return Err(HtmlError::UnsupportedSource(vec!["active_link".to_owned()]));
            }
        }
    }
    Ok(())
}

/// Validates an independently bounded inline field without accepting a whole document.
pub(crate) fn validate_fragment(content: &str) -> Result<(), HtmlError> {
    if content.len() > 1024 * 1024 {
        return Err(HtmlError::LimitExceeded);
    }
    if content
        .chars()
        .any(|character| character.is_control() && !matches!(character, '\n' | '\r' | '\t'))
    {
        return Err(HtmlError::InvalidResponse);
    }
    let lower = content.to_ascii_lowercase();
    let document_tag = ["<html", "<head", "<body"].iter().any(|marker| {
        lower.match_indices(marker).any(|(index, _)| {
            lower
                .as_bytes()
                .get(index + marker.len())
                .is_some_and(|byte| byte.is_ascii_whitespace() || matches!(byte, b'>' | b'/'))
        })
    });
    if document_tag || lower.contains("<!doctype") {
        return Err(HtmlError::UnsupportedSource(vec![
            "whole_document".to_owned(),
        ]));
    }
    let document = Html::parse_fragment(content);
    bounds(&document)?;
    for element in document.tree.nodes().filter_map(ElementRef::wrap) {
        check_element(element)?;
    }
    Ok(())
}

/// Reads required visible identity text without interpreting HTML as instructions.
fn text(node: ElementRef<'_>) -> Result<String, HtmlError> {
    let text = node
        .text()
        .collect::<Vec<_>>()
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if text.is_empty() || text.chars().any(char::is_control) {
        Err(HtmlError::InvalidResponse)
    } else {
        Ok(text)
    }
}

/// Checks canonical identity before selecting only the resource's content subtree.
pub(crate) fn selected(
    bytes: &[u8],
    kind: PageKind,
    url: &reqwest::Url,
    name: &str,
) -> Result<String, HtmlError> {
    if bytes.len() > 1024 * 1024 {
        return Err(HtmlError::LimitExceeded);
    }
    let body = std::str::from_utf8(bytes).map_err(|_| HtmlError::InvalidResponse)?;
    let document = Html::parse_document(body);
    bounds(&document)?;
    if document
        .select(&selector(
            "#challenge-form, script[src*='/cdn-cgi/challenge-platform/']",
        )?)
        .next()
        .is_some()
    {
        return Err(HtmlError::UnsupportedSource(vec![
            "challenge_page".to_owned(),
        ]));
    }
    let mut expected = url.clone();
    expected.set_fragment(None);
    let canonical = one(document.select(&selector("link[rel='canonical']")?))?;
    if canonical.value().attr("href") != Some(expected.as_str()) {
        return Err(HtmlError::InvalidResponse);
    }
    let heading = one(document.select(&selector("h1.page-title")?))?;
    if !matches!(kind, PageKind::Subclass)
        && text(heading)? != name.split_whitespace().collect::<Vec<_>>().join(" ")
    {
        return Err(HtmlError::InvalidResponse);
    }
    let content = match kind {
        PageKind::Background | PageKind::Feat => annotated(&document, kind, url)?,
        PageKind::Class | PageKind::Subclass => class_content(&document, kind, url, name)?,
    };
    validate_fragment(&content)?;
    Ok(content)
}

/// Uses the same observed record-specific description/footer layout for two resources.
fn annotated(document: &Html, kind: PageKind, url: &reqwest::Url) -> Result<String, HtmlError> {
    let resource = if kind == PageKind::Background {
        "background"
    } else {
        "feat"
    };
    let route = if kind == PageKind::Background {
        "/backgrounds/"
    } else {
        "/feats/"
    };
    let slug = url
        .path()
        .strip_prefix(route)
        .ok_or(HtmlError::InvalidResponse)?;
    let container = one(document.select(&selector(&format!(
        ".details-container.details-container-{resource}"
    ))?))?;
    let prefix = format!("details-container-{resource}-");
    let identities: Vec<_> = container
        .value()
        .classes()
        .filter(|class| class.starts_with(&prefix))
        .collect();
    if identities != [format!("{prefix}{slug}")] {
        return Err(HtmlError::InvalidResponse);
    }
    let content = one(container.select(&selector(".details-container-content")?))?;
    let description = one(content.select(&selector(".details-container-content-description")?))?;
    text(one(description.select(&selector(
        ".details-container-content-description-text",
    )?))?)?;
    let footer = one(content.select(&selector(".details-container-content-footer")?))?;
    text(one(footer.select(&selector(".source-description")?))?)?;
    Ok(content.html())
}

/// Keeps primary class content separate from all adjacent subclass records.
fn class_content(
    document: &Html,
    kind: PageKind,
    url: &reqwest::Url,
    name: &str,
) -> Result<String, HtmlError> {
    let root = one(document.select(&selector(".static-container.static-container-details")?))?;
    let content = one(root.children().filter_map(ElementRef::wrap).filter(|node| {
        node.value()
            .has_class("content-container", scraper::CaseSensitivity::CaseSensitive)
    }))?;
    if let Some(fragment) = url.fragment() {
        let nodes = selector("[id], a[name]")?;
        let targets: Vec<_> = content
            .select(&nodes)
            .filter(|node| {
                node.value().attr("id") == Some(fragment)
                    || node.value().attr("name") == Some(fragment)
            })
            .collect();
        if targets.is_empty() {
            return Err(HtmlError::UnsupportedSource(vec![
                "missing_fragment".to_owned(),
            ]));
        }
        let target = fragment_heading(one(targets.into_iter())?)?;
        let subclass = target
            .ancestors()
            .filter_map(ElementRef::wrap)
            .find(|node| {
                node.value().has_class(
                    "subitems-list-details-item",
                    scraper::CaseSensitivity::CaseSensitive,
                )
            });
        if kind == PageKind::Subclass {
            let selected = subclass.ok_or(HtmlError::InvalidResponse)?;
            let heading = one(selected.select(&selector(
                ".subitems-list-details-header h2, .subitems-list-details-header h3",
            )?))?;
            if text(heading)? != name.split_whitespace().collect::<Vec<_>>().join(" ") {
                return Err(HtmlError::InvalidResponse);
            }
            if matches!(target.value().name(), "h1" | "h2" | "h3") {
                return Ok(selected.html());
            }
            return heading_range(target);
        }
        if subclass.is_some() {
            return Err(HtmlError::InvalidResponse);
        }
        return heading_range(target);
    }
    if kind == PageKind::Subclass {
        return Err(HtmlError::UnsupportedSource(vec![
            "missing_fragment".to_owned(),
        ]));
    }
    let children: Vec<_> = content.children().filter_map(ElementRef::wrap).collect();
    let start = children
        .iter()
        .position(|node| node.value().name() == "h1")
        .ok_or(HtmlError::InvalidResponse)?;
    let mut end = children
        .iter()
        .position(|node| {
            node.value().has_class(
                "subitems-list-details",
                scraper::CaseSensitivity::CaseSensitive,
            )
        })
        .ok_or(HtmlError::InvalidResponse)?;
    if end <= start {
        return Err(HtmlError::InvalidResponse);
    }
    if children[end - 1].value().name() == "h2" {
        end -= 1;
    }
    if end <= start {
        return Err(HtmlError::InvalidResponse);
    }
    Ok(children[start..end]
        .iter()
        .map(ElementRef::html)
        .collect::<Vec<_>>()
        .join("\n"))
}

/// Resolves an exact heading, its inner anchor, or an adjacent named anchor.
fn fragment_heading(target: ElementRef<'_>) -> Result<ElementRef<'_>, HtmlError> {
    let target = if heading_rank(target).is_some() {
        target
    } else if let Some(parent) = target
        .parent()
        .and_then(ElementRef::wrap)
        .filter(|node| heading_rank(*node).is_some())
    {
        parent
    } else if target.value().name() == "a" && target.value().attr("name").is_some() {
        target
            .next_siblings()
            .filter_map(ElementRef::wrap)
            .next()
            .filter(|node| heading_rank(*node).is_some())
            .ok_or_else(|| HtmlError::UnsupportedSource(vec!["fragment_structure".to_owned()]))?
    } else {
        return Err(HtmlError::UnsupportedSource(vec![
            "fragment_structure".to_owned(),
        ]));
    };
    Ok(target)
}

/// Selects one heading and its following siblings without crossing the next peer section.
fn heading_range(target: ElementRef<'_>) -> Result<String, HtmlError> {
    let rank = heading_rank(target)
        .ok_or_else(|| HtmlError::UnsupportedSource(vec!["fragment_structure".to_owned()]))?;
    let mut result = target.html();
    for sibling in target.next_siblings().filter_map(ElementRef::wrap) {
        if heading_rank(sibling).is_some_and(|next| next <= rank)
            || sibling.value().has_class(
                "subitems-list-details",
                scraper::CaseSensitivity::CaseSensitive,
            )
        {
            break;
        }
        result.push('\n');
        result.push_str(&sibling.html());
        if result.len() > 1024 * 1024 {
            return Err(HtmlError::LimitExceeded);
        }
    }
    Ok(result)
}

/// Reads a standard heading rank without guessing a fragment's meaning.
fn heading_rank(node: ElementRef<'_>) -> Option<u8> {
    match node.value().name() {
        "h1" => Some(1),
        "h2" => Some(2),
        "h3" => Some(3),
        "h4" => Some(4),
        "h5" => Some(5),
        "h6" => Some(6),
        _ => None,
    }
}
