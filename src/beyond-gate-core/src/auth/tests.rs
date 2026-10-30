//! Synthetic credential-store tests; no native store or account is accessed.

use std::{error::Error, io};

use keyring_core::{Error as NativeError, mock};

use super::*;

/// Rejects unsafe test identities without opening any native credential service.
#[cfg(feature = "e2e-isolation")]
#[test]
fn isolated_context_and_entries_do_not_select_product_credentials() {
    for value in ["", "local-session", &"A".repeat(32), &"f".repeat(33)] {
        assert_eq!(
            validate_e2e_context(value)
                .expect_err("invalid context")
                .kind(),
            AuthErrorKind::TestContextRequired
        );
    }
    let context = "0123456789abcdef0123456789abcdef";
    validate_e2e_context(context).expect("valid context");
    let backend = mock::Store::new().expect("synthetic backend");
    let product = SessionStore::from_store(backend.as_ref(), None).expect("product");
    let isolated = SessionStore::from_identity(backend.as_ref(), None, "beyond-gate-e2e", context)
        .expect("isolated");
    let other = SessionStore::from_identity(
        backend.as_ref(),
        None,
        "beyond-gate-e2e",
        "ffffffffffffffffffffffffffffffff",
    )
    .expect("other run");
    product
        .save(&SessionSecret::new(b"synthetic-product".to_vec()).expect("secret"))
        .expect("save");
    isolated
        .save(&SessionSecret::new(b"synthetic-test".to_vec()).expect("secret"))
        .expect("save");
    isolated.logout().expect("isolated deletion");
    assert_eq!(
        product.status().expect("product unchanged"),
        SessionStatus::StoredUnverified
    );
    assert_eq!(other.status().expect("other absent"), SessionStatus::Absent);
}

/// Builds the production entry identity in an isolated in-memory test store.
///
/// # Panics
///
/// Panics if the synthetic store or its fixed entry cannot be built.
fn session_store() -> SessionStore {
    let store = mock::Store::new().expect("create synthetic store");
    SessionStore::from_store(store.as_ref(), None).expect("build fixed test entry")
}

/// Injects one typed backend failure without changing the saved synthetic value.
///
/// # Panics
///
/// Panics if the test entry is not backed by a mock credential.
fn fail_next(store: &SessionStore, error: NativeError) {
    store
        .entry
        .as_any()
        .downcast_ref::<mock::Cred>()
        .expect("test store must be synthetic")
        .set_error(error);
}

/// Checks shared entry ownership, local lifecycle, and isolation from other entries.
#[test]
fn fixed_entry_lifecycle_preserves_other_credentials() {
    let backend = mock::Store::new().expect("create mock");
    let first = SessionStore::from_store(backend.as_ref(), None).expect("first adapter");
    let second = SessionStore::from_store(backend.as_ref(), None).expect("second adapter");
    let unrelated = backend
        .build("another-app", ACCOUNT, None)
        .expect("other app");
    let other_account = backend
        .build(SERVICE, "another-session", None)
        .expect("other account");
    unrelated
        .set_secret(b"synthetic-other-app")
        .expect("save other app");
    other_account
        .set_secret(b"synthetic-other-account")
        .expect("save other account");

    assert_eq!(
        first.entry.get_specifiers(),
        Some((SERVICE.to_owned(), ACCOUNT.to_owned()))
    );
    assert_eq!(first.status().expect("missing"), SessionStatus::Absent);
    first.logout().expect("logout while absent");
    let secret = SessionSecret::new(b"synthetic-session-only".to_vec()).expect("secret");
    first.save(&secret).expect("save session");
    assert_eq!(
        second.status().expect("shared presence"),
        SessionStatus::StoredUnverified
    );
    assert_eq!(
        second.entry.get_secret().expect("unchanged value"),
        secret.0.as_slice()
    );

    second.logout().expect("remove session");
    assert_eq!(
        first.status().expect("shared removal"),
        SessionStatus::Absent
    );
    first.logout().expect("repeat logout");
    assert_eq!(
        unrelated.get_secret().expect("other app remains"),
        b"synthetic-other-app"
    );
    assert_eq!(
        other_account.get_secret().expect("other account remains"),
        b"synthetic-other-account"
    );
}

/// Checks exact local size bounds without inventing a text or upstream schema.
#[test]
fn opaque_secret_size_bounds_are_exact() {
    for bytes in [vec![], vec![b'x'; MAX_SECRET_BYTES + 1]] {
        assert_eq!(
            SessionSecret::new(bytes).expect_err("invalid size").kind(),
            AuthErrorKind::InvalidSecret
        );
    }
    let store = session_store();
    for bytes in [vec![0], vec![0xff; MAX_SECRET_BYTES]] {
        let secret = SessionSecret::new(bytes).expect("bounded opaque bytes");
        store.save(&secret).expect("mock accepts bounded secret");
        assert_eq!(
            store.status().expect("present"),
            SessionStatus::StoredUnverified
        );
    }
}

/// Ensures status does not remove invalid data and logout can still remove it.
#[test]
fn invalid_stored_data_is_an_error_not_an_absent_session() {
    let store = session_store();
    for bytes in [vec![], vec![b'x'; MAX_SECRET_BYTES + 1]] {
        store
            .entry
            .set_secret(&bytes)
            .expect("seed invalid synthetic data");
        assert_eq!(
            store.status().expect_err("invalid stored value").kind(),
            AuthErrorKind::InvalidSecret
        );
        assert_eq!(store.entry.get_secret().expect("value unchanged"), bytes);
        store
            .logout()
            .expect("remove invalid entry without reading it");
        assert_eq!(store.status().expect("removed"), SessionStatus::Absent);
    }
}

/// Provides representative failures, including native strings that must stay hidden.
fn native_failures() -> Vec<(NativeError, AuthErrorKind)> {
    vec![
        (
            NativeError::NoStorageAccess(Box::new(io::Error::other("synthetic-secret-access"))),
            AuthErrorKind::StorageAccess,
        ),
        (
            NativeError::NoDefaultStore,
            AuthErrorKind::StorageUnavailable,
        ),
        (
            NativeError::NotSupportedByStore("synthetic-secret-backend".into()),
            AuthErrorKind::StorageUnavailable,
        ),
        (
            NativeError::PlatformFailure(Box::new(io::Error::other("synthetic-secret-platform"))),
            AuthErrorKind::StorageFailure,
        ),
        (
            NativeError::BadStoreFormat("synthetic-secret-format".into()),
            AuthErrorKind::StorageFailure,
        ),
        (
            NativeError::BadEncoding(b"synthetic-secret-encoding".to_vec()),
            AuthErrorKind::InvalidSecret,
        ),
        (
            NativeError::BadDataFormat(
                b"synthetic-secret-data".to_vec(),
                Box::new(io::Error::other("synthetic-secret-cause")),
            ),
            AuthErrorKind::InvalidSecret,
        ),
        (
            NativeError::TooLong("synthetic-secret-limit".into(), 1),
            AuthErrorKind::InvalidSecret,
        ),
        (
            NativeError::Invalid(
                "synthetic-secret-field".into(),
                "synthetic-secret-value".into(),
            ),
            AuthErrorKind::InvalidSecret,
        ),
        (
            NativeError::Ambiguous(vec![]),
            AuthErrorKind::AmbiguousEntry,
        ),
    ]
}

/// Checks failures cannot look like absence, successful storage, or successful logout.
#[test]
fn storage_failures_propagate_without_changing_the_session() {
    let store = session_store();
    let secret = SessionSecret::new(b"synthetic-saved-value".to_vec()).expect("secret");
    store.save(&secret).expect("seed session");
    for (native, kind) in native_failures() {
        fail_next(&store, native);
        assert_eq!(store.status().expect_err("status must fail").kind(), kind);
        assert_eq!(
            store.entry.get_secret().expect("value preserved"),
            secret.0.as_slice()
        );
    }
    for (native, kind) in native_failures() {
        fail_next(&store, native);
        assert_eq!(store.logout().expect_err("logout must fail").kind(), kind);
        assert_eq!(
            store.status().expect("value preserved"),
            SessionStatus::StoredUnverified
        );
    }
    for (native, kind) in native_failures() {
        fail_next(&store, native);
        assert_eq!(
            store.save(&secret).expect_err("write must fail").kind(),
            kind
        );
        assert_eq!(
            store.entry.get_secret().expect("value preserved"),
            secret.0.as_slice()
        );
    }
}

/// Checks every formatter and error source stops before secret-bearing native data.
#[test]
fn native_causes_and_secret_debug_are_redacted() {
    let store = session_store();
    let secret = SessionSecret::new(b"synthetic-secret-session".to_vec()).expect("secret");
    store.save(&secret).expect("seed session");
    assert!(!format!("{secret:?} {store:?}").contains("synthetic-secret"));
    for (native, kind) in native_failures() {
        let error = AuthError::storage(native);
        assert_eq!(error.kind(), kind);
        let mut current: Option<&dyn Error> = Some(&error);
        let mut depth = 0;
        while let Some(cause) = current {
            assert!(!format!("{cause} {cause:?}").contains("synthetic-secret"));
            current = cause.source();
            depth += 1;
            assert!(
                depth <= 2,
                "native error chain must stop at the safe wrapper"
            );
        }
        let cause = error
            .source()
            .expect("retained cause")
            .downcast_ref::<StorageCause>()
            .expect("safe typed cause");
        assert_eq!(cause.kind(), kind);
    }
}

/// Checks blocked login has no native cause and cannot claim successful authentication.
#[test]
fn login_has_an_explicit_blocker() {
    let error = login_blocker();
    assert_eq!(error.kind(), AuthErrorKind::LoginBlocked);
    assert!(error.source().is_none());
    assert!(error.to_string().contains("no working session handoff"));
}
