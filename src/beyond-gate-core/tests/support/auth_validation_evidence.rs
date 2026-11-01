//! Explicitly approved, bounded comparison of anonymous and real exchange responses.
//!
//! Reports fixed field classifications only. JWT decoding is not signature
//! validation and never establishes an authenticated product session.
//! All live requests remain ignored by default. No store or content API is used.

use std::io::{self, IsTerminal, Read};
use std::time::{SystemTime, UNIX_EPOCH};

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use reqwest::{
    blocking::Client,
    header::{ACCEPT, ACCEPT_ENCODING, CONTENT_ENCODING, CONTENT_TYPE, COOKIE},
};
use serde::Deserialize;
use serde_json::value::RawValue;
use zeroize::Zeroizing;

use super::{AUTH_URL, MAX_RESPONSE_BYTES, ProbeError, probe_client, session_header};

/// Distinguishes this approval from the older exchange and negative-only scopes.
const APPROVAL: &str = "BEYOND_GATE_VALIDATION_EVIDENCE_APPROVED";
/// Identifies precisely three fixed cases and restricted response inspection.
const APPROVAL_VALUE: &str = "absent-invented-real-fields-v1";
/// Authorizes exactly one separate real-cookie request after the negative scope stopped.
const POSITIVE_APPROVAL_VALUE: &str = "one-real-cookie-fields-v1";
/// Caps a returned token before any base64 decoding.
const MAX_TOKEN_BYTES: usize = 8192;
/// Fits the shared cookie name-plus-value limit without changing product behavior.
const MAX_VALUE_BYTES: usize = 8179;

/// Borrows only approved fields; all other response and claim values are ignored.
#[derive(Deserialize)]
struct Fields<'a> {
    #[serde(borrow)]
    token: Option<&'a str>,
    ttl: Option<u64>,
    #[serde(borrow, rename = "userId", alias = "user_id")]
    user_id: Option<&'a RawValue>,
    #[serde(
        borrow,
        rename = "http://schemas.xmlsoap.org/ws/2005/05/identity/claims/nameidentifier"
    )]
    name_identifier: Option<&'a RawValue>,
    #[serde(borrow)]
    sub: Option<&'a RawValue>,
    #[serde(rename = "isAnonymous", alias = "anonymous", alias = "is_anonymous")]
    anonymous: Option<bool>,
    exp: Option<u64>,
}

/// Reports the type/category of an identifier without returning its value.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Identifier {
    Missing,
    Zero,
    Positive,
    Other,
}

/// Contains only fixed categories and presence/expiry flags, never account values.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Facts {
    user_id: Identifier,
    name_identifier: Identifier,
    subject: Identifier,
    anonymous: Option<bool>,
    expiry_present: bool,
    expiry_future: bool,
}

/// Contains no token, raw response, identifier, or usable authentication grant.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Evidence {
    token_present: bool,
    ttl_positive: bool,
    response: Facts,
    claims: Option<Facts>,
}

/// Rejects inherited or generic approval before any prompt, client, or request.
fn require_approval(value: Option<&str>) -> Result<(), ProbeError> {
    if value != Some(APPROVAL_VALUE) {
        return Err(ProbeError::ApprovalRequired);
    }
    Ok(())
}

/// Classifies a scalar identifier without allocating or exposing its text.
fn identifier(value: Option<&RawValue>) -> Result<Identifier, ProbeError> {
    let Some(value) = value else {
        return Ok(Identifier::Missing);
    };
    let raw = value.get();
    let numeric = if raw.starts_with('"') {
        let text: &str = serde_json::from_str(raw).map_err(ProbeError::Json)?;
        text.parse::<u64>().ok()
    } else if raw.starts_with(|character: char| character.is_ascii_digit()) {
        raw.parse::<u64>().ok()
    } else if raw == "null" {
        return Ok(Identifier::Missing);
    } else {
        return Err(ProbeError::InvalidEvidence);
    };
    Ok(match numeric {
        Some(0) => Identifier::Zero,
        Some(_) => Identifier::Positive,
        None => Identifier::Other,
    })
}

/// Reduces borrowed approved fields to fixed facts; no identifier is retained.
fn facts(fields: &Fields<'_>, now: u64) -> Result<Facts, ProbeError> {
    Ok(Facts {
        user_id: identifier(fields.user_id)?,
        name_identifier: identifier(fields.name_identifier)?,
        subject: identifier(fields.sub)?,
        anonymous: fields.anonymous,
        expiry_present: fields.exp.is_some(),
        expiry_future: fields.exp.is_some_and(|expiry| expiry > now),
    })
}

/// Decodes a bounded JWT payload for field evidence, not authentication.
fn inspect(body: &[u8], now: u64) -> Result<Evidence, ProbeError> {
    if body.len() > MAX_RESPONSE_BYTES {
        return Err(ProbeError::BodyTooLarge);
    }
    let response: Fields<'_> = serde_json::from_slice(body).map_err(ProbeError::Json)?;
    let Some(token) = response.token.filter(|token| !token.trim().is_empty()) else {
        return Ok(Evidence {
            token_present: false,
            ttl_positive: response.ttl.is_some_and(|ttl| ttl > 0),
            response: facts(&response, now)?,
            claims: None,
        });
    };
    if token.len() > MAX_TOKEN_BYTES {
        return Err(ProbeError::InvalidEvidence);
    }
    let mut segments = token.split('.');
    let header = segments.next().ok_or(ProbeError::InvalidEvidence)?;
    let payload = segments.next().ok_or(ProbeError::InvalidEvidence)?;
    let signature = segments.next().ok_or(ProbeError::InvalidEvidence)?;
    if header.is_empty()
        || payload.is_empty()
        || signature.is_empty()
        || segments.next().is_some()
        || ![header, payload, signature].iter().all(|segment| {
            segment
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"-_".contains(&byte))
        })
    {
        return Err(ProbeError::InvalidEvidence);
    }
    let decoded = Zeroizing::new(
        URL_SAFE_NO_PAD
            .decode(payload)
            .map_err(ProbeError::Base64)?,
    );
    let claims: Fields<'_> = serde_json::from_slice(&decoded).map_err(ProbeError::Json)?;
    Ok(Evidence {
        token_present: true,
        ttl_positive: response.ttl.is_some_and(|ttl| ttl > 0),
        response: facts(&response, now)?,
        claims: Some(facts(&claims, now)?),
    })
}

/// Clears owned response bytes on success and every partial-read or limit failure.
fn bounded_body(reader: impl Read) -> Result<Zeroizing<Vec<u8>>, ProbeError> {
    let mut body = Zeroizing::new(Vec::new());
    reader
        .take((MAX_RESPONSE_BYTES + 1) as u64)
        .read_to_end(&mut body)
        .map_err(ProbeError::Io)?;
    if body.len() > MAX_RESPONSE_BYTES {
        return Err(ProbeError::BodyTooLarge);
    }
    Ok(body)
}

/// Makes exactly one direct attempt; failure stops the caller's remaining cases.
fn observe(client: &Client, url: &str, cookie: Option<&str>) -> Result<Evidence, ProbeError> {
    let mut request = client
        .post(url)
        .header(CONTENT_TYPE, "application/json")
        .header(ACCEPT, "application/json")
        .header(ACCEPT_ENCODING, "identity");
    if let Some(cookie) = cookie {
        if cookie.len() > MAX_VALUE_BYTES {
            return Err(ProbeError::InvalidCookie);
        }
        request = request.header(COOKIE, session_header(cookie)?);
    }
    println!("validation-evidence request_attempt_started=true");
    let response = request.send().map_err(ProbeError::Http)?;
    if response.status().as_u16() != 200 {
        return Err(ProbeError::Status(response.status().as_u16()));
    }
    let content_type = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .ok_or(ProbeError::InvalidEvidence)?;
    if !content_type
        .split(';')
        .next()
        .is_some_and(|value| value.trim().eq_ignore_ascii_case("application/json"))
        || response
            .headers()
            .get(CONTENT_ENCODING)
            .is_some_and(|value| !value.as_bytes().eq_ignore_ascii_case(b"identity"))
    {
        return Err(ProbeError::InvalidEvidence);
    }
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| ProbeError::Io(io::Error::other(error)))?
        .as_secs();
    inspect(&bounded_body(response)?, now)
}

/// Prints fixed labels and categories only; this result never claims validation.
fn report(case: &str, evidence: Evidence) {
    println!(
        "validation-evidence case={case} status=200 token_present={} ttl_positive={} response={:?} claims={:?} account_validated=false saved=false",
        evidence.token_present, evidence.ttl_positive, evidence.response, evidence.claims
    );
}

/// Uses the separately approved three-case scope and a hidden real-cookie prompt.
#[test]
#[ignore = "requires exact three-request approval and a hidden local terminal prompt"]
fn live_validation_evidence() -> Result<(), ProbeError> {
    run_live(false)
}

/// Runs only the two unspent cases after a probe bug stopped the absent-cookie observation.
#[test]
#[ignore = "requires the approved three-request scope with exactly two cases remaining"]
fn live_remaining_validation_evidence() -> Result<(), ProbeError> {
    run_live(true)
}

/// Uses only the separately approved single real-cookie scope, never repeating negatives.
#[test]
#[ignore = "requires exact one-real-cookie approval and hidden local input"]
fn live_positive_validation_evidence() -> Result<(), ProbeError> {
    let approval = std::env::var(APPROVAL).ok();
    if approval.as_deref() != Some(POSITIVE_APPROVAL_VALUE) {
        return Err(ProbeError::ApprovalRequired);
    }
    if !io::stdin().is_terminal() || !io::stderr().is_terminal() {
        return Err(ProbeError::TerminalRequired);
    }
    let client = probe_client().build().map_err(ProbeError::Http)?;
    let result = positive_case(&client);
    if let Err(error) = &result {
        println!("validation-evidence stopped={error} account_validated=false saved=false");
    }
    result
}

/// Gates either fixed case sequence before input and preserves every stop failure.
fn run_live(remaining_only: bool) -> Result<(), ProbeError> {
    let approval = std::env::var(APPROVAL).ok();
    require_approval(approval.as_deref())?;
    if !io::stdin().is_terminal() || !io::stderr().is_terminal() {
        return Err(ProbeError::TerminalRequired);
    }
    let result = approved_cases(remaining_only);
    if let Err(error) = &result {
        println!("validation-evidence stopped={error} account_validated=false saved=false");
    }
    result
}

/// Executes only the three approved cases and stops before any subsequent case on failure.
fn approved_cases(remaining_only: bool) -> Result<(), ProbeError> {
    let client = probe_client().build().map_err(ProbeError::Http)?;
    for (case, cookie) in [
        ("absent", None),
        ("invented", Some("beyond-gate-invalid-synthetic-v1")),
    ]
    .into_iter()
    .skip(usize::from(remaining_only))
    {
        report(case, observe(&client, AUTH_URL, cookie)?);
    }
    positive_case(&client)
}

/// Reads one cookie through the terminal and makes one exchange without saving or validation.
fn positive_case(client: &Client) -> Result<(), ProbeError> {
    eprintln!("One real-cookie observation only. No product session has been validated.");
    eprintln!(
        "Enter only your approved CobaltSession. Empty input cancels the real-cookie request."
    );
    println!("validation-evidence hidden_input_pending=true");
    let cookie = Zeroizing::new(
        rpassword::prompt_password("CobaltSession (hidden; never enter it in chat): ")
            .map_err(ProbeError::Io)?,
    );
    if cookie.is_empty() {
        return Err(ProbeError::InvalidCookie);
    }
    println!("validation-evidence hidden_input_received=true");
    let evidence = observe(client, AUTH_URL, Some(&cookie))?;
    report("real", evidence);
    if !evidence.token_present {
        return Err(ProbeError::MissingToken);
    }
    Ok(())
}

/// Builds an invented JWT-shaped envelope without a real signature or account.
fn fixture(claims: &str) -> Vec<u8> {
    let payload = URL_SAFE_NO_PAD.encode(claims);
    format!(r#"{{"token":"e30.{payload}.c3ludGhldGlj","ttl":300}}"#).into_bytes()
}

/// Checks numeric and string identifiers without including either value in output.
#[test]
fn offline_fixed_facts_omit_all_material() {
    let body = fixture(
        r#"{"userId":42,"sub":"synthetic-private-account","http://schemas.xmlsoap.org/ws/2005/05/identity/claims/nameidentifier":"0","isAnonymous":false,"exp":101,"private":"synthetic-private-data"}"#,
    );
    let result = inspect(&body, 100).expect("invented facts");
    let claims = result.claims.expect("invented token claims");
    assert_eq!(claims.user_id, Identifier::Positive);
    assert_eq!(claims.name_identifier, Identifier::Zero);
    assert_eq!(claims.subject, Identifier::Other);
    assert_eq!(claims.anonymous, Some(false));
    assert!(claims.expiry_future);
    assert!(!format!("{result:?}").contains("synthetic-private"));
    assert!(!format!("{result:?}").contains("42"));
    assert!(!format!("{result:?}").contains("101"));
}

/// Checks missing, zero, unknown and elapsed expiry without guessing login status.
#[test]
fn offline_anonymous_and_expiry_categories_are_explicit() {
    let result = inspect(
        &fixture(r#"{"user_id":"0","anonymous":true,"exp":100}"#),
        100,
    )
    .expect("invented anonymous fields");
    let claims = result.claims.expect("invented token claims");
    assert_eq!(claims.user_id, Identifier::Zero);
    assert_eq!(claims.anonymous, Some(true));
    assert!(claims.expiry_present);
    assert!(!claims.expiry_future);
    let result = inspect(&fixture("{}"), 100).expect("missing fields");
    let claims = result.claims.expect("invented token claims");
    assert_eq!(claims.subject, Identifier::Missing);
    assert_eq!(claims.anonymous, None);
    assert!(!claims.expiry_present);
}

/// Treats the recorded absent/null token shape as negative evidence, never usable login.
#[test]
fn offline_missing_token_is_a_negative_observation_not_a_parser_failure() {
    for body in [
        br#"{"token":null,"ttl":300}"#.as_slice(),
        br#"{"ttl":300}"#,
        br#"{"token":"","ttl":300}"#,
    ] {
        let result = inspect(body, 0).expect("recorded negative shape");
        assert!(!result.token_present);
        assert!(result.ttl_positive);
        assert!(result.claims.is_none());
    }
}

/// Rejects malformed encoding, duplicate approved fields and non-scalar identifiers.
#[test]
fn offline_unexpected_shapes_stop_without_material_in_errors() {
    for body in [
        br#"{"token":"opaque-private-token"}"#.to_vec(),
        br#"{"token":"a.b.c.d"}"#.to_vec(),
        br#"{"token":"a.=.c"}"#.to_vec(),
        br#"{"token":"e30.e30.sig","token":"synthetic-private"}"#.to_vec(),
        fixture(r#"{"userId":[]}"#),
        fixture(r#"{"userId":1,"user_id":2}"#),
        fixture(r#"{"isAnonymous":"synthetic-private"}"#),
    ] {
        let error = inspect(&body, 100).expect_err("unexpected shape");
        assert!(!format!("{error:?}").contains("synthetic-private"));
    }
}

/// Checks the exact response bound while retaining zeroizing owned bytes.
#[test]
fn offline_body_limit_is_exact() {
    let mut body = fixture("{}");
    body.resize(MAX_RESPONSE_BYTES, b' ');
    assert!(inspect(&bounded_body(body.as_slice()).expect("exact limit"), 0).is_ok());
    body.push(b' ');
    assert!(matches!(
        bounded_body(body.as_slice()),
        Err(ProbeError::BodyTooLarge)
    ));
}

/// Rejects generic approval and redirected input before any live operation.
#[test]
fn offline_approval_and_terminal_gate_stop_before_requests() {
    use std::process::{Command, Stdio};
    for value in [None, Some("true"), Some("absent-json-shape-v1")] {
        assert!(require_approval(value).is_err());
    }
    assert!(require_approval(Some(APPROVAL_VALUE)).is_ok());
    for (entry, approval) in [
        ("live_validation_evidence", APPROVAL_VALUE),
        ("live_remaining_validation_evidence", APPROVAL_VALUE),
        ("live_positive_validation_evidence", POSITIVE_APPROVAL_VALUE),
    ] {
        let output = Command::new(std::env::current_exe().expect("probe binary"))
            .args([
                "--ignored",
                "--exact",
                &format!("validation_evidence::{entry}"),
                "--nocapture",
            ])
            .env(APPROVAL, approval)
            .stdin(Stdio::null())
            .output()
            .expect("redirected probe");
        assert!(!output.status.success());
        assert!(!String::from_utf8_lossy(&output.stderr).contains("CobaltSession (hidden"));
        assert!(!String::from_utf8_lossy(&output.stdout).contains("validation-evidence case="));
    }
}

/// Exercises the actual request and response path against one bounded local mock per case.
#[test]
fn offline_exchange_checks_headers_and_stops_on_denial_or_unexpected_content() {
    use std::io::Write;
    use std::net::TcpListener;
    use std::thread;
    use std::time::{Duration, Instant};

    for (status, content_type, encoding, accepted) in [
        (200, "application/json", "", true),
        (200, "text/html", "", false),
        (200, "application/json", "Content-Encoding: gzip\r\n", false),
        (401, "application/json", "", false),
        (403, "application/json", "", false),
        (429, "application/json", "", false),
        (302, "application/json", "", false),
    ] {
        let listener = TcpListener::bind("127.0.0.1:0").expect("mock");
        listener.set_nonblocking(true).expect("bounded accept");
        let address = listener.local_addr().expect("mock address");
        let body = fixture("{}");
        let worker = thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(3);
            let mut stream = loop {
                match listener.accept() {
                    Ok((stream, _)) => break stream,
                    Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                        assert!(Instant::now() < deadline, "mock accept deadline");
                        thread::sleep(Duration::from_millis(5));
                    }
                    Err(_) => panic!("mock accept failed"),
                }
            };
            stream.set_nonblocking(false).expect("blocking mock stream");
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .expect("read bound");
            let mut request = Vec::new();
            while !request.ends_with(b"\r\n\r\n") {
                let mut byte = [0];
                stream.read_exact(&mut byte).expect("mock request");
                request.push(byte[0]);
                assert!(request.len() < 4096, "synthetic header bound");
            }
            let mut response = format!(
                "HTTP/1.1 {status} Synthetic\r\nContent-Type: {content_type}\r\n{encoding}Content-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            ).into_bytes();
            response.extend_from_slice(&body);
            stream.write_all(&response).expect("mock response");
            String::from_utf8(request).expect("synthetic request")
        });
        let client = probe_client()
            .https_only(false)
            .build()
            .expect("mock client");
        let result = observe(
            &client,
            &format!("http://{address}/token"),
            Some("synthetic-session"),
        );
        assert_eq!(result.is_ok(), accepted);
        let request = worker.join().expect("mock worker").to_ascii_lowercase();
        assert!(request.starts_with("post /token http/1.1\r\n"));
        assert!(request.contains("cookie: cobaltsession=synthetic-session\r\n"));
        assert!(request.contains("accept-encoding: identity\r\n"));
    }
}
