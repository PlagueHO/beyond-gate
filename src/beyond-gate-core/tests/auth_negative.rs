//! Bounded negative-authentication evidence without account or credential access.
//!
//! The live test is ignored and separately gated. It sends only absent or invented
//! cookie input. HTTP status is evidence, not a production validation contract.
//! Response bodies, headers, transport causes, and tokens never enter reports.

use std::{
    fmt,
    io::{self, Read},
    time::Duration,
};

use reqwest::{
    blocking::{Client, ClientBuilder},
    header::{ACCEPT, CONTENT_TYPE, COOKIE, HeaderValue},
};
use serde::Deserialize;
use zeroize::Zeroizing;
/// Fixes the sole candidate origin and operation; callers cannot select a live URL.
const AUTH_URL: &str = "https://auth-service.dndbeyond.com/v1/cobalt-token";
/// Requires approval of this exact negative-only scope, not general product approval.
const APPROVAL: &str = "BEYOND_GATE_NEGATIVE_AUTH_APPROVED";
/// Distinguishes explicit approval from an accidentally inherited Boolean flag.
const APPROVAL_VALUE: &str = "absent-and-synthetic-v1";
/// Requires the separately approved body-reading scope, not the status-only scope.
const SHAPE_APPROVAL_VALUE: &str = "absent-json-shape-v1";
/// Bounds a response before inspecting only two non-secret field-presence facts.
const MAX_BODY_BYTES: usize = 64 * 1024;

/// Keeps unexpected success separate from expected negative authentication.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Outcome {
    Rejected,
    Stop,
}

/// Reports a fixed diagnostic without retaining response or request data.
enum CheckError {
    ApprovalRequired,
    Transport,
    UnexpectedStatus,
    InvalidResponse,
    BodyTooLarge,
}

impl fmt::Display for CheckError {
    /// Omits URLs, headers, OS details, and peer data from failures.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::ApprovalRequired => "exact negative-authentication scope approval is required",
            Self::Transport => "negative authentication transport failed; stopped without retry",
            Self::UnexpectedStatus => "negative authentication returned a stop status",
            Self::InvalidResponse => "response shape is not the approved JSON shape; stopped",
            Self::BodyTooLarge => "response shape exceeded 65536 bytes; stopped",
        })
    }
}

impl fmt::Debug for CheckError {
    /// Keeps the test runner's failure output as safe as ordinary diagnostics.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, formatter)
    }
}

/// Selects direct TLS with no proxy, redirects, retries, or automatic cookie jar.
fn client_builder() -> ClientBuilder {
    Client::builder()
        .https_only(true)
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .retry(reqwest::retry::never())
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(10))
        .user_agent("beyond-gate-negative-auth-probe")
}

/// Takes one fixed negative observation and drops the response without reading its body.
///
/// Only the private live entry supplies `AUTH_URL`. Offline tests supply loopback.
/// No caller can supply a cookie; the optional value is compiled synthetic input.
fn observe(client: &Client, url: &str, synthetic: bool) -> Result<(u16, Outcome), CheckError> {
    let mut request = client
        .post(url)
        .header(CONTENT_TYPE, "application/json")
        .header(ACCEPT, "application/json");
    if synthetic {
        let mut header = HeaderValue::from_static("CobaltSession=beyond-gate-invalid-synthetic-v1");
        header.set_sensitive(true);
        request = request.header(COOKIE, header);
    }
    let response = request.send().map_err(|_| CheckError::Transport)?;
    let status = response.status().as_u16();
    // A 401 is the expected negative observation, not a request to bypass denial.
    let outcome = if status == 401 {
        Outcome::Rejected
    } else {
        Outcome::Stop
    };
    drop(response);
    Ok((status, outcome))
}

/// Runs at most two sequential requests and stops on every non-401 or transport failure.
fn negative_checks(client: &Client, url: &str) -> Result<(), CheckError> {
    for (name, synthetic) in [("absent-cookie", false), ("invented-cookie", true)] {
        let (status, outcome) = observe(client, url, synthetic)?;
        eprintln!("negative-auth case={name} status={status} body_read=false retry=false");
        if outcome != Outcome::Rejected {
            return Err(CheckError::UnexpectedStatus);
        }
    }
    Ok(())
}

/// Requires exact scope approval before building a client or making a request.
fn require_approval(value: Option<&str>, expected: &str) -> Result<(), CheckError> {
    if value != Some(expected) {
        return Err(CheckError::ApprovalRequired);
    }
    Ok(())
}

/// Observes missing and invented credentials only, without account or OS-store access.
///
/// A 200, 403, redirect, rate limit, server error, timeout, or transport error stops
/// the entire check. No body is read, no redirect followed, and no retry performed.
/// Two observed 401 statuses do not establish real-session usability or expiry.
#[test]
#[ignore = "requires approval of exact two-request negative-only scope; no real cookie input"]
fn live_negative_authentication() -> Result<(), CheckError> {
    let approval = std::env::var(APPROVAL).ok();
    require_approval(approval.as_deref(), APPROVAL_VALUE)?;
    let client = client_builder()
        .build()
        .map_err(|_| CheckError::Transport)?;
    negative_checks(&client, AUTH_URL)
}

/// Borrows a token from the zeroizing body without creating a returned token allocation.
#[derive(Deserialize)]
struct ResponseFields<'a> {
    #[serde(borrow)]
    token: Option<&'a str>,
    ttl: Option<u64>,
}

/// Contains observations only; this is not a service-validation result.
#[derive(Debug, Eq, PartialEq)]
struct ResponseShape {
    token_present: bool,
    ttl_positive: bool,
}

/// Reads a bounded body into owned zeroizing memory.
fn shape_body(reader: impl Read) -> Result<Zeroizing<Vec<u8>>, CheckError> {
    let mut bytes = Zeroizing::new(Vec::new());
    reader
        .take((MAX_BODY_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| CheckError::Transport)?;
    if bytes.len() > MAX_BODY_BYTES {
        return Err(CheckError::BodyTooLarge);
    }
    Ok(bytes)
}

/// Returns only fixed presence facts, never token text or unrecognized fields.
fn inspect_shape(body: &[u8]) -> Result<ResponseShape, CheckError> {
    let fields: ResponseFields<'_> =
        serde_json::from_slice(body).map_err(|_| CheckError::InvalidResponse)?;
    Ok(ResponseShape {
        token_present: fields.token.is_some_and(|token| !token.trim().is_empty()),
        ttl_positive: fields.ttl.is_some_and(|ttl| ttl > 0),
    })
}

/// Accepts only the approved status and content type before any response-body read.
fn require_shape_response(status: u16, content_type: &str) -> Result<(), CheckError> {
    if status != 200 {
        return Err(CheckError::UnexpectedStatus);
    }
    if !content_type
        .split(';')
        .next()
        .is_some_and(|mime| mime.trim().eq_ignore_ascii_case("application/json"))
    {
        return Err(CheckError::InvalidResponse);
    }
    Ok(())
}

/// Makes one separately approved absent-cookie request and emits only field-presence facts.
///
/// Does not follow a token, decode its claims, or read account/content endpoints.
/// JSON decoder scratch copies are not guaranteed to clear; the owned body is
/// zeroizing and token text is borrowed. Invalid or escaped token input stops.
#[test]
#[ignore = "requires exact one-request absent-cookie JSON-shape scope approval"]
fn live_unauthenticated_response_shape() -> Result<(), CheckError> {
    let approval = std::env::var(APPROVAL).ok();
    require_approval(approval.as_deref(), SHAPE_APPROVAL_VALUE)?;
    let client = client_builder()
        .build()
        .map_err(|_| CheckError::Transport)?;
    let response = client
        .post(AUTH_URL)
        .header(CONTENT_TYPE, "application/json")
        .header(ACCEPT, "application/json")
        .send()
        .map_err(|_| CheckError::Transport)?;
    let status = response.status().as_u16();
    eprintln!("absent-response-shape status={status} retry=false");
    let content_type = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|header| header.to_str().ok())
        .ok_or(CheckError::InvalidResponse)?;
    require_shape_response(status, content_type)?;
    let body = shape_body(response)?;
    let shape = inspect_shape(&body)?;
    eprintln!(
        "absent-response-shape token_present={} ttl_positive={} account_validated=false",
        shape.token_present, shape.ttl_positive
    );
    Ok(())
}

/// Serves bounded synthetic HTTP responses and records only local request headers.
///
/// # Panics
///
/// Panics if the local mock cannot bind, receive its bounded requests, or terminate.
fn mock(statuses: &[u16]) -> (Result<(), CheckError>, Vec<String>) {
    use std::{
        io::{Read, Write},
        net::TcpListener,
        thread,
        time::Instant,
    };

    let listener = TcpListener::bind("127.0.0.1:0").expect("bind local mock");
    listener.set_nonblocking(true).expect("bound mock accept");
    let address = listener.local_addr().expect("read mock address");
    let statuses = statuses.to_vec();
    let worker = thread::spawn(move || {
        let mut requests = Vec::new();
        for status in statuses {
            let deadline = Instant::now() + Duration::from_secs(5);
            let mut stream = loop {
                match listener.accept() {
                    Ok((stream, _)) => break stream,
                    Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                        assert!(Instant::now() < deadline, "mock request timed out");
                        thread::sleep(Duration::from_millis(5));
                    }
                    Err(_) => panic!("local mock accept failed"),
                }
            };
            stream
                .set_nonblocking(false)
                .expect("make accepted mock stream blocking");
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .expect("bound local read");
            stream
                .set_write_timeout(Some(Duration::from_secs(2)))
                .expect("bound local write");
            let mut request = Vec::new();
            let mut buffer = [0; 1024];
            while !request.ends_with(b"\r\n\r\n") {
                let count = stream.read(&mut buffer).expect("read local request");
                assert_ne!(count, 0, "local request ended");
                request.extend_from_slice(&buffer[..count]);
                assert!(request.len() <= 16384, "local request exceeded its bound");
            }
            write!(
                stream,
                "HTTP/1.1 {status} Mock\r\nLocation: http://{address}/forbidden-redirect\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            )
            .expect("write local status");
            requests.push(String::from_utf8(request).expect("synthetic request UTF-8"));
        }
        requests
    });
    let client = client_builder()
        .https_only(false)
        .build()
        .expect("build local mock client");
    let result = negative_checks(&client, &format!("http://{address}/token"));
    (result, worker.join().expect("join local mock"))
}

/// Checks exact request count, absent versus invented cookie, and no bearer input.
#[test]
fn offline_negative_requests_are_fixed_and_bounded() {
    let (result, requests) = mock(&[401, 401]);
    assert!(result.is_ok());
    assert_eq!(requests.len(), 2);
    for request in &requests {
        let request = request.to_ascii_lowercase();
        assert!(request.starts_with("post /token http/1.1\r\n"));
        assert!(!request.contains("\r\nauthorization:"));
    }
    assert!(!requests[0].to_ascii_lowercase().contains("\r\ncookie:"));
    assert!(requests[1].contains("CobaltSession=beyond-gate-invalid-synthetic-v1"));
}

/// Checks that unexpected success, challenge, redirect, rate limit, or server failure stops.
#[test]
fn offline_stop_status_never_sends_second_request() {
    for status in [200, 201, 302, 307, 400, 403, 429, 500, 503] {
        let (result, requests) = mock(&[status]);
        assert!(matches!(result, Err(CheckError::UnexpectedStatus)));
        assert_eq!(requests.len(), 1);
    }
}

/// Checks that a failure on the final request cannot become a negative-check pass.
#[test]
fn offline_second_request_failure_remains_failure() {
    let (result, requests) = mock(&[401, 403]);
    assert!(matches!(result, Err(CheckError::UnexpectedStatus)));
    assert_eq!(requests.len(), 2);
}

/// Checks exact approval text without changing process-global environment variables.
#[test]
fn offline_approval_is_exact() {
    for value in [
        None,
        Some(""),
        Some("1"),
        Some("true"),
        Some("product-login"),
    ] {
        assert!(matches!(
            require_approval(value, APPROVAL_VALUE),
            Err(CheckError::ApprovalRequired)
        ));
    }
    assert!(require_approval(Some(APPROVAL_VALUE), APPROVAL_VALUE).is_ok());
    assert!(require_approval(Some(APPROVAL_VALUE), SHAPE_APPROVAL_VALUE).is_err());
    assert!(require_approval(Some(SHAPE_APPROVAL_VALUE), APPROVAL_VALUE).is_err());
    assert!(require_approval(Some(SHAPE_APPROVAL_VALUE), SHAPE_APPROVAL_VALUE).is_ok());
}

/// Checks that the live client cannot send even invented material over plain HTTP.
#[test]
fn offline_live_client_rejects_plain_http() {
    let client = client_builder().build().expect("build TLS client");
    assert!(matches!(
        observe(&client, "http://127.0.0.1:1/token", true),
        Err(CheckError::Transport)
    ));
}

/// Checks actual JSON shape parsing without returning invented token or account data.
#[test]
fn offline_response_shape_never_returns_material() {
    let shape = inspect_shape(
        br#"{"token":"synthetic-private-token","ttl":300,"unknown":"synthetic-private-account"}"#,
    )
    .expect("synthetic shape");
    assert!(shape.token_present);
    assert!(shape.ttl_positive);
    assert!(!format!("{shape:?}").contains("synthetic-private"));
    for body in [br#"{}"#.as_slice(), br#"{"token":null,"ttl":null}"#] {
        let shape = inspect_shape(body).expect("valid absent fields");
        assert!(!shape.token_present);
        assert!(!shape.ttl_positive);
    }
    let shape = inspect_shape(br#"{"token":" ","ttl":0}"#).expect("empty fields");
    assert!(!shape.token_present);
    assert!(!shape.ttl_positive);
}

/// Checks malformed, duplicate, wrong-type, and non-object input without reflected diagnostics.
#[test]
fn offline_response_shape_rejects_unsafe_input() {
    for body in [
        b"synthetic-private-invalid-json".as_slice(),
        br#"[]"#,
        br#"{"token":42}"#,
        br#"{"ttl":-1}"#,
        br#"{"token":"synthetic","token":"another"}"#,
        br#"{"token":"synthetic\u002dprivate"}"#,
    ] {
        let error = inspect_shape(body).expect_err("invalid response shape");
        assert!(!format!("{error:?}").contains("synthetic"));
    }
}

/// Checks body limits before JSON inspection, including exact-limit trailing whitespace.
#[test]
fn offline_response_shape_body_bound_is_exact() {
    let mut body = br#"{"token":"synthetic","ttl":300}"#.to_vec();
    body.resize(MAX_BODY_BYTES, b' ');
    let bytes = shape_body(body.as_slice()).expect("exact body limit");
    assert!(inspect_shape(&bytes).expect("bounded JSON").token_present);
    body.push(b' ');
    assert!(matches!(
        shape_body(body.as_slice()),
        Err(CheckError::BodyTooLarge)
    ));
}

/// Checks that unexpected statuses and HTML/challenge content stop before body inspection.
#[test]
fn offline_shape_requires_ok_json_response() {
    for status in [201, 302, 401, 403, 429, 500] {
        assert!(require_shape_response(status, "application/json").is_err());
    }
    for content_type in ["", "text/html", "text/plain", "application/jsonp"] {
        assert!(require_shape_response(200, content_type).is_err());
    }
    assert!(require_shape_response(200, "application/json; charset=utf-8").is_ok());
}
