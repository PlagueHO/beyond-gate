//! Disabled mobile metadata investigation with synthetic transport and redaction checks.
//!
//! The candidate is not a content-access grant or a production source. Running the
//! ignored entry requires separate approval; knowing its acknowledgment is not permission.

use super::*;

/// Separates this proposed one-request scope from every consumed library scope.
pub(super) const APPROVAL: &str = "issue-33-mobile-metadata-one-attempt";
/// Selects metadata only, never book codes, signed URLs, or book downloads.
const URL: &str = "https://www.dndbeyond.com/mobile/api/v6/available-user-content";

/// Encodes one cookie as form data without placing credentials in a URL or header.
fn token_form(cookie: &BrowserSessionCookie) -> Zeroizing<Vec<u8>> {
    let mut form = Zeroizing::new(Vec::with_capacity(6 + cookie.0.len() * 3));
    form.extend_from_slice(b"token=");
    let hex = b"0123456789ABCDEF";
    for &byte in cookie.0.iter() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            form.push(byte);
        } else {
            form.extend_from_slice(&[
                b'%',
                hex[usize::from(byte >> 4)],
                hex[usize::from(byte & 15)],
            ]);
        }
    }
    form
}

/// Allows only the fixed candidate or an explicit synthetic loopback target.
fn check_target(url: &str, loopback: bool) -> Result<(), ProbeError> {
    super::check_target(url, URL, loopback)
}

/// Counts the single POST before sending; neither errors nor redirects cause another request.
async fn metadata(
    client: &Client,
    cookie: &BrowserSessionCookie,
    url: &str,
    loopback: bool,
    attempts: &mut usize,
) -> Result<Value, ProbeError> {
    check_target(url, loopback)?;
    if *attempts != 0 {
        return Err(ProbeError::RequestLimit);
    }
    *attempts += 1;
    let form = token_form(cookie);
    let response = client
        .post(url)
        .header(ACCEPT, "application/json")
        .header(ACCEPT_ENCODING, "identity")
        .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(form.to_vec())
        .send()
        .await
        .map_err(ProbeError::Transport)?;
    body(response).await
}

/// Checks only the candidate envelope and bounded metadata, never ownership semantics.
fn licenses(value: &Value) -> Result<(&[Value], usize), ProbeError> {
    if value.get("status").and_then(Value::as_str) != Some("success") {
        return Err(ProbeError::Shape);
    }
    let data = value.get("data").unwrap_or(value);
    let licenses = data
        .get("Licenses")
        .and_then(Value::as_array)
        .ok_or(ProbeError::Shape)?;
    if licenses.len() > MAX_RECORDS {
        return Err(ProbeError::RecordLimit);
    }
    let mut count = 0;
    for license in licenses {
        let entity_type = license.get("EntityTypeID").ok_or(ProbeError::Shape)?;
        if !entity_type.is_string() && !entity_type.is_number() {
            return Err(ProbeError::Shape);
        }
        let entities = license
            .get("Entities")
            .and_then(Value::as_array)
            .ok_or(ProbeError::Shape)?;
        if entities.len() > MAX_RECORDS - count {
            return Err(ProbeError::RecordLimit);
        }
        count += entities.len();
        for entity in entities {
            let id = entity.get("id").ok_or(ProbeError::Shape)?;
            if (!id.is_string() && !id.is_number())
                || entity
                    .get("isOwned")
                    .is_some_and(|owned| !owned.is_boolean())
            {
                return Err(ProbeError::Shape);
            }
        }
    }
    Ok((licenses, count))
}

/// Describes fixed field names and types only; account keys and all values are omitted.
fn schema(value: &Value, path: &str, depth: usize, output: &mut BTreeSet<String>) {
    output.insert(format!("{path}:{}", kind(value)));
    if depth == 0 {
        return;
    }
    match value {
        Value::Object(fields) => {
            for (name, value) in fields {
                if matches!(
                    name.as_str(),
                    "Licenses" | "EntityTypeID" | "Entities" | "id" | "isOwned"
                ) {
                    schema(value, &format!("{path}.{name}"), depth - 1, output);
                } else {
                    output.insert(format!("{path}.[unknown_field]"));
                }
            }
        }
        Value::Array(values) => {
            for value in values.iter().take(MAX_RECORDS) {
                schema(value, &format!("{path}[]"), depth - 1, output);
            }
        }
        _ => {}
    }
}

/// Emits bounded structural observations without converting any license into access evidence.
fn inspect(value: &Value) -> Result<String, ProbeError> {
    let (licenses, entities) = licenses(value)?;
    let mut fields = BTreeSet::new();
    schema(value.get("data").unwrap_or(value), "root", 5, &mut fields);
    let mut output = fields.into_iter().collect::<Vec<_>>().join("\n");
    output.push_str(&format!(
        "\nlicenses_count={}; entities_count={entities}; content_access=unverified",
        licenses.len()
    ));
    Ok(output)
}

/// Loads the fixed entry once and makes one metadata POST, with no bearer exchange or writes.
async fn live_check() -> Result<(), ProbeError> {
    let session = Session::default();
    let deadline = Instant::now() + Duration::from_secs(30);
    let cookie = session
        .load_cookie(deadline)
        .await
        .map_err(ProbeError::Session)?
        .ok_or_else(|| ProbeError::Session(SessionError::new(SessionErrorKind::Absent)))?;
    let mut attempts = 0;
    let value = metadata(&client()?, &cookie, URL, false, &mut attempts).await?;
    drop(cookie);
    eprintln!("{}", inspect(&value)?);
    eprintln!("metadata_probe=completed; attempts={attempts}; stored_session=unchanged");
    Ok(())
}

/// Remains ignored until the operator separately approves the documented exact scope.
#[tokio::test]
#[ignore = "requires separate issue #33 approval for one protected read and one metadata POST"]
async fn live_mobile_metadata() -> Result<(), ProbeError> {
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

/// Rejects missing, earlier, and isolated acknowledgments without store or network access.
#[test]
fn approval_and_target_guards() {
    for value in [
        None,
        Some(""),
        Some(super::APPROVAL),
        Some(CONTRACT_APPROVAL),
    ] {
        assert!(check_approval(value, APPROVAL, false).is_err());
    }
    assert!(check_approval(Some(APPROVAL), APPROVAL, true).is_err());
    assert!(check_approval(Some(APPROVAL), APPROVAL, false).is_ok());
    assert!(check_target(URL, false).is_ok());
    assert!(check_target("http://127.0.0.1:1234/metadata", true).is_ok());
    for url in [
        "http://127.0.0.1:1234/metadata",
        "https://www.dndbeyond.com/mobile/api/v6/book-codes",
        "https://www.dndbeyond.com/mobile/api/v6/available-user-content?token=synthetic",
        "https://foreign.invalid/metadata",
    ] {
        assert!(check_target(url, false).is_err());
    }
    for url in [
        "http://localhost:1234/metadata",
        "http://synthetic@127.0.0.1:1234/metadata",
        "http://127.0.0.1:1234/book",
        "http://127.0.0.1:1234/metadata?token=synthetic",
        "http://127.0.0.1:1234/metadata#fragment",
    ] {
        assert!(check_target(url, true).is_err());
    }
}

/// Keeps form separators and special cookie bytes inside one invented token field.
#[test]
fn form_encoding_is_exact() {
    let cookie =
        BrowserSessionCookie::new(b"synthetic+&=%/token".to_vec()).expect("invented cookie");
    assert_eq!(
        &*token_form(&cookie),
        b"token=synthetic%2B%26%3D%25%2Ftoken"
    );
}

/// Emits no identifiers, scalar values, private keys, or upstream messages.
#[test]
fn summaries_are_structural_not_access_grants() {
    let value = serde_json::json!({
        "status": "success",
        "data": {
            "Licenses": [{
                "EntityTypeID": "private-type",
                "Entities": [{"id":"private-id", "isOwned":true, "private-key":"private-value"}]
            }],
            "private-account": "private-name"
        }
    });
    let summary = inspect(&value).expect("invented metadata");
    assert!(!summary.contains("private"));
    assert!(summary.contains("root.Licenses[].Entities[].isOwned:boolean"));
    assert!(summary.contains("licenses_count=1; entities_count=1; content_access=unverified"));
    for value in [
        serde_json::json!({"status":"success","Licenses":[]}),
        serde_json::json!({"status":"success","data":{"Licenses":[]}}),
    ] {
        assert!(
            inspect(&value)
                .expect("empty metadata")
                .contains("content_access=unverified")
        );
    }
    for value in [
        serde_json::json!({"status":"denied","message":"private-error"}),
        serde_json::json!({"status":"success","data":null}),
        serde_json::json!({"status":"success","Licenses":[{"EntityTypeID":1,"Entities":[{}]}]}),
        serde_json::json!({"status":"success","Licenses":[{"EntityTypeID":1,"Entities":[{"id":1,"isOwned":"true"}]}]}),
    ] {
        assert!(inspect(&value).is_err());
    }
}

/// Counts raw licenses and entities across all types before any structural output.
#[test]
fn metadata_record_bounds_are_exact() {
    let entity = serde_json::json!({"id":1});
    let license = serde_json::json!({"EntityTypeID":1,"Entities":[]});
    for count in [MAX_RECORDS, MAX_RECORDS + 1] {
        let value = serde_json::json!({"status":"success","Licenses":vec![license.clone();count]});
        assert_eq!(inspect(&value).is_ok(), count == MAX_RECORDS);
        let value = serde_json::json!({"status":"success","Licenses":[
            {"EntityTypeID":1,"Entities":vec![entity.clone();count/2]},
            {"EntityTypeID":2,"Entities":vec![entity.clone();count-count/2]}
        ]});
        assert_eq!(inspect(&value).is_ok(), count == MAX_RECORDS);
    }
}

/// Verifies the actual wire request has only the encoded token body and no credential headers.
#[tokio::test]
async fn request_contract_uses_post_form_only() {
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
        let header_end = loop {
            let count = stream.read(&mut buffer).await.expect("request");
            assert_ne!(count, 0);
            bytes.extend_from_slice(&buffer[..count]);
            assert!(bytes.len() <= 8192);
            if let Some(position) = bytes.windows(4).position(|bytes| bytes == b"\r\n\r\n") {
                break position + 4;
            }
        };
        let headers = std::str::from_utf8(&bytes[..header_end])
            .expect("ASCII headers")
            .to_ascii_lowercase();
        assert!(headers.starts_with("post /metadata http/1.1\r\n"));
        assert!(headers.contains("\r\ncontent-type: application/x-www-form-urlencoded\r\n"));
        assert!(headers.contains("\r\naccept: application/json\r\n"));
        assert!(headers.contains("\r\naccept-encoding: identity\r\n"));
        assert!(!headers.contains("authorization:"));
        assert!(!headers.contains("cookie:"));
        assert!(!headers.contains("synthetic-secret"));
        let length: usize = headers
            .lines()
            .find_map(|line| line.strip_prefix("content-length: "))
            .expect("body length")
            .parse()
            .expect("numeric length");
        assert!(length <= 8192 - header_end);
        while bytes.len() < header_end + length {
            let count = stream.read(&mut buffer).await.expect("form bytes");
            assert_ne!(count, 0);
            bytes.extend_from_slice(&buffer[..count]);
            assert!(bytes.len() <= 8192);
        }
        assert_eq!(&bytes[header_end..], b"token=synthetic-secret%2B%26%3D");
        stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}")
            .await
            .expect("response");
    });
    timeout(Duration::from_secs(5), async {
        let cookie = BrowserSessionCookie::new(b"synthetic-secret+&=".to_vec()).expect("cookie");
        let client = client().expect("client");
        let mut attempts = 0;
        metadata(&client, &cookie, &url, true, &mut attempts)
            .await
            .expect("synthetic POST");
        assert_eq!(attempts, 1);
        worker.await.expect("wire assertions");
    })
    .await
    .expect("bounded mock");
}

/// Stops after the first response on redirects, denial, throttling, and malformed bodies.
#[tokio::test]
async fn transport_is_single_attempt_and_redacted() {
    let client = client().expect("synthetic client");
    let cookie = BrowserSessionCookie::new(b"synthetic-private-cookie".to_vec()).expect("cookie");
    for (status, extra, content) in [
        (307, "Location: /must-not-follow\r\n", b"{}".to_vec()),
        (401, "", b"{}".to_vec()),
        (403, "", b"{}".to_vec()),
        (429, "Retry-After: 0\r\n", b"{}".to_vec()),
        (503, "", b"{}".to_vec()),
        (200, "Content-Encoding: gzip\r\n", b"{}".to_vec()),
        (200, "", b"private-malformed-body".to_vec()),
        (200, "", vec![b' '; MAX_BYTES + 1]),
    ] {
        let headers = format!("Content-Type: application/json\r\n{extra}");
        let (url, worker) = mock_response(status, &headers, content).await;
        let mut attempts = 0;
        let error = metadata(&client, &cookie, &url, true, &mut attempts)
            .await
            .expect_err("failure");
        assert_eq!(attempts, 1);
        let diagnostic = format!("{error} {error:?}");
        assert!(!diagnostic.contains("private"));
        assert!(!diagnostic.contains(&url));
        assert!(matches!(
            metadata(&client, &cookie, &url, true, &mut attempts).await,
            Err(ProbeError::RequestLimit)
        ));
        worker.await.expect("no retry");
    }
}

/// Accepts the exact response cap and rejects a different media type.
#[tokio::test]
async fn response_bounds_and_media_are_checked() {
    let client = client().expect("synthetic client");
    let cookie = BrowserSessionCookie::new(b"synthetic-cookie".to_vec()).expect("cookie");
    let mut content = b"{}".to_vec();
    content.resize(MAX_BYTES, b' ');
    let (url, worker) = mock_response(200, "Content-Type: application/json\r\n", content).await;
    let mut attempts = 0;
    assert!(matches!(
        metadata(&client, &cookie, &url, false, &mut attempts).await,
        Err(ProbeError::Target)
    ));
    assert_eq!(attempts, 0);
    metadata(&client, &cookie, &url, true, &mut attempts)
        .await
        .expect("exact cap");
    assert_eq!(attempts, 1);
    worker.await.expect("mock");
    let (url, worker) = mock_response(200, "Content-Type: text/html\r\n", b"{}".to_vec()).await;
    assert!(matches!(
        metadata(&client, &cookie, &url, true, &mut 0).await,
        Err(ProbeError::ContentType)
    ));
    worker.await.expect("mock");
}

/// Exercises the actual fixed request deadline against a server that never sends headers.
#[tokio::test]
async fn request_timeout_stops_without_retry() {
    use tokio::{
        io::AsyncReadExt,
        net::TcpListener,
        sync::oneshot,
        time::{advance, pause, resume},
    };
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("loopback");
    let url = format!(
        "http://{}/metadata",
        listener.local_addr().expect("address")
    );
    let (accepted, ready) = oneshot::channel();
    let worker = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("request");
        let mut bytes = vec![0; 8192];
        assert!(stream.read(&mut bytes).await.expect("request bytes") > 0);
        accepted.send(()).expect("signal accepted request");
        let mut remaining = Vec::new();
        if let Err(error) = stream.read_to_end(&mut remaining).await {
            assert!(matches!(
                error.kind(),
                std::io::ErrorKind::ConnectionReset | std::io::ErrorKind::ConnectionAborted
            ));
        }
        assert!(
            timeout(Duration::from_millis(100), listener.accept())
                .await
                .is_err(),
            "timeout must not retry"
        );
    });
    let request = tokio::spawn(async move {
        let cookie = BrowserSessionCookie::new(b"synthetic-cookie".to_vec()).expect("cookie");
        let mut attempts = 0;
        let result = metadata(
            &client().expect("client"),
            &cookie,
            &url,
            true,
            &mut attempts,
        )
        .await;
        (result, attempts)
    });
    ready.await.expect("request reached local server");
    pause();
    advance(Duration::from_secs(16)).await;
    let (result, attempts) = request.await.expect("request task");
    assert!(matches!(result, Err(ProbeError::Transport(error)) if error.is_timeout()));
    assert_eq!(attempts, 1);
    worker.await.expect("no retry");
    resume();
}
