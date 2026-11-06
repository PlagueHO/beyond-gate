//! Separately approved cookie-only website inspection, never a production fallback.
//!
//! Literal markers are diagnostic observations, not DOM parsing, text extraction,
//! preview detection, ownership evidence, or fresh content authorization.

use super::*;

/// Separates this single website request from the consumed mobile metadata scope.
const APPROVAL: &str = "issue-33-website-cookie-one-attempt";
/// Selects only the previously observed canonical introduction, without its fragment.
const URL: &str = "https://www.dndbeyond.com/sources/dnd/cos/introduction";

/// Reports literal presence without interpreting arbitrary HTML or returning its text.
fn contains(bytes: &[u8], marker: &[u8]) -> bool {
    bytes.windows(marker.len()).any(|window| window == marker)
}

/// Rejects missing structure and known challenges, then emits fixed observations only.
fn inspect(bytes: &[u8]) -> Result<String, ProbeError> {
    if bytes.len() > MAX_BYTES {
        return Err(ProbeError::BodyLimit);
    }
    let article = contains(bytes, b"p-article-content");
    let section = contains(bytes, b"RunningtheAdventure");
    let challenge = [
        b"/cdn-cgi/challenge-platform/".as_slice(),
        b"id=\"challenge-form\"",
        b"id='challenge-form'",
    ]
    .iter()
    .any(|marker| contains(bytes, marker));
    if !article || !section || challenge {
        return Err(ProbeError::Shape);
    }
    Ok(format!(
        "status=200; media_type=text/html; response_bytes={}; article_literal={article}; section_literal={section}; canonical_url_literal={}; content_access=unverified",
        bytes.len(),
        contains(bytes, URL.as_bytes())
    ))
}

/// Sends one cookie-only GET to the fixed route; errors never select another transport.
async fn chapter(
    client: &Client,
    cookie: &BrowserSessionCookie,
    url: &str,
    loopback: bool,
    attempts: &mut usize,
) -> Result<Zeroizing<Vec<u8>>, ProbeError> {
    check_target(url, URL, loopback)?;
    if *attempts != 0 {
        return Err(ProbeError::RequestLimit);
    }
    let value = std::str::from_utf8(&cookie.0).map_err(|_| ProbeError::Shape)?;
    let headers = SessionHeaders::cookie("CobaltSession", value).map_err(ProbeError::Headers)?;
    *attempts += 1;
    let request = headers.apply(
        client
            .get(url)
            .header(ACCEPT, "text/html")
            .header(ACCEPT_ENCODING, "identity"),
    );
    let response = request.send().await.map_err(ProbeError::Transport)?;
    bounded_body(response, "text/html").await
}

/// Reads the fixed entry once, inspects one response in memory, and changes no session state.
async fn live_check() -> Result<(), ProbeError> {
    let session = Session::default();
    let deadline = Instant::now() + Duration::from_secs(30);
    let cookie = session
        .load_cookie(deadline)
        .await
        .map_err(ProbeError::Session)?
        .ok_or_else(|| ProbeError::Session(SessionError::new(SessionErrorKind::Absent)))?;
    let mut attempts = 0;
    let bytes = chapter(&client()?, &cookie, URL, false, &mut attempts).await?;
    drop(cookie);
    eprintln!("{}", inspect(&bytes)?);
    eprintln!("website_probe=completed; attempts={attempts}; stored_session=unchanged");
    Ok(())
}

/// Requires the separately approved cookie-only scope before any store or network access.
#[tokio::test]
#[ignore = "requires separate issue #33 approval for one protected read and one cookie-only GET"]
async fn live_website_cookie() -> Result<(), ProbeError> {
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

/// Rejects earlier acknowledgments and every live route except the fixed canonical path.
#[test]
fn approval_and_targets_are_separate() {
    for approval in [
        None,
        Some(""),
        Some(super::APPROVAL),
        Some(CONTRACT_APPROVAL),
        Some(mobile_probe::APPROVAL),
    ] {
        assert!(check_approval(approval, APPROVAL, false).is_err());
    }
    assert!(check_approval(Some(APPROVAL), APPROVAL, true).is_err());
    assert!(check_approval(Some(APPROVAL), APPROVAL, false).is_ok());
    assert!(check_target(URL, URL, false).is_ok());
    assert!(check_target("http://127.0.0.1:1234/metadata", URL, true).is_ok());
    for url in [
        "http://127.0.0.1:1234/metadata",
        "https://www.dndbeyond.com/sources/dnd/cos",
        "https://www.dndbeyond.com/sources/cos/introduction",
        "https://www.dndbeyond.com/sources/dnd/cos/introduction#RunningtheAdventure",
        "https://www.dndbeyond.com/sources/dnd/cos/introduction?synthetic=1",
        "https://www.dndbeyond.com/mobile/api/v6/available-user-content",
        "https://foreign.invalid/sources/dnd/cos/introduction",
    ] {
        assert!(check_target(url, URL, false).is_err());
    }
}

/// Never prints synthetic text, identifiers, HTML attributes, or unknown challenge data.
#[test]
fn observations_are_not_content_or_access() {
    let bytes = b"<article class=\"p-article-content\" data-private=\"private-id\"><h2 id=\"RunningtheAdventure\">private-text</h2></article>";
    let summary = inspect(bytes).expect("invented structure");
    assert!(summary.contains("status=200; media_type=text/html"));
    assert!(summary.contains("article_literal=true; section_literal=true"));
    assert!(summary.contains("content_access=unverified"));
    assert!(!summary.contains("private"));
    assert!(!summary.contains("<"));
    for bytes in [
        b"<article class=\"p-article-content\">private-warning</article>".as_slice(),
        b"<h2 id=\"RunningtheAdventure\">private-heading</h2>",
        b"p-article-content RunningtheAdventure /cdn-cgi/challenge-platform/private",
        b"p-article-content RunningtheAdventure <form id=\"challenge-form\">",
        b"p-article-content RunningtheAdventure <form id='challenge-form'>",
    ] {
        let error = inspect(bytes).expect_err("missing structure or challenge");
        assert!(!format!("{error} {error:?}").contains("private"));
    }
}

/// Verifies the actual GET has one cookie, no bearer, no body, and no browser impersonation.
#[tokio::test]
async fn wire_request_is_cookie_only() {
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
        let (mut stream, _) = listener.accept().await.expect("one request");
        let mut bytes = Vec::new();
        let mut buffer = [0; 1024];
        while !bytes.windows(4).any(|window| window == b"\r\n\r\n") {
            let count = stream.read(&mut buffer).await.expect("request");
            assert_ne!(count, 0);
            bytes.extend_from_slice(&buffer[..count]);
            assert!(bytes.len() <= 8192);
        }
        let headers = std::str::from_utf8(&bytes)
            .expect("ASCII request")
            .to_ascii_lowercase();
        assert!(headers.starts_with("get /metadata http/1.1\r\n"));
        assert!(headers.ends_with("\r\n\r\n"));
        assert!(headers.contains("\r\ncookie: cobaltsession=synthetic-secret\r\n"));
        assert_eq!(headers.matches("\r\ncookie:").count(), 1);
        assert!(headers.contains("\r\naccept: text/html\r\n"));
        assert!(headers.contains("\r\naccept-encoding: identity\r\n"));
        assert!(headers.contains("\r\nuser-agent: beyond-gate/"));
        assert!(!headers.contains("authorization:"));
        assert!(!headers.contains("referer:"));
        assert!(!headers.contains("content-length:"));
        let body = b"<article class=\"p-article-content\"><h2 id=\"RunningtheAdventure\">synthetic</h2></article>";
        let head = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        stream.write_all(head.as_bytes()).await.expect("headers");
        stream.write_all(body).await.expect("synthetic response");
        drop(stream);
        assert!(
            timeout(Duration::from_millis(100), listener.accept())
                .await
                .is_err()
        );
    });
    timeout(Duration::from_secs(5), async {
        let cookie = BrowserSessionCookie::new(b"synthetic-secret".to_vec()).expect("cookie");
        let mut attempts = 0;
        let bytes = chapter(
            &client().expect("client"),
            &cookie,
            &url,
            true,
            &mut attempts,
        )
        .await
        .expect("synthetic GET");
        assert!(inspect(&bytes).is_ok());
        assert_eq!(attempts, 1);
        worker.await.expect("wire assertions");
    })
    .await
    .expect("bounded mock");
}

/// Stops on redirects, denial, throttling, encoded bodies, and unexpected media types.
#[tokio::test]
async fn failures_never_retry_or_change_transport() {
    let client = client().expect("client");
    let cookie = BrowserSessionCookie::new(b"synthetic-private-cookie".to_vec()).expect("cookie");
    for (status, headers) in [
        (
            307,
            "Location: /must-not-follow\r\nContent-Type: text/html\r\n",
        ),
        (401, "Content-Type: text/html\r\n"),
        (403, "Content-Type: text/html\r\n"),
        (429, "Retry-After: 0\r\nContent-Type: text/html\r\n"),
        (503, "Content-Type: text/html\r\n"),
        (200, "Content-Type: application/json\r\n"),
        (200, "Content-Type: text/html\r\nContent-Encoding: gzip\r\n"),
        (
            200,
            "Content-Type: text/html\r\nContent-Type: application/json\r\n",
        ),
    ] {
        let (url, worker) = mock_response(status, headers, b"private-body".to_vec()).await;
        let mut attempts = 0;
        let error = chapter(&client, &cookie, &url, true, &mut attempts)
            .await
            .expect_err("terminal response");
        assert_eq!(attempts, 1);
        let diagnostic = format!("{error} {error:?}");
        assert!(!diagnostic.contains("private"));
        assert!(!diagnostic.contains(&url));
        assert!(matches!(
            chapter(&client, &cookie, &url, true, &mut attempts).await,
            Err(ProbeError::RequestLimit)
        ));
        worker.await.expect("no retry");
    }
}

/// Accepts the exact response cap and rejects excess bytes before inspecting markers.
#[tokio::test]
async fn response_and_target_bounds_are_exact() {
    let client = client().expect("client");
    let cookie = BrowserSessionCookie::new(b"synthetic-cookie".to_vec()).expect("cookie");
    for size in [MAX_BYTES, MAX_BYTES + 1] {
        let mut bytes = b"p-article-content RunningtheAdventure".to_vec();
        bytes.resize(size, b' ');
        let (url, worker) = mock_response(200, "Content-Type: text/html\r\n", bytes).await;
        let mut attempts = 0;
        assert!(matches!(
            chapter(&client, &cookie, &url, false, &mut attempts).await,
            Err(ProbeError::Target)
        ));
        assert_eq!(attempts, 0);
        let result = chapter(&client, &cookie, &url, true, &mut attempts).await;
        if size == MAX_BYTES {
            let bytes = result.expect("exact cap");
            assert_eq!(bytes.len(), MAX_BYTES);
            assert!(inspect(&bytes).is_ok());
        } else {
            assert!(matches!(result, Err(ProbeError::BodyLimit)));
        }
        assert_eq!(attempts, 1);
        worker.await.expect("bounded response");
    }
}
