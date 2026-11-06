//! Separately approved source/chapter structure inspection with a final anonymous control.
//!
//! HTML stays in memory. Only fixed labels and numeric observations leave the
//! parser. These diagnostics do not implement a reader or grant content access.

use super::*;
use scraper::{ElementRef, Html, Selector};
use std::collections::BTreeMap;

/// Separates these three ordered requests from both consumed one-request scopes.
const APPROVAL: &str = "issue-33-protected-structure-three-attempts";
/// Selects the previously observed source landing page, not another book.
const SOURCE: &str = "https://www.dndbeyond.com/sources/dnd/cos";
/// Selects the non-preview chapter supplied by the user.
const CHAPTER: &str = "https://www.dndbeyond.com/sources/dnd/cos/into-the-mists";
/// Prevents response-derived keys or values from entering diagnostic output.
type Counts = BTreeMap<&'static str, usize>;

/// Holds bounded HTML or a final control refusal without formatting response content.
enum Page {
    Html(Zeroizing<Vec<u8>>),
    Restricted(u16),
}

impl fmt::Debug for Page {
    /// Prints byte counts and statuses only, even when a test assertion fails.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Html(bytes) => write!(formatter, "Html {{ bytes: {} }}", bytes.len()),
            Self::Restricted(status) => write!(formatter, "Restricted {{ status: {status} }}"),
        }
    }
}

/// Parses fixed CSS selectors without displaying dependency errors or HTML.
fn selector(value: &str) -> Result<Selector, ProbeError> {
    Selector::parse(value).map_err(|_| ProbeError::Shape)
}

/// Requires the selected canonical document and rejects known challenge elements.
fn document(bytes: &[u8], expected: &str) -> Result<Html, ProbeError> {
    if bytes.len() > MAX_BYTES {
        return Err(ProbeError::BodyLimit);
    }
    let text = std::str::from_utf8(bytes).map_err(|_| ProbeError::Shape)?;
    let document = Html::parse_document(text);
    let challenge = selector(
        "#challenge-form, #challenge-running, script[src*='/cdn-cgi/challenge-platform/']",
    )?;
    if document.select(&challenge).next().is_some() {
        return Err(ProbeError::Shape);
    }
    let canonical = selector("link[rel~='canonical']")?;
    let mut links = document.select(&canonical);
    let href = links
        .next()
        .and_then(|link| link.value().attr("href"))
        .ok_or(ProbeError::SourcePath)?;
    let target = Url::parse(expected)
        .and_then(|base| base.join(href))
        .map_err(|_| ProbeError::SourcePath)?;
    if links.next().is_some() || target.as_str() != expected {
        return Err(ProbeError::SourcePath);
    }
    Ok(document)
}

/// Counts all raw outline links and retains only private, transient selected-chapter fragments.
fn source_structure(bytes: &[u8]) -> Result<(Counts, BTreeSet<String>), ProbeError> {
    let document = document(bytes, SOURCE)?;
    let roots = selector(".compendium-toc-full-text")?;
    let mut roots = document.select(&roots);
    let root = roots.next().ok_or(ProbeError::Shape)?;
    if roots.next().is_some() {
        return Err(ProbeError::Shape);
    }
    let anchors = selector("a")?;
    let base = Url::parse(SOURCE).map_err(|_| ProbeError::SourcePath)?;
    let mut targets = BTreeSet::new();
    let mut fragments = BTreeSet::new();
    let mut raw = 0;
    let mut duplicate = 0;
    let mut empty_labels = 0;
    let mut chapters = 0;
    let mut sections = 0;
    let mut root_links = 0;
    let mut other = 0;
    let mut unsafe_links = 0;
    let mut legacy = 0;
    for anchor in root.select(&anchors) {
        raw += 1;
        if raw > MAX_RECORDS {
            return Err(ProbeError::RecordLimit);
        }
        if !anchor.text().any(|text| !text.trim().is_empty()) {
            empty_labels += 1;
        }
        let Some(href) = anchor.value().attr("href").filter(|href| !href.is_empty()) else {
            unsafe_links += 1;
            continue;
        };
        let Ok(target) = base.join(href) else {
            unsafe_links += 1;
            continue;
        };
        if !targets.insert(target.to_string()) {
            duplicate += 1;
        }
        if target.scheme() != "https"
            || !target.username().is_empty()
            || target.password().is_some()
            || target.port().is_some()
            || target.query().is_some()
        {
            unsafe_links += 1;
            continue;
        }
        let is_legacy = target.path().starts_with("/sources/cos/");
        let in_book = target.host_str() == Some("www.dndbeyond.com")
            && (target.path() == "/sources/dnd/cos"
                || target.path().starts_with("/sources/dnd/cos/")
                || is_legacy);
        if !in_book {
            other += 1;
            continue;
        }
        legacy += usize::from(is_legacy);
        if target.fragment().is_some() {
            sections += 1;
        } else if target.path() == "/sources/dnd/cos" {
            root_links += 1;
        } else {
            chapters += 1;
        }
        if matches!(
            target.path(),
            "/sources/dnd/cos/into-the-mists" | "/sources/cos/into-the-mists"
        ) && let Some(fragment) = target.fragment().filter(|fragment| !fragment.is_empty())
        {
            fragments.insert(fragment.to_owned());
        }
    }
    Ok((
        BTreeMap::from([
            ("canonical_match", 1),
            ("outline_containers", 1),
            ("raw_links", raw),
            ("duplicate_targets", duplicate),
            ("empty_labels", empty_labels),
            ("chapter_links", chapters),
            ("fragment_links", sections),
            ("root_links", root_links),
            ("other_links", other),
            ("unsafe_links", unsafe_links),
            ("observed_legacy_links", legacy),
            ("selected_chapter_fragments", fragments.len()),
        ]),
        fragments,
    ))
}

/// Detects explicit hidden ancestors without inferring access from arbitrary CSS names.
fn hidden(element: ElementRef<'_>) -> bool {
    std::iter::once(element)
        .chain(element.ancestors().filter_map(ElementRef::wrap))
        .any(|element| {
            element.value().attr("hidden").is_some()
                || element.value().attr("aria-hidden") == Some("true")
        })
}

/// Counts article structure and target matches without returning identifiers or text.
fn chapter_structure(bytes: &[u8], fragments: &BTreeSet<String>) -> Result<Counts, ProbeError> {
    let document = document(bytes, CHAPTER)?;
    let articles = selector(".p-article-content")?;
    let mut articles = document.select(&articles);
    let article = articles.next().ok_or(ProbeError::Shape)?;
    if articles.next().is_some() || hidden(article) {
        return Err(ProbeError::Shape);
    }
    let headings = selector("h1, h2, h3, h4, h5, h6")?;
    let mut levels = [0; 6];
    let mut heading_ids = 0;
    for heading in article.select(&headings) {
        let index = match heading.value().name() {
            "h1" => 0,
            "h2" => 1,
            "h3" => 2,
            "h4" => 3,
            "h5" => 4,
            "h6" => 5,
            _ => return Err(ProbeError::Shape),
        };
        levels[index] += 1;
        heading_ids += usize::from(heading.value().attr("id").is_some());
    }
    let mut ids = BTreeSet::new();
    let mut duplicate_ids = 0;
    for element in article.select(&selector("[id]")?) {
        if let Some(id) = element.value().attr("id")
            && !ids.insert(id)
        {
            duplicate_ids += 1;
        }
    }
    let mut purchase_links = 0;
    for anchor in article.select(&selector("a[href]")?) {
        if let Some(href) = anchor.value().attr("href")
            && let Ok(target) = Url::parse(CHAPTER).and_then(|base| base.join(href))
            && target.host_str() == Some("www.dndbeyond.com")
            && target.path().starts_with("/marketplace")
        {
            purchase_links += 1;
        }
    }
    Ok(BTreeMap::from([
        ("canonical_match", 1),
        ("article_containers", 1),
        ("h1", levels[0]),
        ("h2", levels[1]),
        ("h3", levels[2]),
        ("h4", levels[3]),
        ("h5", levels[4]),
        ("h6", levels[5]),
        ("headings_with_ids", heading_ids),
        ("unique_ids", ids.len()),
        ("duplicate_ids", duplicate_ids),
        ("paragraphs", article.select(&selector("p")?).count()),
        ("list_items", article.select(&selector("li")?).count()),
        ("tables", article.select(&selector("table")?).count()),
        (
            "excluded_elements",
            article
                .select(&selector("script, style, nav, aside, form")?)
                .count(),
        ),
        ("article_marketplace_links", purchase_links),
        ("article_text_bytes", article.text().map(str::len).sum()),
        ("expected_fragments", fragments.len()),
        (
            "matched_fragments",
            fragments
                .iter()
                .filter(|id| ids.contains(id.as_str()))
                .count(),
        ),
    ]))
}

/// Formats only controlled labels and numeric counts, never response-derived strings.
fn summary(role: &'static str, counts: &Counts) -> String {
    let counts = counts
        .iter()
        .map(|(name, count)| format!("{name}={count}"))
        .collect::<Vec<_>>()
        .join("; ");
    format!("role={role}; {counts}; content_access=unverified")
}

/// Enforces the source, chapter, anonymous-control order before sending each request.
async fn page(
    client: &Client,
    cookie: Option<&BrowserSessionCookie>,
    url: &str,
    loopback: bool,
    attempts: &mut usize,
) -> Result<Page, ProbeError> {
    if *attempts >= 3 {
        return Err(ProbeError::RequestLimit);
    }
    let control = *attempts == 2;
    let expected = if *attempts == 0 { SOURCE } else { CHAPTER };
    check_target(url, expected, loopback)?;
    if cookie.is_some() == control {
        return Err(ProbeError::Target);
    }
    let mut request = client
        .get(url)
        .header(ACCEPT, "text/html")
        .header(ACCEPT_ENCODING, "identity");
    if let Some(cookie) = cookie {
        let value = std::str::from_utf8(&cookie.0).map_err(|_| ProbeError::Shape)?;
        let headers =
            SessionHeaders::cookie("CobaltSession", value).map_err(ProbeError::Headers)?;
        request = headers.apply(request);
    }
    *attempts += 1;
    let response = request.send().await.map_err(ProbeError::Transport)?;
    let status = response.status().as_u16();
    if control && matches!(status, 301 | 302 | 303 | 307 | 308 | 401 | 403) {
        return Ok(Page::Restricted(status));
    }
    bounded_body(response, "text/html").await.map(Page::Html)
}

/// Prevents another request or a completion claim after synchronous parsing reaches the deadline.
fn check_deadline(deadline: Instant) -> Result<(), ProbeError> {
    if Instant::now() >= deadline {
        Err(ProbeError::TimedOut)
    } else {
        Ok(())
    }
}

/// Reads one protected entry, processes two selected pages, then runs the final anonymous control.
async fn live_check() -> Result<(), ProbeError> {
    let deadline = Instant::now() + Duration::from_secs(30);
    let cookie = Session::default()
        .load_cookie(deadline)
        .await
        .map_err(ProbeError::Session)?
        .ok_or_else(|| ProbeError::Session(SessionError::new(SessionErrorKind::Absent)))?;
    let authenticated = client()?;
    let anonymous = client()?;
    let mut attempts = 0;
    let Page::Html(source) =
        page(&authenticated, Some(&cookie), SOURCE, false, &mut attempts).await?
    else {
        return Err(ProbeError::Shape);
    };
    eprintln!(
        "role=source; status=200; media_type=text/html; response_bytes={}",
        source.len()
    );
    let (counts, fragments) = source_structure(&source)?;
    eprintln!("{}", summary("source", &counts));
    drop(source);
    check_deadline(deadline)?;
    let Page::Html(chapter) =
        page(&authenticated, Some(&cookie), CHAPTER, false, &mut attempts).await?
    else {
        return Err(ProbeError::Shape);
    };
    eprintln!(
        "role=chapter; status=200; media_type=text/html; response_bytes={}",
        chapter.len()
    );
    let counts = chapter_structure(&chapter, &fragments)?;
    eprintln!("{}", summary("chapter", &counts));
    drop(chapter);
    drop(cookie);
    drop(authenticated);
    check_deadline(deadline)?;
    match page(&anonymous, None, CHAPTER, false, &mut attempts).await? {
        Page::Restricted(status) => eprintln!(
            "role=anonymous_control; status={status}; restriction_reason=unverified; content_access=unverified"
        ),
        Page::Html(bytes) => {
            eprintln!(
                "role=anonymous_control; status=200; media_type=text/html; response_bytes={}",
                bytes.len()
            );
            eprintln!(
                "{}",
                summary("anonymous_control", &chapter_structure(&bytes, &fragments)?)
            );
        }
    }
    check_deadline(deadline)?;
    eprintln!("structure_probe=completed; attempts={attempts}; stored_session=unchanged");
    Ok(())
}

/// Keeps the three-request scope ignored and separate from every earlier acknowledgment.
#[tokio::test]
#[ignore = "requires separate issue #33 approval for two selected pages and a final anonymous control"]
async fn live_protected_structure() -> Result<(), ProbeError> {
    let approval = std::env::var("BEYOND_GATE_BOOK_PROBE_APPROVAL").ok();
    check_approval(
        approval.as_deref(),
        APPROVAL,
        cfg!(feature = "e2e-isolation"),
    )?;
    timeout(Duration::from_secs(30), live_check())
        .await
        .map_err(|_| ProbeError::TimedOut)?
}

/// Produces original synthetic HTML with only the fixed public canonical path.
fn fixture(url: &str, body: &str) -> Vec<u8> {
    format!("<!doctype html><html><head><link rel=\"canonical\" href=\"{url}\"></head><body>{body}</body></html>").into_bytes()
}

/// Counts every link role and matches private fragment IDs without printing them.
#[test]
fn structures_are_counted_and_redacted() {
    let source = fixture(
        SOURCE,
        &format!(
            "<div class=\"compendium-toc-full-text\"><a href=\"{CHAPTER}\">Synthetic chapter</a><a href=\"/sources/cos/into-the-mists#private-one\">One</a><a href=\"{CHAPTER}#private-two\">Two</a><a href=\"{CHAPTER}#private-two\">Two</a><a href=\"https://foreign.invalid/private-value\">Outside</a><a href=\"javascript:private-value\">Unsafe</a><a>Missing</a></div>"
        ),
    );
    let (counts, fragments) = source_structure(&source).expect("invented outline");
    assert_eq!(counts["raw_links"], 7);
    assert_eq!(counts["chapter_links"], 1);
    assert_eq!(counts["fragment_links"], 3);
    assert_eq!(counts["other_links"], 1);
    assert_eq!(counts["unsafe_links"], 2);
    assert_eq!(counts["duplicate_targets"], 1);
    assert_eq!(counts["observed_legacy_links"], 1);
    assert_eq!(fragments.len(), 2);
    let chapter = fixture(
        CHAPTER,
        "<article class=\"p-article-content\"><h1 id=\"private-one\">Synthetic</h1><h2 id=\"private-two\">Synthetic</h2><h2 id=\"private-two\">Duplicate</h2><p>private-text</p></article>",
    );
    let counts = chapter_structure(&chapter, &fragments).expect("invented article");
    assert_eq!(counts["h1"], 1);
    assert_eq!(counts["h2"], 2);
    assert_eq!(counts["duplicate_ids"], 1);
    assert_eq!(counts["matched_fragments"], 2);
    let output = summary("chapter", &counts);
    assert!(!output.contains("private"));
    assert!(!output.contains("Synthetic"));
    assert!(!output.contains("<"));
    assert!(output.contains("content_access=unverified"));
}

/// Rejects ambiguous roots, hidden articles, challenges, wrong canonical paths, and excess links.
#[test]
fn structure_guards_and_raw_bounds_are_exact() {
    for count in [MAX_RECORDS, MAX_RECORDS + 1] {
        let anchors = "<a href=\"#synthetic\">Synthetic</a>".repeat(count);
        let source = fixture(
            SOURCE,
            &format!("<div class=\"compendium-toc-full-text\">{anchors}</div>"),
        );
        assert_eq!(source_structure(&source).is_ok(), count == MAX_RECORDS);
    }
    for (url, body) in [
        (
            "https://foreign.invalid/private",
            "<article class=\"p-article-content\"></article>",
        ),
        (
            CHAPTER,
            "<div id=\"challenge-form\"></div><article class=\"p-article-content\"></article>",
        ),
        (
            CHAPTER,
            "<article class=\"p-article-content\"></article><article class=\"p-article-content\"></article>",
        ),
        (
            CHAPTER,
            "<div hidden><article class=\"p-article-content\"></article></div>",
        ),
        (
            CHAPTER,
            "<article class=\"p-article-content\" aria-hidden=\"true\"></article>",
        ),
        (CHAPTER, "<p>private missing article</p>"),
    ] {
        assert!(chapter_structure(&fixture(url, body), &BTreeSet::new()).is_err());
    }
    assert!(document(&[0xff], CHAPTER).is_err());
    assert!(document(&vec![b' '; MAX_BYTES + 1], CHAPTER).is_err());
    assert!(source_structure(&fixture(SOURCE, "<div class=\"compendium-toc-full-text\"></div><div class=\"compendium-toc-full-text\"></div>")).is_err());
}

/// Rejects earlier scopes and invalid authentication order before any transport attempt.
#[tokio::test]
async fn approval_and_order_are_explicit() {
    for approval in [
        None,
        Some(""),
        Some(mobile_probe::APPROVAL),
        Some("issue-33-website-cookie-one-attempt"),
    ] {
        assert!(check_approval(approval, APPROVAL, false).is_err());
    }
    assert!(check_approval(Some(APPROVAL), APPROVAL, true).is_err());
    assert!(check_approval(Some(APPROVAL), APPROVAL, false).is_ok());
    let client = client().expect("client");
    let cookie = BrowserSessionCookie::new(b"synthetic".to_vec()).expect("cookie");
    for (count, cookie) in [(0, None), (1, None), (2, Some(&cookie)), (3, Some(&cookie))] {
        let mut attempts = count;
        assert!(
            page(
                &client,
                cookie,
                "http://127.0.0.1:1/metadata",
                true,
                &mut attempts
            )
            .await
            .is_err()
        );
        assert_eq!(attempts, count);
    }
    for (url, expected) in [
        (CHAPTER, SOURCE),
        (SOURCE, CHAPTER),
        ("https://foreign.invalid/private", CHAPTER),
    ] {
        assert!(check_target(url, expected, false).is_err());
    }
}

/// Verifies two cookie-only GETs followed by one anonymous GET, with no fourth request.
#[tokio::test]
async fn wire_order_and_anonymous_control_are_exact() {
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("loopback");
    let url = format!(
        "http://{}/metadata",
        listener.local_addr().expect("address")
    );
    let worker = tokio::spawn(async move {
        for index in 0..3 {
            let (mut stream, _) = listener.accept().await.expect("request");
            let mut bytes = Vec::new();
            let mut buffer = [0; 1024];
            while !bytes.windows(4).any(|window| window == b"\r\n\r\n") {
                let count = stream.read(&mut buffer).await.expect("request bytes");
                assert_ne!(count, 0);
                bytes.extend_from_slice(&buffer[..count]);
                assert!(bytes.len() <= 8192);
            }
            let headers = std::str::from_utf8(&bytes)
                .expect("ASCII headers")
                .to_ascii_lowercase();
            assert!(headers.starts_with("get /metadata http/1.1\r\n"));
            assert!(headers.contains("\r\naccept: text/html\r\n"));
            assert!(headers.contains("\r\naccept-encoding: identity\r\n"));
            assert!(headers.contains("\r\nuser-agent: beyond-gate/"));
            assert!(!headers.contains("authorization:"));
            assert!(!headers.contains("referer:"));
            assert_eq!(
                headers.contains("\r\ncookie: cobaltsession=synthetic-secret\r\n"),
                index < 2
            );
            assert_eq!(
                headers.matches("\r\ncookie:").count(),
                usize::from(index < 2)
            );
            let response = if index < 2 {
                "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: 14\r\nConnection: close\r\n\r\nsynthetic-body"
            } else {
                "HTTP/1.1 307 Temporary Redirect\r\nLocation: /must-not-follow\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            };
            stream
                .write_all(response.as_bytes())
                .await
                .expect("response");
        }
        assert!(
            timeout(Duration::from_millis(100), listener.accept())
                .await
                .is_err()
        );
    });
    timeout(Duration::from_secs(5), async {
        let client = client().expect("client");
        let cookie = BrowserSessionCookie::new(b"synthetic-secret".to_vec()).expect("cookie");
        let mut attempts = 0;
        for index in 0..3 {
            let cookie = (index < 2).then_some(&cookie);
            let result = page(&client, cookie, &url, true, &mut attempts)
                .await
                .expect("ordered request");
            assert!(!format!("{result:?}").contains("synthetic-body"));
            if index == 2 {
                assert!(matches!(result, Page::Restricted(307)));
            } else {
                assert!(matches!(result, Page::Html(_)));
            }
        }
        assert_eq!(attempts, 3);
        assert!(matches!(
            page(&client, None, &url, true, &mut attempts).await,
            Err(ProbeError::RequestLimit)
        ));
        worker.await.expect("wire assertions");
    })
    .await
    .expect("bounded mock");
}

/// Rejects failed authenticated responses without retrying, following redirects, or reading excess data.
#[tokio::test]
async fn authenticated_failures_are_terminal() {
    let client = client().expect("client");
    let cookie = BrowserSessionCookie::new(b"synthetic-private-cookie".to_vec()).expect("cookie");
    for (status, headers, body) in [
        (
            307,
            "Location: /must-not-follow\r\nContent-Type: text/html\r\n",
            b"private-body".to_vec(),
        ),
        (401, "Content-Type: text/html\r\n", b"private-body".to_vec()),
        (403, "Content-Type: text/html\r\n", b"private-body".to_vec()),
        (
            429,
            "Retry-After: 0\r\nContent-Type: text/html\r\n",
            b"private-body".to_vec(),
        ),
        (503, "Content-Type: text/html\r\n", b"private-body".to_vec()),
        (200, "Content-Type: application/json\r\n", b"{}".to_vec()),
        (
            200,
            "Content-Type: text/html\r\nContent-Encoding: gzip\r\n",
            b"private-body".to_vec(),
        ),
        (
            200,
            "Content-Type: text/html\r\n",
            vec![b' '; MAX_BYTES + 1],
        ),
    ] {
        let (url, worker) = mock_response(status, headers, body).await;
        let mut attempts = 0;
        let error = page(&client, Some(&cookie), &url, true, &mut attempts)
            .await
            .expect_err("terminal response");
        assert_eq!(attempts, 1);
        assert!(!format!("{error} {error:?}").contains("private"));
        worker.await.expect("no retry");
    }
}
