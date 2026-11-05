//! Ignored, explicitly approved library metadata inspection; never a product read.
//!
//! The live check reads the fixed protected entry without saving or deleting it.
//! It makes one exchange and at most two metadata GET attempts, without retries.
//! Ordinary tests below use only synthetic data and loopback HTTP.

use super::*;
use crate::http::{SessionHeaders, direct_client_builder};
use reqwest::{
    Client, Response,
    header::{ACCEPT, ACCEPT_ENCODING, CONTENT_ENCODING, CONTENT_TYPE},
};
use serde_json::Value;
use std::collections::BTreeSet;
use tokio::time::timeout;
use zeroize::Zeroizing;

/// Requires a non-secret acknowledgment in addition to selecting the ignored test.
const APPROVAL: &str = "issue-32-metadata-three-attempts";
/// Separates the newly approved contract checks from the consumed schema-only scope.
const CONTRACT_APPROVAL: &str = "issue-32-metadata-contract-three-attempts";
/// Selects only the two first-party metadata routes approved for this inspection.
const METADATA_URLS: [&str; 2] = [
    "https://api.dndbeyond.com/library/sources?platform=Web",
    "https://api.dndbeyond.com/library/sources/me",
];
/// Bounds each uncompressed metadata response, including unknown fields.
const MAX_BYTES: usize = 1024 * 1024;
/// Bounds structural inspection without claiming an upstream pagination contract.
const MAX_RECORDS: usize = 1_000;

/// Retains typed causes internally while displaying only fixed safe classifications.
enum ProbeError {
    ApprovalRequired,
    IsolatedBuild,
    Session(SessionError),
    Service(service::EstablishmentError),
    Transport(reqwest::Error),
    Status(u16),
    Encoding,
    ContentType,
    BodyLimit,
    Json(serde_json::Error),
    Shape,
    RecordLimit,
    PageLimit,
    Incomplete,
    SourcePath,
    SourceConflict,
    OwnershipMismatch,
    RequestLimit,
    Expired,
    TimedOut,
}

impl fmt::Display for ProbeError {
    /// Never formats headers, URLs, response values, or raw dependency errors.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ApprovalRequired => formatter.write_str("explicit probe approval is required"),
            Self::IsolatedBuild => formatter.write_str("probe requires the fixed product entry"),
            Self::Session(error) => fmt::Display::fmt(error, formatter),
            Self::Service(error) => fmt::Display::fmt(error, formatter),
            Self::Transport(error) if error.is_timeout() => {
                formatter.write_str("metadata request timed out")
            }
            Self::Transport(_) => formatter.write_str("metadata transport failed"),
            Self::Status(status) => write!(formatter, "HTTP {status}; stopped without retry"),
            Self::Encoding => formatter.write_str("encoded response rejected"),
            Self::ContentType => formatter.write_str("non-JSON response rejected"),
            Self::BodyLimit => formatter.write_str("response exceeded 1 MiB"),
            Self::Json(error) => write!(formatter, "invalid JSON: {:?}", error.classify()),
            Self::Shape => formatter.write_str("unexpected metadata shape"),
            Self::RecordLimit => formatter.write_str("inspection record limit exceeded"),
            Self::PageLimit => formatter.write_str("metadata exceeds the existing per-page limit"),
            Self::Incomplete => formatter.write_str("totalCount does not match returned records"),
            Self::SourcePath => formatter.write_str("a source has no valid canonical book path"),
            Self::SourceConflict => formatter.write_str("source identifiers conflict"),
            Self::OwnershipMismatch => formatter.write_str("ownership has an unmatched source"),
            Self::RequestLimit => formatter.write_str("three-attempt request budget exhausted"),
            Self::Expired => formatter.write_str("bearer expired; no exchange retry"),
            Self::TimedOut => formatter.write_str("complete probe deadline reached"),
        }
    }
}

impl fmt::Debug for ProbeError {
    /// Keeps Rust test-runner failures as safe as ordinary diagnostics.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, formatter)
    }
}

impl std::error::Error for ProbeError {}

/// Rejects unapproved or isolated runs before opening protected storage.
fn check_approval(value: Option<&str>, expected: &str, isolated: bool) -> Result<(), ProbeError> {
    if value != Some(expected) {
        return Err(ProbeError::ApprovalRequired);
    }
    if isolated {
        return Err(ProbeError::IsolatedBuild);
    }
    Ok(())
}

/// Builds direct transport with the same no-redirect and no-retry policy as core.
fn client() -> Result<Client, ProbeError> {
    direct_client_builder()
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(ProbeError::Transport)
}

/// Reads at most the byte cap plus one byte; compressed and non-JSON bodies fail.
async fn body(mut response: Response) -> Result<Value, ProbeError> {
    if response.status() != reqwest::StatusCode::OK {
        return Err(ProbeError::Status(response.status().as_u16()));
    }
    if response
        .headers()
        .get_all(CONTENT_ENCODING)
        .iter()
        .any(|value| !value.as_bytes().eq_ignore_ascii_case(b"identity"))
    {
        return Err(ProbeError::Encoding);
    }
    let json_type = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| {
            value
                .split(';')
                .next()
                .is_some_and(|mime| mime.trim().eq_ignore_ascii_case("application/json"))
        });
    if !json_type {
        return Err(ProbeError::ContentType);
    }
    let mut bytes = Zeroizing::new(Vec::new());
    while let Some(chunk) = response.chunk().await.map_err(ProbeError::Transport)? {
        let remaining = MAX_BYTES + 1 - bytes.len();
        bytes.extend_from_slice(&chunk[..chunk.len().min(remaining)]);
        if bytes.len() > MAX_BYTES {
            return Err(ProbeError::BodyLimit);
        }
    }
    let value = serde_json::from_slice(&bytes).map_err(ProbeError::Json)?;
    eprintln!("metadata_response_bytes={}", bytes.len());
    Ok(value)
}

/// Counts an attempt before dispatch; an error never retries or follows another URL.
async fn metadata(
    client: &Client,
    headers: &SessionHeaders,
    url: &str,
    attempts: &mut usize,
) -> Result<Value, ProbeError> {
    if *attempts >= 3 {
        return Err(ProbeError::RequestLimit);
    }
    *attempts += 1;
    let request = headers.apply(
        client
            .get(url)
            .header(ACCEPT, "application/json")
            .header(ACCEPT_ENCODING, "identity"),
    );
    body(request.send().await.map_err(ProbeError::Transport)?).await
}

/// Names JSON types only; scalar values never enter the structural output.
fn kind(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

/// Allows fixed schema names only, not arbitrary response keys or map identifiers.
fn known_field(name: &str) -> bool {
    matches!(
        name,
        "items"
            | "sources"
            | "data"
            | "pagination"
            | "next"
            | "continuation"
            | "page"
            | "pageSize"
            | "pageCount"
            | "total"
            | "totalCount"
            | "hasMore"
            | "nextPage"
            | "nextCursor"
            | "skip"
            | "take"
            | "count"
            | "sourceId"
            | "id"
            | "name"
            | "title"
            | "relativePath"
            | "relativeSlugPath"
            | "slug"
            | "url"
            | "sourceUrl"
            | "sourceURL"
            | "marketplaceUrl"
            | "isOwned"
            | "isSharedWithMe"
            | "isFree"
            | "ownership"
            | "type"
            | "contextId"
            | "campaignId"
            | "sharingId"
            | "sharingContextId"
            | "ownerId"
            | "isActive"
            | "status"
            | "error"
            | "message"
            | "ruleset"
            | "publisher"
            | "label"
            | "isThirdParty"
    )
}

/// Collects bounded schema paths while replacing every unknown key with a marker.
fn schema(value: &Value, path: &str, depth: usize, output: &mut BTreeSet<String>) {
    output.insert(format!("{path}:{}", kind(value)));
    if depth == 0 {
        return;
    }
    match value {
        Value::Object(fields) => {
            for (name, value) in fields {
                if known_field(name) {
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

/// Accepts only collection envelopes for structural inspection, not production use.
fn records(value: &Value) -> Result<&[Value], ProbeError> {
    let entries = value.as_array().or_else(|| {
        ["items", "sources", "data"]
            .into_iter()
            .find_map(|field| value.get(field).and_then(Value::as_array))
    });
    let entries = entries.ok_or(ProbeError::Shape)?;
    if entries.len() > MAX_RECORDS {
        return Err(ProbeError::RecordLimit);
    }
    if !entries.iter().all(Value::is_object) {
        return Err(ProbeError::Shape);
    }
    Ok(entries)
}

/// Returns only fixed schema descriptions and presence categories, never record values.
fn inspect(value: &Value, ownership: bool) -> Result<String, ProbeError> {
    let mut fields = BTreeSet::new();
    schema(value, "root", 5, &mut fields);
    let mut output = fields.into_iter().collect::<Vec<_>>().join("\n");
    let mut owned = false;
    let mut shared = false;
    let mut unknown = false;
    for record in records(value)? {
        if !record
            .get("sourceId")
            .is_some_and(|id| id.is_number() || id.is_string())
        {
            return Err(ProbeError::Shape);
        }
        if ownership {
            let observations = record
                .get("ownership")
                .and_then(Value::as_array)
                .ok_or(ProbeError::Shape)?;
            if observations.len() > MAX_RECORDS {
                return Err(ProbeError::RecordLimit);
            }
            for observation in observations {
                match observation.get("type").and_then(Value::as_str) {
                    Some("owned") => owned = true,
                    Some("shared") => shared = true,
                    Some(_) => unknown = true,
                    None => return Err(ProbeError::Shape),
                }
            }
        } else if !record.get("name").is_some_and(Value::is_string) {
            return Err(ProbeError::Shape);
        }
    }
    if ownership {
        output.push_str(&format!(
            "\nownership_types: owned_present={owned}, shared_present={shared}, unknown_present={unknown}"
        ));
    }
    Ok(output)
}

/// Checks a complete collection without printing either total or record values.
fn complete_collection(value: &Value) -> Result<&[Value], ProbeError> {
    let entries = records(value)?;
    let complete =
        value.get("totalCount").and_then(Value::as_u64) == u64::try_from(entries.len()).ok();
    eprintln!("total_count_matches={complete}");
    if !complete {
        return Err(ProbeError::Incomplete);
    }
    Ok(entries)
}

/// Checks the first-party path-building rule without displaying the constructed URL.
fn valid_source_path(record: &Value) -> bool {
    let path = match record.get("relativeSlugPath").and_then(Value::as_str) {
        Some(path) if !path.is_empty() => path.to_owned(),
        _ => match record.get("slug").and_then(Value::as_str) {
            Some(slug) if !slug.is_empty() => format!("sources/dnd/{slug}"),
            _ => return false,
        },
    };
    path.starts_with("sources/")
        && path.split('/').all(|segment| {
            !segment.is_empty()
                && segment
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        })
}

/// Verifies metadata completeness, raw bounds, unique identifiers, and source paths.
fn check_sources(value: &Value) -> Result<(), ProbeError> {
    let entries = complete_collection(value)?;
    let within_page_limit = entries.len() <= crate::library::MAX_ENTRIES_PER_PAGE;
    let within_total_limit = entries.len() <= crate::library::MAX_ENTRIES;
    eprintln!("within_existing_page_limit={within_page_limit}");
    eprintln!("within_existing_total_limit={within_total_limit}");
    let mut ids = std::collections::HashSet::new();
    let unique = entries.iter().all(|record| {
        record
            .get("sourceId")
            .and_then(Value::as_u64)
            .is_some_and(|id| id > 0 && ids.insert(id))
    });
    let valid_paths = entries.iter().all(valid_source_path);
    eprintln!("unique_numeric_source_ids={unique}");
    eprintln!("all_sources_match_book_path_rule={valid_paths}");
    if !within_page_limit {
        return Err(ProbeError::PageLimit);
    }
    if !within_total_limit {
        return Err(ProbeError::RecordLimit);
    }
    if !unique {
        return Err(ProbeError::SourceConflict);
    }
    if !valid_paths {
        return Err(ProbeError::SourcePath);
    }
    Ok(())
}

/// Verifies complete ownership records against metadata without printing identifiers.
fn check_ownership(value: &Value, sources: &Value) -> Result<(), ProbeError> {
    let entries = complete_collection(value)?;
    let within_page_limit = entries.len() <= crate::library::MAX_ENTRIES_PER_PAGE;
    eprintln!("within_existing_page_limit={within_page_limit}");
    if !within_page_limit {
        return Err(ProbeError::PageLimit);
    }
    let ids = records(sources)?
        .iter()
        .filter_map(|record| record.get("sourceId").and_then(Value::as_u64))
        .collect::<std::collections::HashSet<_>>();
    let matched = entries.iter().all(|record| {
        record
            .get("sourceId")
            .and_then(Value::as_u64)
            .is_some_and(|id| ids.contains(&id))
    });
    eprintln!("all_ownership_sources_matched={matched}");
    if !matched {
        return Err(ProbeError::OwnershipMismatch);
    }
    let summary = inspect(value, true)?;
    if let Some(presence) = summary.lines().last() {
        eprintln!("{presence}");
    }
    Ok(())
}

/// Uses the approved fixed entry once, then performs no more than the two fixed GETs.
async fn live_check(contract: bool) -> Result<(), ProbeError> {
    let session = Session::default();
    let deadline = Instant::now() + Duration::from_secs(60);
    let cookie = session
        .load_cookie(deadline)
        .await
        .map_err(ProbeError::Session)?
        .ok_or_else(|| ProbeError::Session(SessionError::new(SessionErrorKind::Absent)))?;
    let mut attempts = 1;
    let bearer = session
        .exchange(&cookie, deadline)
        .await
        .map_err(ProbeError::Session)?;
    drop(cookie);
    eprintln!("authentication=accepted; attempts=1");
    let client = client()?;
    let mut sources = None;
    for (index, url) in METADATA_URLS.iter().enumerate() {
        if !bearer.usable(SystemTime::now()) {
            return Err(ProbeError::Expired);
        }
        let headers = bearer.headers().map_err(ProbeError::Service)?;
        let value = metadata(&client, &headers, url, &mut attempts).await?;
        if !bearer.usable(SystemTime::now()) {
            return Err(ProbeError::Expired);
        }
        eprintln!("metadata_step={}; HTTP=200", index + 1);
        if contract {
            if index == 0 {
                check_sources(&value)?;
                sources = Some(value);
            } else {
                check_ownership(&value, sources.as_ref().ok_or(ProbeError::Shape)?)?;
            }
        } else {
            eprintln!("{}", inspect(&value, index == 1)?);
        }
    }
    eprintln!("probe=completed; attempts={attempts}; stored_session=unchanged");
    Ok(())
}

/// Runs only after separate approval and explicit selection; ordinary Cargo tests skip it.
#[tokio::test]
#[ignore = "requires separate issue #32 approval for protected storage and three HTTP attempts"]
async fn approved_live_library_metadata() -> Result<(), ProbeError> {
    let approval = std::env::var("BEYOND_GATE_LIBRARY_PROBE_APPROVAL").ok();
    check_approval(
        approval.as_deref(),
        APPROVAL,
        cfg!(feature = "e2e-isolation"),
    )?;
    timeout(Duration::from_secs(60), live_check(false))
        .await
        .map_err(|_| ProbeError::TimedOut)?
}

/// Runs only the separately approved follow-up; any failed check stops remaining requests.
#[tokio::test]
#[ignore = "requires separate issue #32 approval for the follow-up contract checks"]
async fn approved_live_library_contract_checks() -> Result<(), ProbeError> {
    let approval = std::env::var("BEYOND_GATE_LIBRARY_PROBE_APPROVAL").ok();
    check_approval(
        approval.as_deref(),
        CONTRACT_APPROVAL,
        cfg!(feature = "e2e-isolation"),
    )?;
    timeout(Duration::from_secs(60), live_check(true))
        .await
        .map_err(|_| ProbeError::TimedOut)?
}

/// Ensures acknowledgment checks cannot admit missing, stale, or isolated approval.
#[test]
fn approval_is_explicit_and_product_scoped() {
    assert!(check_approval(None, APPROVAL, false).is_err());
    assert!(check_approval(Some("1"), APPROVAL, false).is_err());
    assert!(check_approval(Some(APPROVAL), APPROVAL, true).is_err());
    assert!(check_approval(Some(APPROVAL), APPROVAL, false).is_ok());
    assert!(check_approval(Some(APPROVAL), CONTRACT_APPROVAL, false).is_err());
    assert!(check_approval(Some(CONTRACT_APPROVAL), CONTRACT_APPROVAL, false).is_ok());
}

/// Checks count equality, source joins, null paths, duplicates, and the raw page cap.
#[test]
fn contract_checks_reject_incomplete_or_unusable_metadata() {
    let valid = serde_json::json!({
        "items": [{"sourceId": 1, "name": "Invented", "slug": "invented"}],
        "totalCount": 1
    });
    check_sources(&valid).expect("invented complete source");
    let owned = serde_json::json!({
        "items": [{"sourceId": 1, "ownership": [{"type": "owned"}]}],
        "totalCount": 1
    });
    check_ownership(&owned, &valid).expect("invented matching ownership");
    let mut incomplete = valid.clone();
    incomplete["totalCount"] = serde_json::json!(2);
    assert!(matches!(
        check_sources(&incomplete),
        Err(ProbeError::Incomplete)
    ));
    let mut missing_path = valid.clone();
    missing_path["items"][0]["slug"] = Value::Null;
    assert!(matches!(
        check_sources(&missing_path),
        Err(ProbeError::SourcePath)
    ));
    let mut unmatched = owned;
    unmatched["items"][0]["sourceId"] = serde_json::json!(2);
    assert!(matches!(
        check_ownership(&unmatched, &valid),
        Err(ProbeError::OwnershipMismatch)
    ));
    let mut duplicate = valid.clone();
    duplicate["items"]
        .as_array_mut()
        .expect("synthetic array")
        .push(valid["items"][0].clone());
    duplicate["totalCount"] = serde_json::json!(2);
    assert!(matches!(
        check_sources(&duplicate),
        Err(ProbeError::SourceConflict)
    ));
    let entries = (1..=crate::library::MAX_ENTRIES_PER_PAGE + 1)
        .map(|id| serde_json::json!({"sourceId": id, "slug": format!("invented-{id}")}))
        .collect::<Vec<_>>();
    let over_page = serde_json::json!({"totalCount": entries.len(), "items": entries});
    assert!(matches!(
        check_sources(&over_page),
        Err(ProbeError::PageLimit)
    ));
    for path in [
        "sources/../private",
        "//foreign.invalid",
        "sources/test?private",
    ] {
        assert!(!valid_source_path(
            &serde_json::json!({"relativeSlugPath": path})
        ));
    }
}

/// Proves neither scalar data nor arbitrary map keys can enter the safe summary.
#[test]
fn structural_summary_omits_private_values_and_unknown_keys() {
    let value = serde_json::json!({
        "items": [{
            "sourceId": "private-source",
            "name": "private-title",
            "ownership": [{
                "type": "shared",
                "campaignId": "private-campaign",
                "private-map-key": "private-map-value"
            }]
        }],
        "pagination": {"nextCursor": "private-cursor"}
    });
    let output = inspect(&value, true).expect("synthetic ownership shape");
    assert!(!output.contains("private"));
    assert!(output.contains("root.items[].ownership[].campaignId:string"));
    assert!(output.contains("shared_present=true"));
    assert!(output.contains("[unknown_field]"));
}

/// Rejects malformed collections and evidence instead of reporting empty success.
#[test]
fn invalid_shapes_and_excess_records_fail() {
    for value in [
        serde_json::json!({"unexpected": []}),
        serde_json::json!({"items": [null]}),
        serde_json::json!({"items": [{"sourceId": 1}]}),
        serde_json::json!({"items": [{"sourceId": 1, "ownership": [{"type": null}]}]}),
    ] {
        assert!(inspect(&value, true).is_err());
    }
    let value = Value::Array(vec![serde_json::json!({}); MAX_RECORDS + 1]);
    assert!(matches!(records(&value), Err(ProbeError::RecordLimit)));
}

/// Serves exactly one invented response and records whether transport sends another attempt.
async fn mock_response(
    status: u16,
    headers: &str,
    body: Vec<u8>,
) -> (String, tokio::task::JoinHandle<()>) {
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("loopback bind");
    let url = format!(
        "http://{}/metadata",
        listener.local_addr().expect("address")
    );
    let head = format!(
        "HTTP/1.1 {status} Synthetic\r\nContent-Length: {}\r\n{headers}Connection: close\r\n\r\n",
        body.len()
    );
    let worker = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("one request");
        let mut request = vec![0; 8192];
        let count = stream.read(&mut request).await.expect("request bytes");
        assert!(count > 0);
        stream.write_all(head.as_bytes()).await.expect("headers");
        // A byte-cap failure may close the connection before the complete body is sent.
        if let Err(error) = stream.write_all(&body).await {
            assert!(matches!(
                error.kind(),
                std::io::ErrorKind::ConnectionReset
                    | std::io::ErrorKind::ConnectionAborted
                    | std::io::ErrorKind::BrokenPipe
            ));
        }
        drop(stream);
        assert!(
            timeout(Duration::from_millis(100), listener.accept())
                .await
                .is_err(),
            "transport must not retry"
        );
    });
    (url, worker)
}

/// Checks redirects, denial, throttling, encoding, bad JSON, and body limits locally.
#[tokio::test]
async fn transport_stops_without_retry_or_redirect() {
    let client = client().expect("synthetic client");
    let headers = SessionHeaders::bearer("synthetic-secret").expect("synthetic header");
    let cases = [
        (201, "", b"{}".to_vec()),
        (307, "Location: /must-not-follow\r\n", b"{}".to_vec()),
        (401, "", b"{}".to_vec()),
        (403, "", b"{}".to_vec()),
        (429, "Retry-After: 0\r\n", b"{}".to_vec()),
        (503, "", b"{}".to_vec()),
        (200, "Content-Encoding: gzip\r\n", b"{}".to_vec()),
        (200, "", b"private-invalid-json".to_vec()),
        (200, "", vec![b' '; MAX_BYTES + 1]),
    ];
    for (status, extra, content) in cases {
        let response_headers = format!("Content-Type: application/json\r\n{extra}");
        let (url, worker) = mock_response(status, &response_headers, content).await;
        let mut attempts = 1;
        let error = metadata(&client, &headers, &url, &mut attempts)
            .await
            .expect_err("synthetic failure");
        assert_eq!(attempts, 2);
        let diagnostic = format!("{error} {error:?}");
        assert!(!diagnostic.contains("private"));
        assert!(!diagnostic.contains("synthetic-secret"));
        assert!(!diagnostic.contains(&url));
        worker.await.expect("mock completed");
    }
}

/// Verifies exact byte-cap success and refusal before a fourth network attempt.
#[tokio::test]
async fn byte_and_attempt_boundaries_are_enforced() {
    let client = client().expect("synthetic client");
    let headers = SessionHeaders::bearer("synthetic-secret").expect("synthetic header");
    let mut content = b"[]".to_vec();
    content.resize(MAX_BYTES, b' ');
    let (url, worker) = mock_response(200, "Content-Type: application/json\r\n", content).await;
    let mut attempts = 2;
    let value = metadata(&client, &headers, &url, &mut attempts)
        .await
        .expect("exact byte cap succeeds");
    assert!(value.as_array().is_some_and(Vec::is_empty));
    assert_eq!(attempts, 3);
    assert!(matches!(
        metadata(&client, &headers, &url, &mut attempts).await,
        Err(ProbeError::RequestLimit)
    ));
    worker.await.expect("no fourth request");
}
