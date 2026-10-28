//! Synthetic loopback contracts for the HTTP boundary; no account or live host.

use super::*;
use std::{
    error::Error,
    sync::{
        Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    task::{JoinHandle, JoinSet},
    time::timeout,
};

/// Contains only synthetic data and a controllable response schedule.
#[derive(Clone)]
struct Reply {
    status: u16,
    body: Vec<u8>,
    headers: String,
    chunked: bool,
    close_delimited: bool,
    header_delay: Duration,
    body_delay: Duration,
}

impl Reply {
    /// Creates a length-delimited synthetic JSON response.
    fn new(status: u16, body: &[u8]) -> Self {
        Self {
            status,
            body: body.to_vec(),
            headers: String::new(),
            chunked: false,
            close_delimited: false,
            header_delay: Duration::ZERO,
            body_delay: Duration::ZERO,
        }
    }

    /// Adds controlled response headers to a synthetic reply.
    fn headers(mut self, headers: &str) -> Self {
        self.headers = headers.to_owned();
        self
    }
}

/// Records actual wire requests and their arrival times.
#[derive(Default)]
struct Observations {
    requests: Mutex<Vec<(String, Instant)>>,
    active: AtomicUsize,
    peak: AtomicUsize,
}

/// Owns a bounded local server whose workers stop when the test ends.
struct Mock {
    origin: String,
    observations: Arc<Observations>,
    worker: JoinHandle<()>,
}

impl Mock {
    /// Binds a loopback server and repeats the last reply after the given sequence.
    async fn start(replies: Vec<Reply>) -> Self {
        assert!(!replies.is_empty());
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind loopback");
        let origin = format!("http://{}", listener.local_addr().expect("local address"));
        let observations = Arc::new(Observations::default());
        let recorded = observations.clone();
        let worker = tokio::spawn(async move {
            let mut workers = JoinSet::new();
            let mut index = 0;
            loop {
                tokio::select! {
                    accepted = listener.accept() => {
                        let (stream, _) = accepted.expect("accept synthetic request");
                        let reply = replies[index.min(replies.len() - 1)].clone();
                        index += 1;
                        let observations = recorded.clone();
                        workers.spawn(async move {
                            let result = timeout(
                                Duration::from_secs(5),
                                respond(stream, reply, observations),
                            ).await.expect("mock handler deadline");
                            if let Err(error) = result {
                                assert!(matches!(
                                    error.kind(),
                                    std::io::ErrorKind::ConnectionReset
                                        | std::io::ErrorKind::ConnectionAborted
                                        | std::io::ErrorKind::BrokenPipe
                                        | std::io::ErrorKind::NotConnected
                                        | std::io::ErrorKind::UnexpectedEof
                                ), "unexpected mock socket failure: {error}");
                            }
                        });
                    }
                    Some(result) = workers.join_next(), if !workers.is_empty() => {
                        result.expect("synthetic response worker");
                    }
                }
            }
        });
        Self {
            origin,
            observations,
            worker,
        }
    }

    /// Builds the same client with a private loopback exception and smaller caps.
    fn client(&self, policy: RequestPolicy) -> HttpClient {
        HttpClient::build(&[&self.origin], policy, true).expect("mock client")
    }

    /// Returns the number of complete request headers received by this server.
    fn count(&self) -> usize {
        self.observations
            .requests
            .lock()
            .expect("request lock")
            .len()
    }

    /// Waits for real requests instead of assuming tasks have started.
    async fn wait_for(&self, count: usize) {
        timeout(Duration::from_secs(2), async {
            while self.count() < count {
                sleep(Duration::from_millis(2)).await;
            }
        })
        .await
        .expect("expected mock requests");
    }
}

impl Drop for Mock {
    /// Cancels the accept loop; its JoinSet also cancels all response workers.
    fn drop(&mut self) {
        self.worker.abort();
    }
}

/// Reads bounded headers and sends a synthetic response with optional stalls.
///
/// Socket errors after client cancellation are expected and returned to the
/// worker. They are not confused with successful client reads.
async fn respond(
    mut stream: TcpStream,
    reply: Reply,
    observations: Arc<Observations>,
) -> std::io::Result<()> {
    let mut request = Vec::new();
    while !request.ends_with(b"\r\n\r\n") {
        let byte = stream.read_u8().await?;
        request.push(byte);
        assert!(request.len() < 32 * 1024, "mock request header bound");
    }
    observations.requests.lock().expect("request lock").push((
        String::from_utf8(request).expect("synthetic ASCII headers"),
        Instant::now(),
    ));
    let active = observations.active.fetch_add(1, Ordering::SeqCst) + 1;
    observations.peak.fetch_max(active, Ordering::SeqCst);
    let result = write_reply(&mut stream, reply).await;
    observations.active.fetch_sub(1, Ordering::SeqCst);
    result
}

/// Writes a reply without making the mock dependent on a production HTTP server.
async fn write_reply(stream: &mut TcpStream, reply: Reply) -> std::io::Result<()> {
    sleep(reply.header_delay).await;
    let framing = if reply.chunked {
        "Transfer-Encoding: chunked\r\n".to_owned()
    } else if reply.close_delimited {
        String::new()
    } else {
        format!("Content-Length: {}\r\n", reply.body.len())
    };
    stream
        .write_all(
            format!(
                "HTTP/1.1 {} Mock\r\n{framing}{}Connection: close\r\n\r\n",
                reply.status, reply.headers,
            )
            .as_bytes(),
        )
        .await?;
    sleep(reply.body_delay).await;
    if reply.chunked {
        for chunk in reply.body.chunks(7) {
            stream
                .write_all(format!("{:x}\r\n", chunk.len()).as_bytes())
                .await?;
            stream.write_all(chunk).await?;
            stream.write_all(b"\r\n").await?;
        }
        stream.write_all(b"0\r\n\r\n").await?;
    } else {
        stream.write_all(&reply.body).await?;
    }
    stream.shutdown().await
}

/// Supplies a small synthetic typed response with no upstream schema claim.
#[derive(Debug, serde::Deserialize, PartialEq)]
struct Item {
    value: u32,
}

/// Uses smaller body limits while preserving the agreed retry and time caps.
fn policy() -> RequestPolicy {
    RequestPolicy {
        bytes: 128,
        ..RequestPolicy::default()
    }
}

/// Checks typed output, ignored unknown fields, and sensitive headers on the wire.
#[tokio::test]
async fn reads_typed_json_with_scoped_session_headers() {
    let mock = Mock::start(vec![Reply::new(200, br#"{"value":7,"extra":true}"#)]).await;
    let client = mock.client(policy());
    for session in [
        SessionHeaders::bearer("synthetic-private-token").expect("token"),
        SessionHeaders::cookie("Synthetic", "synthetic-private-cookie").expect("cookie"),
    ] {
        let item: Item = client
            .get_json(&format!("{}/item", mock.origin), Some(&session))
            .await
            .expect("typed read");
        assert_eq!(item, Item { value: 7 });
        assert!(!format!("{session:?}").contains("synthetic-private"));
    }
    let requests = mock.observations.requests.lock().expect("request lock");
    assert_eq!(requests.len(), 2);
    let bearer = requests[0].0.to_ascii_lowercase();
    assert!(bearer.starts_with("get /item http/1.1\r\n"));
    assert!(bearer.contains("authorization: bearer synthetic-private-token\r\n"));
    assert!(!bearer.contains("\r\ncookie:"));
    assert!(bearer.contains("accept-encoding: identity\r\n"));
    assert!(!bearer.contains("\r\nreferer:"));
    let cookie = requests[1].0.to_ascii_lowercase();
    assert!(cookie.contains("cookie: synthetic=synthetic-private-cookie\r\n"));
    assert!(!cookie.contains("\r\nauthorization:"));
}

/// Rejects header injection, empty secrets, and secrets above the input bound.
#[test]
fn session_input_is_validated_and_sensitive() {
    for value in ["", "two values", "a\r\nx:y", "a;b=c", "\u{80}"] {
        assert!(SessionHeaders::bearer(value).is_err());
        assert!(SessionHeaders::cookie("Synthetic", value).is_err());
    }
    for name in ["", "a=b", "a;b", "a\r\n", "a b"] {
        assert!(SessionHeaders::cookie(name, "value").is_err());
    }
    assert!(SessionHeaders::bearer(&"x".repeat(8192)).is_ok());
    assert!(SessionHeaders::bearer(&"x".repeat(8193)).is_err());
    assert!(SessionHeaders::cookie("n", &"x".repeat(8191)).is_ok());
    assert!(SessionHeaders::cookie("n", &"x".repeat(8192)).is_err());
    let token = SessionHeaders::bearer("synthetic-private").expect("valid token");
    assert!(token.authorization.as_ref().expect("header").is_sensitive());
    assert!(!format!("{:?}", token.authorization).contains("synthetic-private"));
    let cookie = SessionHeaders::cookie("name", "synthetic-private").expect("valid cookie");
    assert!(cookie.cookie.as_ref().expect("header").is_sensitive());
    assert!(!format!("{:?}", cookie.cookie).contains("synthetic-private"));
}

/// Validates exact origins without contacting the named synthetic hosts.
#[tokio::test]
async fn rejects_unapproved_and_unsafe_destinations_before_sending() {
    let client = HttpClient::new(&["https://allowed.invalid"]).expect("valid origin");
    for url in [
        "https://allowed.invalid.evil.invalid/item",
        "https://other.invalid/item",
        "https://allowed.invalid:444/item",
    ] {
        let error = client
            .get_json::<Item>(url, None)
            .await
            .expect_err("unapproved");
        assert_eq!(error.kind(), ErrorKind::UnapprovedOrigin);
        assert_eq!(error.attempts(), 0);
    }
    for url in [
        "http://allowed.invalid/item",
        "ftp://allowed.invalid/item",
        "https://synthetic-private@allowed.invalid/item",
        "https://user:synthetic-private@allowed.invalid/item",
        "https://allowed.invalid/item#synthetic-private",
        "invalid synthetic-private URL",
    ] {
        let error = client
            .get_json::<Item>(url, None)
            .await
            .expect_err("invalid");
        assert_eq!(error.kind(), ErrorKind::InvalidInput);
        assert_safe_error(&error);
    }
    let empty = HttpClient::new(&[]).expect("empty denies all");
    assert_eq!(
        empty
            .get_json::<Item>("https://allowed.invalid", None)
            .await
            .expect_err("no origins")
            .kind(),
        ErrorKind::UnapprovedOrigin
    );
    for origin in [
        "http://127.0.0.1:1",
        "https://allowed.invalid/path",
        "https://allowed.invalid?x=1",
        "https://allowed.invalid/#fragment",
        "https://user:synthetic-private@allowed.invalid",
    ] {
        assert_eq!(
            HttpClient::new(&[origin])
                .expect_err("invalid origin")
                .kind(),
            ErrorKind::InvalidInput
        );
    }
    let normalized = HttpClient::new(&["https://ALLOWED.invalid:443/"]).expect("normalized");
    assert_eq!(&*normalized.origins, &["https://allowed.invalid"]);
}

/// Proves an unapproved local target sees neither credentials nor a request.
#[tokio::test]
async fn unapproved_mock_receives_no_request() {
    let approved = Mock::start(vec![Reply::new(200, b"{}")]).await;
    let denied = Mock::start(vec![Reply::new(200, b"{}")]).await;
    let session = SessionHeaders::bearer("synthetic-private").expect("token");
    let error = approved
        .client(policy())
        .get_json::<Item>(&denied.origin, Some(&session))
        .await
        .expect_err("different port");
    assert_eq!(error.kind(), ErrorKind::UnapprovedOrigin);
    assert_eq!(denied.count(), 0);
    assert_eq!(approved.count(), 0);
}

/// Rejects every redirect without forwarding a bearer or cookie to its target.
#[tokio::test]
async fn redirects_are_never_followed() {
    let target = Mock::start(vec![Reply::new(200, br#"{"value":1}"#)]).await;
    for status in [301, 302, 303, 307, 308] {
        let source = Mock::start(vec![
            Reply::new(status, b"synthetic-private-body")
                .headers(&format!("Location: {}/stolen\r\n", target.origin)),
        ])
        .await;
        let client = HttpClient::build(&[&source.origin, &target.origin], policy(), true)
            .expect("two allowed origins");
        for session in [
            SessionHeaders::bearer("synthetic-private").expect("token"),
            SessionHeaders::cookie("Synthetic", "synthetic-private").expect("cookie"),
        ] {
            let error = client
                .get_json::<Item>(&source.origin, Some(&session))
                .await
                .expect_err("redirect");
            assert_eq!(error.kind(), ErrorKind::Redirect);
            assert_eq!(error.attempts(), 1);
        }
        assert_eq!(source.count(), 2);
        assert_eq!(target.count(), 0);
    }
}

/// Distinguishes terminal statuses and never retries denied or rejected sessions.
#[tokio::test]
async fn status_errors_are_typed_and_terminal_when_required() {
    for (status, kind) in [
        (401, ErrorKind::SessionUnusable),
        (403, ErrorKind::AccessDenied),
        (404, ErrorKind::UpstreamStatus),
        (500, ErrorKind::UpstreamStatus),
    ] {
        let mut reply = Reply::new(status, b"synthetic-private-body");
        reply.body_delay = Duration::from_secs(2);
        let mock = Mock::start(vec![reply]).await;
        let error = mock
            .client(policy())
            .get_json::<Item>(&mock.origin, None)
            .await
            .expect_err("status error");
        assert_eq!(error.kind(), kind);
        assert_eq!(error.status(), Some(status));
        assert_eq!(error.attempts(), 1);
        assert_eq!(mock.count(), 1);
        assert_safe_error(&error);
    }
}

/// Preserves a denied character page as a typed inaccessible result.
#[tokio::test]
async fn character_assembly_preserves_access_denied_page_errors() {
    let mock = Mock::start(vec![Reply::new(403, b"synthetic-private-body")]).await;
    let error = mock
        .client(policy())
        .get_json::<Item>(&mock.origin, None)
        .await
        .expect_err("denied synthetic response");
    let failure = crate::character_campaign::assemble_characters([Err(error)])
        .expect_err("no snapshot from an inaccessible page");
    assert!(matches!(
        failure,
        crate::character_campaign::CharacterCampaignError::PageRead(_)
    ));
    let http_error = failure
        .source()
        .and_then(|source| source.downcast_ref::<HttpError>())
        .expect("typed HTTP source");
    assert_eq!(http_error.kind(), ErrorKind::AccessDenied);
    assert_eq!(http_error.status(), Some(403));
    assert!(!format!("{failure} {failure:?}").contains("synthetic-private-body"));
}

/// Checks byte thresholds for length-delimited, chunked, and close-delimited data.
#[tokio::test]
async fn body_limits_are_exact_for_all_framing_modes() {
    for mode in 0..3 {
        for length in [128, 129] {
            let mut body = br#"{"value":7}"#.to_vec();
            body.resize(length, b' ');
            let mut reply = Reply::new(200, &body);
            reply.chunked = mode == 1;
            reply.close_delimited = mode == 2;
            let mock = Mock::start(vec![reply]).await;
            let result = mock
                .client(policy())
                .get_json::<Item>(&mock.origin, None)
                .await;
            if length == 128 {
                assert_eq!(result.expect("at limit"), Item { value: 7 });
            } else {
                let error = result.expect_err("over limit");
                assert_eq!(error.kind(), ErrorKind::BodyTooLarge);
                assert_eq!(error.attempts(), 1);
            }
            assert_eq!(mock.count(), 1);
        }
    }
}

/// Rejects a known excess size before waiting for a stalled response body.
#[tokio::test]
async fn oversized_content_length_stops_before_body() {
    let mut reply = Reply::new(200, &[b' '; 129]);
    reply.body_delay = Duration::from_secs(2);
    let mock = Mock::start(vec![reply]).await;
    let error = timeout(
        Duration::from_secs(1),
        mock.client(policy()).get_json::<Item>(&mock.origin, None),
    )
    .await
    .expect("reject before body")
    .expect_err("oversized");
    assert_eq!(error.kind(), ErrorKind::BodyTooLarge);
}

/// Checks invalid JSON, type failures, and explicit encoding rejection.
#[tokio::test]
async fn malformed_responses_are_not_retried_or_disclosed() {
    for reply in [
        Reply::new(200, b"synthetic-private-body"),
        Reply::new(200, br#"{"value":"synthetic-private-value"}"#),
        Reply::new(200, br#"{"value":7"#),
        Reply::new(200, br#"{"value":7}"#).headers("Content-Encoding: gzip\r\n"),
        Reply::new(204, b""),
    ] {
        let mock = Mock::start(vec![reply]).await;
        let error = mock
            .client(policy())
            .get_json::<Item>(&mock.origin, None)
            .await
            .expect_err("malformed");
        assert_eq!(error.kind(), ErrorKind::MalformedResponse);
        assert_eq!(error.attempts(), 1);
        assert_eq!(mock.count(), 1);
        if let Some(cause) = error.cause() {
            assert!(matches!(cause.kind(), CauseKind::Json(_)));
            assert!(cause.json_position().is_some());
        }
        assert_safe_error(&error);
    }
}

/// Accepts identity content coding regardless of HTTP token capitalization.
#[tokio::test]
async fn identity_content_encoding_is_case_insensitive() {
    let mock = Mock::start(vec![
        Reply::new(200, br#"{"value":7}"#).headers("Content-Encoding: IdEnTiTy\r\n"),
    ])
    .await;
    let item: Item = mock
        .client(policy())
        .get_json(&mock.origin, None)
        .await
        .expect("identity encoding");
    assert_eq!(item.value, 7);
    assert_eq!(mock.count(), 1);
}

/// Verifies exactly three attempts, bounded exponential delay, and final status.
#[tokio::test]
async fn retry_budget_is_exact_and_preserves_final_failure() {
    for status in [429, 502, 503, 504] {
        let mock = Mock::start(vec![Reply::new(status, b"synthetic-private")]).await;
        let error = mock
            .client(policy())
            .get_json::<Item>(&mock.origin, None)
            .await
            .expect_err("exhausted");
        assert_eq!(error.status(), Some(status));
        assert_eq!(error.attempts(), 3);
        assert_eq!(mock.count(), 3);
        assert_eq!(
            error.kind(),
            if status == 429 {
                ErrorKind::RateLimited
            } else {
                ErrorKind::UpstreamStatus
            }
        );
        let requests = mock.observations.requests.lock().expect("request lock");
        assert!(requests[1].1.duration_since(requests[0].1) >= Duration::from_millis(250));
        assert!(requests[2].1.duration_since(requests[1].1) >= Duration::from_millis(500));
    }
}

/// Accepts a later successful response without retrying again or retaining cookies.
#[tokio::test]
async fn transient_status_can_recover_without_a_cookie_jar() {
    let mock = Mock::start(vec![
        Reply::new(503, b"")
            .headers("Retry-After: invalid\r\nSet-Cookie: Synthetic=synthetic-private\r\n"),
        Reply::new(200, br#"{"value":9}"#),
    ])
    .await;
    let value: Item = mock
        .client(policy())
        .get_json(&mock.origin, None)
        .await
        .expect("recovered");
    assert_eq!(value.value, 9);
    assert_eq!(mock.count(), 2);
    for (request, _) in mock
        .observations
        .requests
        .lock()
        .expect("request lock")
        .iter()
    {
        assert!(!request.to_ascii_lowercase().contains("\r\ncookie:"));
    }
}

/// Does not retry earlier than a valid server delay in seconds.
#[tokio::test]
async fn retry_after_seconds_is_a_minimum_wait() {
    let mock = Mock::start(vec![
        Reply::new(429, b"").headers("Retry-After: 1\r\n"),
        Reply::new(200, br#"{"value":3}"#),
    ])
    .await;
    mock.client(policy())
        .get_json::<Item>(&mock.origin, None)
        .await
        .expect("retried");
    let requests = mock.observations.requests.lock().expect("request lock");
    assert_eq!(requests.len(), 2);
    assert!(requests[1].1.duration_since(requests[0].1) >= Duration::from_secs(1));
}

/// Stops with the rate-limit error instead of clipping or overflowing long waits.
#[tokio::test]
async fn excessive_retry_after_does_not_send_another_request() {
    for delay in [
        "60",
        "184467440737095516160",
        "Fri, 31 Dec 9999 23:59:59 GMT",
    ] {
        let mock = Mock::start(vec![
            Reply::new(429, b"").headers(&format!("Retry-After: {delay}\r\n")),
        ])
        .await;
        let error = mock
            .client(policy())
            .get_json::<Item>(&mock.origin, None)
            .await
            .expect_err("wait exceeds budget");
        assert_eq!(error.kind(), ErrorKind::RateLimited);
        assert_eq!(error.attempts(), 1);
        assert_eq!(mock.count(), 1);
        assert!(error.retry_after().is_some());
    }
}

/// Checks both date syntax and delay parsing without depending on wall-clock timing.
#[test]
fn retry_after_parses_dates_and_invalid_values() {
    assert_eq!(retry_after(None), None);
    for value in ["invalid", "-1", "1.5", ""] {
        assert_eq!(
            retry_after(Some(&HeaderValue::from_str(value).expect("header"))),
            None
        );
    }
    assert_eq!(
        retry_after(Some(&HeaderValue::from_static("0"))),
        Some(Duration::ZERO)
    );
    assert_eq!(
        retry_after(Some(&HeaderValue::from_static(
            "Sun, 06 Nov 1994 08:49:37 GMT"
        ))),
        Some(Duration::ZERO)
    );
    let date = std::time::SystemTime::now() + Duration::from_secs(120);
    let header = HeaderValue::from_str(&httpdate::fmt_http_date(date)).expect("date header");
    let delay = retry_after(Some(&header)).expect("valid future date");
    assert!(delay > Duration::from_secs(118) && delay <= Duration::from_secs(120));
}

/// Bounds both headers and body stalls using the attempt timeout.
#[tokio::test]
async fn stalled_headers_and_bodies_time_out() {
    for stalled_body in [false, true] {
        let mut reply = Reply::new(200, br#"{"value":1}"#);
        if stalled_body {
            reply.body_delay = Duration::from_secs(2);
        } else {
            reply.header_delay = Duration::from_secs(2);
        }
        let mock = Mock::start(vec![reply]).await;
        let client = mock.client(RequestPolicy {
            attempt: Duration::from_millis(50),
            retries: 0,
            ..policy()
        });
        let error = client
            .get_json::<Item>(&mock.origin, None)
            .await
            .expect_err("timeout");
        assert_eq!(error.kind(), ErrorKind::Timeout);
        assert!(error.cause().expect("typed timeout").is_timeout());
        assert_eq!(error.attempts(), 1);
    }
}

/// Includes all attempts and backoff in the same total deadline.
#[tokio::test]
async fn total_deadline_is_not_reset_for_retries() {
    let mut stalled = Reply::new(200, br#"{"value":1}"#);
    stalled.header_delay = Duration::from_secs(2);
    let mock = Mock::start(vec![Reply::new(503, b""), stalled]).await;
    let client = mock.client(RequestPolicy {
        total: Duration::from_millis(450),
        ..policy()
    });
    let started = Instant::now();
    let error = client
        .get_json::<Item>(&mock.origin, None)
        .await
        .expect_err("total timeout");
    assert_eq!(error.kind(), ErrorKind::Timeout);
    assert_eq!(error.attempts(), 2);
    assert_eq!(mock.count(), 2);
    assert!(started.elapsed() < Duration::from_secs(1));
}

/// Checks real connection failure classification and bounded transport retries.
#[tokio::test]
async fn connection_failures_keep_a_safe_transport_cause() {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("reserve closed port");
    let origin = format!("http://{}", listener.local_addr().expect("address"));
    drop(listener);
    let client = HttpClient::build(&[&origin], policy(), true).expect("client");
    let error = client
        .get_json::<Item>(&format!("{origin}/?synthetic-private=value"), None)
        .await
        .expect_err("connection refused");
    assert_eq!(error.kind(), ErrorKind::Network);
    assert_eq!(error.attempts(), 3);
    assert_eq!(error.cause().expect("cause").kind(), CauseKind::Transport);
    assert!(error.cause().expect("cause").is_connect());
    assert!(!error.cause().expect("cause").is_dns());
    assert_safe_error(&error);
}

/// Verifies the server sees at most four concurrent requests across client clones.
#[tokio::test]
async fn clones_share_the_concurrency_limit() {
    let mut reply = Reply::new(200, br#"{"value":1}"#);
    reply.body_delay = Duration::from_millis(80);
    let mock = Mock::start(vec![reply]).await;
    let client = mock.client(policy());
    let mut tasks = JoinSet::new();
    for _ in 0..12 {
        let client = client.clone();
        let url = mock.origin.clone();
        tasks.spawn(async move { client.get_json::<Item>(&url, None).await });
    }
    while let Some(result) = tasks.join_next().await {
        result.expect("task").expect("read");
    }
    assert_eq!(mock.count(), 12);
    assert_eq!(mock.observations.peak.load(Ordering::SeqCst), 4);
    assert_eq!(client.permits.available_permits(), 4);
}

/// Includes permit waiting in the total deadline without submitting a request.
#[tokio::test]
async fn queued_read_can_time_out_without_sending() {
    let mock = Mock::start(vec![Reply::new(200, br#"{"value":1}"#)]).await;
    let client = mock.client(RequestPolicy {
        concurrency: 1,
        total: Duration::from_millis(40),
        ..policy()
    });
    let permit = client.permits.acquire().await.expect("hold all permits");
    let error = client
        .get_json::<Item>(&mock.origin, None)
        .await
        .expect_err("queue timeout");
    assert_eq!(error.kind(), ErrorKind::Timeout);
    assert_eq!(error.attempts(), 0);
    assert_eq!(
        error.cause().expect("deadline cause").kind(),
        CauseKind::Deadline
    );
    assert_eq!(mock.count(), 0);
    drop(permit);
    client
        .get_json::<Item>(&mock.origin, None)
        .await
        .expect("permit released");
}

/// Cancels in-flight reads and backoff, then proves the next read can proceed.
#[tokio::test]
async fn cancellation_releases_permits_and_stops_retries() {
    for during_retry in [false, true] {
        let mut first = Reply::new(if during_retry { 503 } else { 200 }, br#"{"value":1}"#);
        if during_retry {
            first.headers = "Retry-After: 1\r\n".to_owned();
        } else {
            first.body_delay = Duration::from_secs(2);
        }
        let mock = Mock::start(vec![first, Reply::new(200, br#"{"value":2}"#)]).await;
        let client = mock.client(RequestPolicy {
            concurrency: 1,
            ..policy()
        });
        let owned = client.clone();
        let url = mock.origin.clone();
        let task = tokio::spawn(async move { owned.get_json::<Item>(&url, None).await });
        mock.wait_for(1).await;
        sleep(Duration::from_millis(10)).await;
        task.abort();
        assert!(task.await.expect_err("cancelled task").is_cancelled());
        assert_eq!(client.permits.available_permits(), 1);
        let result: Item = client
            .get_json(&mock.origin, None)
            .await
            .expect("read after cancel");
        assert_eq!(result.value, 2);
        assert_eq!(mock.count(), 2);
    }
}

/// Does not start an attempt if a ready permit is polled after the total deadline.
#[tokio::test(start_paused = true)]
async fn expired_queue_permit_does_not_start_an_attempt() {
    let mock = Mock::start(vec![Reply::new(200, br#"{"value":1}"#)]).await;
    let client = mock.client(RequestPolicy {
        concurrency: 1,
        total: Duration::from_secs(1),
        ..policy()
    });
    let permit = client.permits.acquire().await.expect("hold all permits");
    let mut read = Box::pin(client.get_json::<Item>(&mock.origin, None));
    tokio::select! {
        biased;
        result = &mut read => panic!("read completed while permit was held: {result:?}"),
        () = std::future::ready(()) => {}
    }
    tokio::time::advance(Duration::from_secs(2)).await;
    drop(permit);
    let error = read.await.expect_err("expired queue wait");
    assert_eq!(error.kind(), ErrorKind::Timeout);
    assert_eq!(error.attempts(), 0);
    assert_eq!(mock.count(), 0);
    assert_eq!(client.permits.available_permits(), 1);
}

/// Rejects zero or widened policy values before any request can start.
#[test]
fn policy_rejects_invalid_or_wider_limits() {
    let base = policy();
    for policy in [
        RequestPolicy {
            connect: Duration::ZERO,
            ..base
        },
        RequestPolicy {
            attempt: Duration::ZERO,
            ..base
        },
        RequestPolicy {
            total: Duration::ZERO,
            ..base
        },
        RequestPolicy {
            concurrency: 0,
            ..base
        },
        RequestPolicy { bytes: 0, ..base },
        RequestPolicy {
            connect: Duration::from_secs(6),
            ..base
        },
        RequestPolicy {
            attempt: Duration::from_secs(16),
            ..base
        },
        RequestPolicy {
            total: Duration::from_secs(31),
            ..base
        },
        RequestPolicy {
            concurrency: 5,
            ..base
        },
        RequestPolicy {
            bytes: 1024 * 1024 + 1,
            ..base
        },
        RequestPolicy { retries: 3, ..base },
    ] {
        assert!(matches!(policy.validate(), Err(error) if error.kind() == ErrorKind::InvalidInput));
    }
}

/// Checks the actual production byte cap, not only a reduced test policy.
#[tokio::test]
async fn production_body_limit_is_one_mib() {
    let default = RequestPolicy::default();
    assert_eq!(default.bytes, 1024 * 1024);
    for extra in [0, 1] {
        let mut body = br#"{"value":7}"#.to_vec();
        body.resize(default.bytes + extra, b' ');
        let mock = Mock::start(vec![Reply::new(200, &body)]).await;
        let result = mock
            .client(default)
            .get_json::<Item>(&mock.origin, None)
            .await;
        if extra == 0 {
            assert_eq!(result.expect("one MiB accepted"), Item { value: 7 });
        } else {
            assert_eq!(
                result.expect_err("one byte over").kind(),
                ErrorKind::BodyTooLarge
            );
        }
    }
}

/// Honors an HTTP-date delay on the wire, not only in the date parser.
#[tokio::test]
async fn retry_after_http_date_is_a_minimum_wait() {
    let now = std::time::SystemTime::now();
    let date = httpdate::fmt_http_date(now + Duration::from_secs(2));
    let parsed = httpdate::parse_http_date(&date).expect("date");
    let earliest = Instant::now() + parsed.duration_since(now).expect("future date");
    let mock = Mock::start(vec![
        Reply::new(503, b"").headers(&format!("Retry-After: {date}\r\n")),
        Reply::new(200, br#"{"value":7}"#),
    ])
    .await;
    mock.client(policy())
        .get_json::<Item>(&mock.origin, None)
        .await
        .expect("recovered");
    let requests = mock.observations.requests.lock().expect("request lock");
    assert_eq!(requests.len(), 2);
    assert!(requests[1].1 >= earliest);
}

/// Uses the longest valid wait when an upstream response repeats Retry-After.
#[tokio::test]
async fn repeated_retry_after_cannot_shorten_the_server_wait() {
    let mock =
        Mock::start(vec![Reply::new(429, b"").headers(
            "Retry-After: invalid\r\nRetry-After: 0\r\nRetry-After: 60\r\n",
        )])
        .await;
    let error = mock
        .client(policy())
        .get_json::<Item>(&mock.origin, None)
        .await
        .expect_err("longest wait exceeds budget");
    assert_eq!(error.kind(), ErrorKind::RateLimited);
    assert_eq!(error.retry_after(), Some(Duration::from_secs(60)));
    assert_eq!(mock.count(), 1);
}

/// Confirms a transient attempt timeout can recover within the total budget.
#[tokio::test]
async fn attempt_timeout_can_retry_a_safe_read() {
    let mut stalled = Reply::new(200, br#"{"value":1}"#);
    stalled.header_delay = Duration::from_millis(150);
    let mock = Mock::start(vec![stalled, Reply::new(200, br#"{"value":2}"#)]).await;
    let client = mock.client(RequestPolicy {
        attempt: Duration::from_millis(50),
        ..policy()
    });
    let item: Item = client
        .get_json(&mock.origin, None)
        .await
        .expect("recovered");
    assert_eq!(item.value, 2);
    assert_eq!(mock.count(), 2);
}

/// Ensures a retry wait occupies a permit instead of allowing extra operations.
#[tokio::test]
async fn retry_wait_holds_the_shared_permit() {
    let mock = Mock::start(vec![
        Reply::new(503, b"").headers("Retry-After: 1\r\n"),
        Reply::new(200, br#"{"value":2}"#),
    ])
    .await;
    let client = mock.client(RequestPolicy {
        concurrency: 1,
        ..policy()
    });
    let first_client = client.clone();
    let first_url = mock.origin.clone();
    let first = tokio::spawn(async move { first_client.get_json::<Item>(&first_url, None).await });
    mock.wait_for(1).await;
    let second_client = client.clone();
    let second_url = mock.origin.clone();
    let second =
        tokio::spawn(async move { second_client.get_json::<Item>(&second_url, None).await });
    sleep(Duration::from_millis(50)).await;
    assert_eq!(mock.count(), 1);
    first.await.expect("first task").expect("first read");
    second.await.expect("second task").expect("second read");
    assert_eq!(mock.count(), 3);
    assert_eq!(client.permits.available_permits(), 1);
}

/// Cancels a queued operation without sending or consuming a permit later.
#[tokio::test]
async fn queued_cancellation_does_not_send_a_request() {
    let mock = Mock::start(vec![Reply::new(200, br#"{"value":1}"#)]).await;
    let client = mock.client(RequestPolicy {
        concurrency: 1,
        ..policy()
    });
    let permit = client.permits.acquire().await.expect("hold all permits");
    let owned = client.clone();
    let url = mock.origin.clone();
    let queued = tokio::spawn(async move { owned.get_json::<Item>(&url, None).await });
    sleep(Duration::from_millis(10)).await;
    queued.abort();
    assert!(
        queued
            .await
            .expect_err("cancelled queue task")
            .is_cancelled()
    );
    drop(permit);
    client
        .get_json::<Item>(&mock.origin, None)
        .await
        .expect("next read");
    assert_eq!(mock.count(), 1);
    assert_eq!(client.permits.available_permits(), 1);
}

/// Preserves actual HTTP failures through offline library page assembly.
#[tokio::test]
async fn library_page_failures_keep_typed_causes_without_partial_results() {
    use crate::library::{AccessEvidence, BookRecord, LibraryError, LibraryPage, assemble_library};

    for (reply, kind, status) in [
        (
            Reply::new(401, b"synthetic-private-body"),
            ErrorKind::SessionUnusable,
            Some(401),
        ),
        (
            Reply::new(403, b"synthetic-private-body"),
            ErrorKind::AccessDenied,
            Some(403),
        ),
        (
            Reply::new(429, b"synthetic-private-body").headers("Retry-After: 3\r\n"),
            ErrorKind::RateLimited,
            Some(429),
        ),
        (
            Reply::new(200, br#"{"value":"synthetic-private-value"}"#),
            ErrorKind::MalformedResponse,
            None,
        ),
    ] {
        let mock = Mock::start(vec![reply]).await;
        let client = mock.client(RequestPolicy {
            retries: 0,
            ..policy()
        });
        let failure = client
            .get_json::<Item>(&mock.origin, None)
            .await
            .expect_err("supplied failed page");
        let first = LibraryPage {
            cursor: None,
            entries: vec![BookRecord {
                id: "synthetic-private-book".to_owned(),
                title: "Synthetic private title".to_owned(),
                source_url: "https://example.com/synthetic-private-book".to_owned(),
                evidence: AccessEvidence::CatalogOnly,
            }],
            next_cursor: Some("synthetic-private-cursor".to_owned()),
        };
        let error = assemble_library([Ok(first), Err(failure)])
            .expect_err("a failed later page cannot return a snapshot");
        assert!(!format!("{error} {error:?}").contains("synthetic-private"));
        let source = error
            .source()
            .expect("typed HTTP source")
            .downcast_ref::<HttpError>()
            .expect("preserved HTTP type");
        assert_eq!(source.kind(), kind);
        assert_eq!(source.status(), status);
        assert_eq!(source.attempts(), 1);
        if kind == ErrorKind::RateLimited {
            assert_eq!(source.retry_after(), Some(Duration::from_secs(3)));
        }
        if kind == ErrorKind::MalformedResponse {
            assert!(matches!(
                source.cause().expect("JSON cause").kind(),
                CauseKind::Json(_)
            ));
        }
        assert_safe_error(source);
        assert!(matches!(error, LibraryError::PageRead(_)));
        assert_eq!(mock.count(), 1);
    }
}

/// Preserves actual HTTP classifications through offline book access and loaders.
#[tokio::test]
async fn book_failures_keep_typed_http_causes_without_content() {
    use crate::book::{BookError, SectionRequest, assemble_outline, read_section};
    use crate::library::AccessState;

    for (reply, kind, status) in [
        (
            Reply::new(401, b"synthetic-private-body"),
            ErrorKind::SessionUnusable,
            Some(401),
        ),
        (
            Reply::new(403, b"synthetic-private-body"),
            ErrorKind::AccessDenied,
            Some(403),
        ),
        (
            Reply::new(429, b"synthetic-private-body").headers("Retry-After: 3\r\n"),
            ErrorKind::RateLimited,
            Some(429),
        ),
        (
            Reply::new(200, br#"{"value":"synthetic-private-value"}"#),
            ErrorKind::MalformedResponse,
            None,
        ),
    ] {
        let mock = Mock::start(vec![reply.clone(), reply.clone(), reply.clone(), reply]).await;
        let client = mock.client(RequestPolicy {
            retries: 0,
            ..policy()
        });
        for boundary in 0..4 {
            let http = client
                .get_json::<Item>(&mock.origin, None)
                .await
                .expect_err("synthetic HTTP failure");
            let failure = match boundary {
                0 => assemble_outline(
                    "invented-book",
                    |_| Err(BookError::SourceRead(http)),
                    |_| panic!("failed access must not load"),
                )
                .expect_err("outline access failed"),
                1 => assemble_outline(
                    "invented-book",
                    |_| Ok(AccessState::Granted),
                    |_| Err(BookError::SourceRead(http)),
                )
                .expect_err("outline load failed"),
                2 => read_section(
                    SectionRequest {
                        book_id: "invented-book",
                        section_id: "opening",
                        continuation: None,
                    },
                    |_, _| Err(BookError::SourceRead(http)),
                    |_, _| panic!("failed access must not load"),
                )
                .expect_err("section access failed"),
                _ => read_section(
                    SectionRequest {
                        book_id: "invented-book",
                        section_id: "opening",
                        continuation: None,
                    },
                    |_, _| Ok(AccessState::Granted),
                    |_, _| Err(BookError::SourceRead(http)),
                )
                .expect_err("section load failed"),
            };
            let source = failure
                .source()
                .and_then(|source| source.downcast_ref::<HttpError>())
                .expect("retained typed HTTP cause");
            assert_eq!(source.kind(), kind);
            assert_eq!(source.status(), status);
            assert_eq!(source.attempts(), 1);
            if kind == ErrorKind::RateLimited {
                assert_eq!(source.retry_after(), Some(Duration::from_secs(3)));
            }
            assert_safe_error(source);
            assert!(!format!("{failure} {failure:?}").contains("synthetic-private"));
        }
        assert_eq!(mock.count(), 4);
    }
}

/// Checks every printable source, including the typed redaction boundary.
fn assert_safe_error(error: &HttpError) {
    assert!(!format!("{error} {error:?}").contains("synthetic-private"));
    let mut source = error.source();
    while let Some(cause) = source {
        assert!(!format!("{cause} {cause:?}").contains("synthetic-private"));
        assert!(cause.downcast_ref::<FailureCause>().is_some());
        source = cause.source();
    }
}
