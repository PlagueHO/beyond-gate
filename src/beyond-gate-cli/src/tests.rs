//! Command execution checks with injected local operations, never native storage.

use beyond_gate_core::auth::AuthErrorKind;

use super::*;

/// Checks login stops before either storage operation can run.
#[test]
fn login_does_not_open_storage() {
    let error = execute_auth(
        AuthAction::Login,
        || panic!("login must not inspect storage"),
        || panic!("login must not delete storage"),
    )
    .expect_err("blocked login");
    assert_eq!(error.kind(), AuthErrorKind::LoginBlocked);
}

/// Checks local status messages do not imply usable or expired sessions.
#[test]
fn status_reports_presence_and_unknown_expiry() {
    for (status, prefix) in [
        (SessionStatus::Absent, "absent:"),
        (SessionStatus::StoredUnverified, "stored-unverified:"),
    ] {
        let message = execute_auth(
            AuthAction::Status,
            || Ok(status),
            || panic!("status must not delete storage"),
        )
        .expect("local status");
        assert!(message.starts_with(prefix));
        if status == SessionStatus::StoredUnverified {
            assert!(message.contains("upstream usability is not checked"));
            assert!(message.contains("expiry is unknown"));
        }
    }
}

/// Checks logout does not read a credential and states the revocation limit.
#[test]
fn logout_only_deletes_and_reports_local_scope() {
    let message = execute_auth(
        AuthAction::Logout,
        || panic!("logout must not read the secret"),
        || Ok(()),
    )
    .expect("local deletion");
    assert!(message.contains("removed or already absent"));
    assert!(message.contains("Upstream revocation is not implemented"));
}

/// Checks shared failures cannot be converted into a successful command result.
#[test]
fn operation_errors_are_preserved() {
    let status = execute_auth(
        AuthAction::Status,
        || Err(auth::login_blocker()),
        || panic!("wrong operation"),
    )
    .expect_err("status failure");
    let logout = execute_auth(
        AuthAction::Logout,
        || panic!("wrong operation"),
        || Err(auth::login_blocker()),
    )
    .expect_err("logout failure");
    assert_eq!(status.kind(), AuthErrorKind::LoginBlocked);
    assert_eq!(logout.kind(), AuthErrorKind::LoginBlocked);
}
