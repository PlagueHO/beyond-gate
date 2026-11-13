//! Offline item page, classification, selection, bound, and request-contract checks.

use super::{ItemError, ItemQuery, ItemSelection, MAX_PAGE, list, show, validate_id};
use super::{
    fixture::{self, Scenario},
    reads::{assemble, finish},
};
use serde_json::{Value, json};
use std::{collections::HashSet, sync::Arc, time::Duration};

/// Validates all selectors before any protected session or source is used.
#[tokio::test]
async fn input_is_strict_and_checked_before_session_access() {
    for id in [
        "17",
        "0:17",
        "7:0",
        "07:17",
        "7:017",
        " 7:17",
        "7:17:1",
        "7:18446744073709551616",
    ] {
        assert!(matches!(validate_id(id), Err(ItemError::InvalidInput)));
    }
    for page in [0, MAX_PAGE + 1] {
        assert!(matches!(
            list(&Default::default(), &ItemQuery { page }).await,
            Err(ItemError::InvalidInput)
        ));
    }
    for value in [
        json!({"page":null}),
        json!({"page":0.5}),
        json!({"skip":0}),
        json!({"page":1,"take":100}),
    ] {
        assert!(serde_json::from_value::<ItemQuery>(value).is_err());
    }
    assert_eq!(
        serde_json::from_value::<ItemQuery>(json!({}))
            .expect("default page")
            .page,
        1
    );
    assert!(serde_json::from_value::<ItemSelection>(json!({"id":"7:17"})).is_err());
}

/// Reads full, terminal, empty, and magic-only pages without automatic continuation.
#[tokio::test]
async fn source_paging_is_explicit_and_counts_remain_mixed() {
    let session = fixture::session(Scenario::Normal).expect("source");
    let first = list(&session, &ItemQuery::default())
        .await
        .expect("first page");
    assert_eq!(first["entries"].as_array().expect("rows").len(), 50);
    assert_eq!(
        first["pagination"],
        json!({"page":1,"next_page":2,
        "source_page_size":100,"source_count":100,"source_total":102})
    );
    assert!(first["entries"][0].get("description").is_none());
    assert_eq!(first["entries"][0]["id"], "7:17");
    assert_eq!(first["entries"][1]["filter_type"], "Armor");
    assert_eq!(fixture::routes(&session).len(), 2);
    let last = list(&session, &ItemQuery { page: 2 })
        .await
        .expect("terminal page");
    assert_eq!(last["pagination"]["next_page"], Value::Null);
    assert_eq!(last["pagination"]["source_count"], 2);
    let empty = list(&session, &ItemQuery { page: 3 })
        .await
        .expect("beyond terminal page");
    assert_eq!(empty["entries"], json!([]));
    assert_eq!(empty["pagination"]["source_total"], 102);
    let magic = fixture::session(Scenario::MagicOnly).expect("source");
    let result = list(&magic, &ItemQuery::default())
        .await
        .expect("classified page");
    assert_eq!(result["entries"], json!([]));
    assert_eq!(result["pagination"]["next_page"], 2);
    assert_eq!(result["pagination"]["source_count"], 100);
}

/// Returns only a freshly selected raw record and preserves its unknown fields.
#[tokio::test]
async fn selection_is_page_scoped_and_raw_fields_survive() {
    let session = fixture::session(Scenario::Normal).expect("source");
    let result = show(
        &session,
        &ItemSelection {
            id: "7:17".to_owned(),
            page: 1,
        },
    )
    .await
    .expect("selected record");
    assert_eq!(result["raw"], fixture::record(17, false, 7));
    assert_eq!(result["page"], 1);
    assert_eq!(
        fixture::routes(&session),
        vec![
            "/api/config/json",
            "/character/v5.1/game-data/items?sharingSetting=2&page=0&pageSize=100"
        ]
    );
    assert!(matches!(
        show(
            &session,
            &ItemSelection {
                id: "7:17".to_owned(),
                page: 2
            }
        )
        .await,
        Err(ItemError::MissingRecord)
    ));
    assert!(matches!(
        show(
            &session,
            &ItemSelection {
                id: "7:18".to_owned(),
                page: 1
            }
        )
        .await,
        Err(ItemError::WrongKind)
    ));
}

/// Rejects broken unselected rows instead of silently filtering them away.
#[tokio::test]
async fn one_bad_row_or_ignored_selector_fails_the_whole_page() {
    for scenario in [
        Scenario::Malformed,
        Scenario::Incomplete,
        Scenario::Duplicate,
        Scenario::InvalidSource,
        Scenario::BadMagic,
        Scenario::Unsupported,
    ] {
        let session = fixture::session(scenario).expect("source");
        assert!(list(&session, &ItemQuery::default()).await.is_err());
        assert!(
            show(
                &session,
                &ItemSelection {
                    id: "7:17".to_owned(),
                    page: 1
                }
            )
            .await
            .is_err()
        );
    }
    let session = fixture::session(Scenario::IgnoredPage).expect("source");
    assert!(matches!(
        list(&session, &ItemQuery { page: 2 }).await,
        Err(ItemError::IncompleteResult)
    ));
}

/// Preserves denied, rejected, expired, and oversized source causes without content.
#[tokio::test]
async fn source_and_session_failures_remain_explicit() {
    for (scenario, message) in [
        (Scenario::Denied, "access_denied"),
        (Scenario::Rejected, "session_rejected"),
        (Scenario::Expired, "session_expired"),
        (Scenario::Oversized, "BodyTooLarge"),
    ] {
        let session = fixture::session(scenario).expect("source");
        let error = list(&session, &ItemQuery::default())
            .await
            .expect_err("failure");
        assert!(error.to_string().contains(message), "{error}");
    }
}

/// Checks exact row/byte limits and distinct composite identities with duplicate numeric IDs.
#[test]
fn bounds_and_composite_identity_are_not_approximated() {
    let sources: HashSet<_> = [7].into_iter().collect();
    let rows = vec![fixture::record(17, false, 7), fixture::record(17, false, 8)];
    let value = assemble(
        fixture::envelope(rows, 0, 2),
        &ItemQuery::default(),
        None,
        &sources,
    )
    .expect("distinct full identities");
    assert_eq!(value["entries"].as_array().expect("entries").len(), 2);
    let rows = (0..101).map(|i| fixture::record(i + 1, false, 7)).collect();
    assert!(matches!(
        assemble(
            fixture::envelope(rows, 0, 101),
            &ItemQuery::default(),
            None,
            &sources
        ),
        Err(ItemError::LimitExceeded)
    ));
    assert!(finish(json!("x".repeat(1024 * 1024 - 2))).is_ok());
    assert!(matches!(
        finish(json!("x".repeat(1024 * 1024 - 1))),
        Err(ItemError::LimitExceeded)
    ));
}

/// Rejects broken row fields and checks the exact source-reference boundary.
#[test]
fn row_identity_metadata_and_reference_limits_are_checked() {
    let sources: HashSet<_> = [7].into_iter().collect();
    for (field, value) in [
        ("id", json!(0)),
        ("entityTypeId", json!(0)),
        ("definitionKey", json!("8:17")),
        ("name", json!(" padded")),
        ("isHomebrew", Value::Null),
        ("weight", json!(-1)),
        ("cost", json!(-1)),
        (
            "sources",
            json!([{"sourceId":7,"sourceType":0,"pageNumber":null}]),
        ),
        (
            "sources",
            json!([{"sourceId":7,"sourceType":1,"pageNumber":-1}]),
        ),
    ] {
        let mut row = fixture::record(17, false, 7);
        row[field] = value;
        assert!(
            matches!(
                assemble(
                    fixture::envelope(vec![row], 0, 1),
                    &ItemQuery::default(),
                    Some("7:17"),
                    &sources
                ),
                Err(ItemError::InvalidResponse)
            ),
            "{field}"
        );
    }
    for (count, valid) in [(1_000, true), (1_001, false)] {
        let mut row = fixture::record(17, false, 7);
        row["sources"] = json!(vec![row["sources"][0].clone(); count]);
        let result = assemble(
            fixture::envelope(vec![row], 0, 1),
            &ItemQuery::default(),
            None,
            &sources,
        );
        if valid {
            assert!(result.is_ok());
        } else {
            assert!(matches!(result, Err(ItemError::LimitExceeded)));
        }
    }
    let mut row = fixture::record(17, false, 7);
    row["accessType"] = json!(1);
    assert!(matches!(
        assemble(
            fixture::envelope(vec![row], 0, 1),
            &ItemQuery::default(),
            None,
            &sources
        ),
        Err(ItemError::UnsupportedOperation)
    ));
}

/// Waits for the one protected request without hiding a stalled fixture.
async fn wait_for_resource(session: &crate::auth::session::Session) {
    tokio::time::timeout(Duration::from_secs(5), async {
        while fixture::routes(session).len() < 2 {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("resource request");
}

/// Preserves typed deadline failure and the single-request budget without real-time delay.
#[tokio::test]
async fn waiting_source_times_out_without_retry_or_partial_success() {
    use crate::{
        auth::session::SessionErrorKind,
        http::{ErrorKind, HttpError},
    };
    use std::error::Error;

    let session = Arc::new(fixture::session(Scenario::Waiting).expect("source"));
    let owner = Arc::clone(&session);
    let task = tokio::spawn(async move { list(&owner, &ItemQuery::default()).await });
    wait_for_resource(&session).await;
    tokio::time::pause();
    tokio::time::advance(Duration::from_secs(61)).await;
    let error = task.await.expect("read task").expect_err("deadline");
    tokio::time::resume();
    match error {
        ItemError::TimedOut => {}
        ItemError::SessionRead(error) => assert!(
            error.kind() == SessionErrorKind::TimedOut
                || error
                    .source()
                    .and_then(|e| e.downcast_ref::<HttpError>())
                    .is_some_and(|e| e.kind() == ErrorKind::Timeout),
            "{error}"
        ),
        error => panic!("unexpected deadline failure: {error}"),
    }
    assert_eq!(fixture::routes(&session).len(), 2);
}

/// Cancels a waiting protected read without retries, fan-out, or global process cleanup.
#[tokio::test]
async fn cancellation_drops_the_only_resource_future() {
    let session = Arc::new(fixture::session(Scenario::Waiting).expect("source"));
    let owner = Arc::clone(&session);
    let task = tokio::spawn(async move { list(&owner, &ItemQuery::default()).await });
    wait_for_resource(&session).await;
    task.abort();
    assert!(task.await.expect_err("cancelled task").is_cancelled());
    assert_eq!(fixture::routes(&session).len(), 2);
}
