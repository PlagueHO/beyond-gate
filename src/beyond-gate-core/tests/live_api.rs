//! Opt-in public and authenticated API integration through the production library reader.
//!
//! Ordinary Cargo tests never run the ignored live test. Credentials come only from
//! explicit process input; this test never selects stored credentials or saves data.

use beyond_gate_core::{
    auth::session::{Session, SessionError, SessionErrorKind},
    library::{LibraryError, discover_library},
};

/// Selects this fixed scope independently of historical diagnostic approvals.
const APPROVAL: &str = "public-and-authenticated-library-five-attempts";
/// Carries only a non-secret scope acknowledgment.
const APPROVAL_ENV: &str = "BEYOND_GATE_LIVE_API_APPROVED";
/// Uses the product's process-only credential input.
const COOKIE_ENV: &str = "BEYOND_GATE_COBALT_SESSION";

/// Rejects missing approval or input before any credential or HTTP operation.
fn check_scope(approval: Option<&str>, has_input: bool) -> Result<(), &'static str> {
    if approval != Some(APPROVAL) {
        return Err("approval_required");
    }
    if !has_input {
        return Err("input_missing");
    }
    if !cfg!(any(target_os = "windows", target_os = "linux")) {
        return Err("unsupported_platform");
    }
    Ok(())
}

/// Selects fixed failure labels without formatting input, responses, or error chains.
fn session_reason(error: &SessionError) -> &'static str {
    match error.kind() {
        SessionErrorKind::Absent => "input_missing",
        SessionErrorKind::InvalidMaterial => "input_invalid",
        SessionErrorKind::Rejected => "session_rejected",
        SessionErrorKind::Expired => "session_expired",
        SessionErrorKind::TimedOut => "timed_out",
        SessionErrorKind::AccessDenied => "access_denied",
        SessionErrorKind::UnsupportedPlatform => "unsupported_platform",
        _ => "private_request_failed",
    }
}

/// Preserves the public/private failure boundary without publishing upstream data.
fn library_reason(error: &LibraryError) -> &'static str {
    match error {
        LibraryError::SessionRead(error) => session_reason(error),
        LibraryError::PageRead(_) => "public_request_failed",
        LibraryError::TimedOut => "timed_out",
        _ => "contract_mismatch",
    }
}

/// Runs the fixed production sequence once; all resource GETs use single-attempt clients.
///
/// The reader makes one public GET, two protected GETs, and at most one exchange
/// before each protected GET: five HTTP attempts maximum, normally four. Its
/// 30-second deadline, 1-MiB GET caps, 64-KiB exchange cap, and collection guards
/// remain unchanged. It never follows pagination, redirects, or denial recovery.
async fn run_live() -> Result<(), &'static str> {
    let approval = std::env::var(APPROVAL_ENV).ok();
    check_scope(approval.as_deref(), true)?;
    let value = std::env::var_os(COOKIE_ENV).ok_or("input_missing")?;
    let session = Session::default().with_environment_value(Some(value));
    session
        .status()
        .await
        .map_err(|error| session_reason(&error))?;
    let snapshot = discover_library(&session)
        .await
        .map_err(|error| library_reason(&error))?;
    if snapshot.books().is_empty() {
        return Err("contract_mismatch");
    }
    Ok(())
}

/// Validates public configuration, exchange, Web metadata, and all ownership contracts.
#[tokio::test]
#[ignore = "requires explicit live approval and a supplied process-only CobaltSession"]
async fn public_and_authenticated_library() -> Result<(), &'static str> {
    let result = run_live().await;
    println!(
        "beyond_gate_live_api={}",
        result.as_ref().map_or_else(|reason| *reason, |_| "passed")
    );
    result
}

/// Confirms historical scopes and absent credentials cannot enable live execution.
#[test]
fn missing_approval_and_input_stop_before_requests() {
    assert_eq!(check_scope(None, true), Err("approval_required"));
    assert_eq!(
        check_scope(Some("issue-32-metadata-three-attempts"), true),
        Err("approval_required")
    );
    assert_eq!(check_scope(Some(APPROVAL), false), Err("input_missing"));
}

/// Checks invalid synthetic process input locally without storage or HTTP activity.
#[tokio::test]
async fn invalid_input_is_rejected_locally() {
    let session = Session::default().with_prompt(b"invented;invalid".to_vec());
    let error = session.status().await.expect_err("invalid synthetic input");
    assert_eq!(session_reason(&error), "input_invalid");
}

/// Keeps contract failures within a fixed label set rather than raw diagnostics.
#[test]
fn library_failure_labels_are_safe() {
    assert_eq!(
        library_reason(&LibraryError::InvalidMetadata),
        "contract_mismatch"
    );
    assert_eq!(
        library_reason(&LibraryError::UnmatchedOwnership),
        "contract_mismatch"
    );
    assert_eq!(library_reason(&LibraryError::TimedOut), "timed_out");
}
