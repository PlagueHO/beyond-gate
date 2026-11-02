//! Command execution checks with injected local operations, never native storage.

use beyond_gate_core::auth::SessionStatus;

use super::*;

/// Keeps CLI shutdown bounded when a cancelled operation leaves a native worker running.
#[test]
fn session_shutdown_does_not_wait_for_blocking_storage() {
    let (release, wait) = std::sync::mpsc::channel();
    let (returned, result) = std::sync::mpsc::channel();
    let caller = std::thread::spawn(move || {
        let outcome: Result<(), Failure> = execute_session(async move {
            let (started, ready) = tokio::sync::oneshot::channel();
            tokio::task::spawn_blocking(move || {
                started.send(()).expect("worker started");
                wait.recv().expect("release synthetic worker");
            });
            ready.await.expect("worker running");
            Err(Failure::Cancelled)
        });
        returned
            .send(matches!(outcome, Err(Failure::Cancelled)))
            .expect("report operation result");
    });
    let outcome = result.recv_timeout(std::time::Duration::from_secs(2));
    release.send(()).expect("release worker before assertions");
    caller.join().expect("CLI caller");
    assert_eq!(outcome, Ok(true));
}

/// Checks local status messages do not imply usable or expired sessions.
#[test]
fn status_reports_presence_and_unknown_expiry() {
    for (status, prefix) in [
        (SessionStatus::Absent, "absent:"),
        (SessionStatus::StoredUnverified, "stored-unverified:"),
    ] {
        let message = status_message(status);
        assert!(message.starts_with(prefix));
        if status == SessionStatus::StoredUnverified {
            assert!(message.contains("upstream usability is not checked"));
            assert!(message.contains("expiry is unknown"));
        }
    }
}
