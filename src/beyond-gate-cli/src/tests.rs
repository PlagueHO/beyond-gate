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
            cancel_on_signal(
                std::future::pending::<Result<(), Failure>>(),
                std::future::ready(Ok(())),
            )
            .await
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

/// Gives a ready signal priority without polling a ready operation or using OS signals.
#[test]
fn cancellation_precedes_reads_and_preserves_signal_failures() {
    use std::{cell::Cell, future::ready};
    let polled = Cell::new(false);
    for signal in [
        Ok(()),
        Err(io::Error::from(io::ErrorKind::PermissionDenied)),
    ] {
        let failed = signal.is_err();
        let result = execute_session(cancel_on_signal(
            async {
                polled.set(true);
                Ok(())
            },
            ready(signal),
        ));
        assert!(!polled.get());
        if failed {
            assert!(matches!(result, Err(Failure::Signal(error))
                    if error.kind() == io::ErrorKind::PermissionDenied));
        } else {
            assert!(matches!(result, Err(Failure::Cancelled)));
        }
    }
}

/// Preserves read results and their typed failures through the shared runtime.
#[test]
fn shared_reads_preserve_values_and_typed_errors() {
    use std::future::ready;
    assert_eq!(
        execute_read(|| ready(Ok::<_, ItemError>(17)), Failure::Item).expect("read"),
        17
    );
    assert!(matches!(
        execute_read(
            || ready(Err::<(), _>(ItemError::InvalidMagicInput)),
            Failure::Item
        ),
        Err(Failure::Item(ItemError::InvalidMagicInput))
    ));
}

/// Constructs runtime-dependent read futures in the same context as their execution.
#[test]
fn read_setup_has_a_runtime_before_constructing_its_future() {
    struct RuntimeReads;
    impl ReadOperations for RuntimeReads {
        /// Creates a timer before returning a future, without storage or network access.
        fn get_book_outline(
            &self,
            _session: &Session,
            _id: &str,
        ) -> impl std::future::Future<Output = Result<BookOutline, BookError>> {
            let timer = tokio::time::sleep(std::time::Duration::ZERO);
            async move {
                timer.await;
                Err(BookError::AccessDenied)
            }
        }
    }
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    assert_eq!(
        run(
            ["beyond-gate", "book", "outline", "book"],
            &RuntimeReads,
            &mut stdout,
            &mut stderr,
        ),
        1
    );
    assert!(stdout.is_empty());
    assert_eq!(
        stderr,
        format!("error: {}\n", Failure::Book(BookError::AccessDenied)).as_bytes(),
    );
}

/// Keeps false and zero distinct from omitted values and preserves repeated selectors.
#[test]
fn shared_query_mapping_preserves_typed_filters() {
    let matches = command()
        .try_get_matches_from(
            concat!(
                "beyond-gate monster list --name Invented --type-id 2 --type-id 1 ",
                "--armor-class-min 0 --legendary false --cr-min 1/8 --page 3"
            )
            .split_whitespace(),
        )
        .expect("monster arguments");
    let arguments = matches
        .subcommand_matches("monster")
        .expect("resource")
        .subcommand_matches("list")
        .expect("list");
    let query = monster_query(arguments).expect("query");
    assert_eq!(query.name.as_deref(), Some("Invented"));
    assert_eq!(query.page, 3);
    assert_eq!(query.type_ids, [2, 1]);
    assert_eq!(query.armor_class_min, Some(0));
    assert_eq!(query.legendary, Some(false));
    assert!(query.mythic.is_none());
    assert_eq!(query.cr_min.as_deref(), Some("1/8"));
    let matches = command()
        .try_get_matches_from(
            "beyond-gate magic-item list --type Armor --type Weapon --tag Utility"
                .split_whitespace(),
        )
        .expect("item arguments");
    let arguments = matches
        .subcommand_matches("magic-item")
        .expect("resource")
        .subcommand_matches("list")
        .expect("list");
    let query = magic_item_query(arguments, 1).expect("query");
    assert_eq!(query.types, ["Armor", "Weapon"]);
    assert_eq!(query.tags, ["Utility"]);
    assert!(query.name.is_none());
    assert!(matches!(
        query.attunement,
        item_catalog::query::Attunement::Any
    ));
}
