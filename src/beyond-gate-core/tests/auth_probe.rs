//! Developer-only session-exchange probe with synthetic offline contract tests.
//!
//! The live test is ignored by default and requires an interactive terminal.
//! The candidate endpoint and `token` field come from community implementations,
//! not an official D&D Beyond API contract. No product operation uses this code.

use std::error::Error;
use std::fmt;
use std::io::{self, IsTerminal, Read};
use std::time::Duration;

use reqwest::StatusCode;
use reqwest::blocking::{Client, ClientBuilder};
use reqwest::header::{ACCEPT, CONTENT_TYPE, COOKIE, HeaderValue};

/// Limits the live request to the community-documented token exchange.
const AUTH_URL: &str = "https://auth-service.dndbeyond.com/v1/cobalt-token";
/// Bounds the candidate session value before it becomes an HTTP header.
const MAX_COOKIE_BYTES: usize = 8 * 1024;
/// Bounds the entire response, including unknown fields.
const MAX_RESPONSE_BYTES: usize = 64 * 1024;

/// Keeps typed failure causes while excluding secret-bearing data from output.
enum ProbeError {
    TerminalRequired,
    InvalidCookie,
    Http(reqwest::Error),
    Io(io::Error),
    Json(serde_json::Error),
    Status(u16),
    BodyTooLarge,
    MissingToken,
}

impl fmt::Display for ProbeError {
    /// Describes failures without printing input, response bodies, or headers.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TerminalRequired => {
                write!(
                    formatter,
                    "run in an interactive terminal; do not pipe a secret"
                )
            }
            Self::InvalidCookie => write!(formatter, "empty or invalid cookie; no request sent"),
            Self::Http(_) => write!(
                formatter,
                "HTTP client or transport failed; no retry attempted"
            ),
            Self::Io(_) => write!(formatter, "terminal or response read failed"),
            Self::Json(_) => write!(formatter, "response is not valid JSON; body omitted"),
            Self::Status(status) => {
                write!(
                    formatter,
                    "HTTP {status}; stopped without redirect or retry"
                )
            }
            Self::BodyTooLarge => write!(formatter, "response exceeded 65536 bytes; stopped"),
            Self::MissingToken => write!(formatter, "response has no nonempty string token"),
        }
    }
}

impl fmt::Debug for ProbeError {
    /// Uses the same redacted output when the Rust test runner reports failure.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, formatter)
    }
}

impl Error for ProbeError {
    /// Preserves typed causes for inspection without including them in output.
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Http(error) => Some(error),
            Self::Io(error) => Some(error),
            Self::Json(error) => Some(error),
            _ => None,
        }
    }
}

/// Builds a direct HTTPS client with one attempt and bounded connection time.
fn probe_client() -> ClientBuilder {
    Client::builder()
        .https_only(true)
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .retry(reqwest::retry::never())
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(15))
        .user_agent("beyond-gate-auth-probe")
}

/// Accepts one RFC 6265 cookie value and marks its header as sensitive.
///
/// # Errors
///
/// Returns `InvalidCookie` for empty, oversized, or invalid cookie values.
fn session_header(cookie: &str) -> Result<HeaderValue, ProbeError> {
    if cookie.is_empty()
        || cookie.len() > MAX_COOKIE_BYTES
        || !cookie.bytes().all(
            |byte| matches!(byte, 0x21 | 0x23..=0x2b | 0x2d..=0x3a | 0x3c..=0x5b | 0x5d..=0x7e),
        )
    {
        return Err(ProbeError::InvalidCookie);
    }

    let mut header = HeaderValue::from_str(&format!("CobaltSession={cookie}"))
        .map_err(|_| ProbeError::InvalidCookie)?;
    header.set_sensitive(true);
    Ok(header)
}

/// Reads at most the response limit plus one byte to detect excess data.
///
/// # Errors
///
/// Returns an I/O failure or `BodyTooLarge` without exposing the response.
fn bounded_body(reader: impl Read) -> Result<Vec<u8>, ProbeError> {
    let mut body = Vec::new();
    reader
        .take((MAX_RESPONSE_BYTES + 1) as u64)
        .read_to_end(&mut body)
        .map_err(ProbeError::Io)?;
    if body.len() > MAX_RESPONSE_BYTES {
        return Err(ProbeError::BodyTooLarge);
    }
    Ok(body)
}

/// Checks the candidate response shape, then drops the token without returning it.
///
/// # Errors
///
/// Returns a redacted JSON or shape error for an unusable response.
fn validate_token(body: &[u8]) -> Result<(), ProbeError> {
    let response: serde_json::Value = serde_json::from_slice(body).map_err(ProbeError::Json)?;
    match response.get("token").and_then(serde_json::Value::as_str) {
        Some(token) if !token.trim().is_empty() => Ok(()),
        _ => Err(ProbeError::MissingToken),
    }
}

/// Makes one exchange attempt without following redirects or reading account data.
///
/// Only the live test supplies a real cookie, and it always uses `AUTH_URL`.
/// Offline tests supply synthetic values to a loopback mock instead.
///
/// # Errors
///
/// Returns a redacted input, transport, status, size, or response-shape failure.
fn exchange(client: &Client, url: &str, cookie: &str) -> Result<(), ProbeError> {
    let header = session_header(cookie)?;
    let response = client
        .post(url)
        .header(COOKIE, header)
        .header(CONTENT_TYPE, "application/json")
        .header(ACCEPT, "application/json")
        .send()
        .map_err(ProbeError::Http)?;
    if response.status() != StatusCode::OK {
        return Err(ProbeError::Status(response.status().as_u16()));
    }
    validate_token(&bounded_body(response)?)
}

/// Tries one real token exchange using hidden local input and no persistent store.
///
/// # Errors
///
/// Returns a safe failure if the terminal, input, exchange, or response is unusable.
#[test]
#[ignore = "manual live account test; requires approval and a hidden terminal prompt"]
fn live_authentication() -> Result<(), ProbeError> {
    if !io::stdin().is_terminal() || !io::stderr().is_terminal() {
        return Err(ProbeError::TerminalRequired);
    }

    let client = probe_client().build().map_err(ProbeError::Http)?;
    eprintln!("One direct token exchange only. No account data will be read.");
    eprintln!("Proceed only for your own approved session. Empty input cancels.");
    let cookie = rpassword::prompt_password("CobaltSession (hidden): ").map_err(ProbeError::Io)?;
    exchange(&client, AUTH_URL, &cookie)?;
    eprintln!("Token exchange succeeded. Token discarded; data access and expiry not tested.");
    Ok(())
}

/// Serves one synthetic response and returns the request seen by the local mock.
///
/// # Panics
///
/// Panics if local sockets, the worker, or the synthetic HTTP exchange fails.
fn mock_exchange(status: u16, body: &[u8]) -> (Result<(), ProbeError>, String) {
    use std::io::Write;
    use std::net::TcpListener;
    use std::thread;
    use std::time::Instant;

    let listener = TcpListener::bind("127.0.0.1:0").expect("bind local mock");
    let address = listener.local_addr().expect("read mock address");
    listener.set_nonblocking(true).expect("bound mock accept");
    let body = body.to_vec();
    let worker = thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut stream = loop {
            match listener.accept() {
                Ok((stream, _)) => break stream,
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                    assert!(Instant::now() < deadline, "mock request timed out");
                    thread::sleep(Duration::from_millis(10));
                }
                Err(error) => panic!("mock accept failed: {error}"),
            }
        };
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .expect("bound mock read");
        stream
            .set_write_timeout(Some(Duration::from_secs(5)))
            .expect("bound mock write");
        let mut request = Vec::new();
        let mut buffer = [0; 1024];
        while !request.ends_with(b"\r\n\r\n") {
            let count = stream.read(&mut buffer).expect("read synthetic request");
            assert_ne!(count, 0, "request ended before headers");
            request.extend_from_slice(&buffer[..count]);
            assert!(request.len() <= 16 * 1024, "request headers too large");
        }
        write!(
            stream,
            "HTTP/1.1 {status} Mock\r\nContent-Length: {}\r\nLocation: http://{address}/redirected\r\nConnection: close\r\n\r\n",
            body.len()
        )
        .expect("write synthetic response headers");
        stream.write_all(&body).expect("write synthetic body");
        String::from_utf8(request).expect("synthetic request is UTF-8")
    });

    let client = probe_client()
        .https_only(false)
        .build()
        .expect("build loopback test client");
    let result = exchange(
        &client,
        &format!("http://{address}/token"),
        "synthetic-session",
    );
    (result, worker.join().expect("join mock"))
}

/// Checks that a synthetic exchange uses only the expected method and credential.
#[test]
fn offline_exchange_accepts_token_without_returning_it() {
    let (result, request) = mock_exchange(200, br#"{"token":"synthetic-token","extra":true}"#);
    assert!(result.is_ok());
    let request = request.to_ascii_lowercase();
    assert!(request.starts_with("post /token http/1.1\r\n"));
    assert!(request.contains("\r\ncookie: cobaltsession=synthetic-session\r\n"));
    assert!(!request.contains("\r\nauthorization:"));
}

/// Checks that redirects, rejected sessions, rate limits, and failures stop.
#[test]
fn offline_exchange_stops_on_non_ok_status() {
    for status in [201, 302, 307, 401, 403, 429, 500] {
        let (result, _) = mock_exchange(status, b"synthetic response not to be printed");
        assert!(matches!(result, Err(ProbeError::Status(actual)) if actual == status));
    }
}

/// Checks that HTML, malformed JSON, and invalid token fields cannot pass.
#[test]
fn offline_exchange_rejects_unusable_bodies() {
    for body in [
        b"<html>synthetic challenge</html>".as_slice(),
        br#"{"token":"unfinished"#,
        br#"{}"#,
        br#"{"token":null}"#,
        br#"{"token":42}"#,
        br#"{"token":""}"#,
        br#"{"token":"   "}"#,
    ] {
        let (result, _) = mock_exchange(200, body);
        assert!(result.is_err());
    }
}

/// Checks the exact response-size threshold, including unknown trailing data.
#[test]
fn offline_body_limit_is_exact() {
    let mut body = br#"{"token":"synthetic-token"}"#.to_vec();
    body.resize(MAX_RESPONSE_BYTES, b' ');
    assert!(validate_token(&bounded_body(body.as_slice()).expect("at limit")).is_ok());
    body.push(b' ');
    assert!(matches!(
        bounded_body(body.as_slice()),
        Err(ProbeError::BodyTooLarge)
    ));
    let (result, _) = mock_exchange(200, &body);
    assert!(matches!(result, Err(ProbeError::BodyTooLarge)));
}

/// Checks that the live client cannot send a credential over plain HTTP.
#[test]
fn offline_live_client_requires_https() {
    let client = probe_client().build().expect("build HTTPS client");
    let result = exchange(&client, "http://127.0.0.1:1/token", "synthetic-session");
    assert!(matches!(result, Err(ProbeError::Http(_))));
}

/// Checks that a noninteractive process exits before asking for a real session.
#[test]
fn offline_noninteractive_run_stops_before_prompt() {
    use std::process::{Command, Stdio};

    let output = Command::new(std::env::current_exe().expect("locate probe test binary"))
        .args(["--ignored", "--exact", "live_authentication", "--nocapture"])
        .stdin(Stdio::null())
        .output()
        .expect("run probe with noninteractive streams");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("run in an interactive terminal"));
    assert!(!stderr.contains("CobaltSession (hidden):"));
    assert!(!stderr.contains("Token exchange succeeded"));
}

/// Checks that invalid input cannot add cookies or inject request headers.
#[test]
fn offline_cookie_validation_rejects_unsafe_input() {
    for cookie in [
        "",
        "two values",
        "a;b=c",
        "a,b",
        "a\"b",
        "a\\b",
        "a\r\nx:y",
        "a\u{80}",
    ] {
        assert!(matches!(
            session_header(cookie),
            Err(ProbeError::InvalidCookie)
        ));
    }
    assert!(session_header(&"x".repeat(MAX_COOKIE_BYTES)).is_ok());
    assert!(matches!(
        session_header(&"x".repeat(MAX_COOKIE_BYTES + 1)),
        Err(ProbeError::InvalidCookie)
    ));
}

/// Checks that header debug output cannot reveal the synthetic session.
#[test]
fn offline_cookie_header_is_sensitive() {
    let header = session_header("synthetic-private-session").expect("valid synthetic cookie");
    assert!(header.is_sensitive());
    assert!(!format!("{header:?}").contains("synthetic-private-session"));
}

/// Checks that retained failure causes do not enter user-visible diagnostics.
#[test]
fn offline_errors_redact_response_and_io_details() {
    let json_error = validate_token(b"synthetic-private-body").expect_err("invalid JSON");
    let io_error = ProbeError::Io(io::Error::other("synthetic-private-session"));
    for error in [json_error, io_error] {
        assert!(error.source().is_some());
        assert!(!format!("{error}").contains("synthetic-private"));
        assert!(!format!("{error:?}").contains("synthetic-private"));
    }
}
