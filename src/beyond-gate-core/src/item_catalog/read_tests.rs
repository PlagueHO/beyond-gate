//! Offline item page, classification, selection, bound, and request-contract checks.

use super::{
    Attunement, ItemError, ItemQuery, ItemSelection, MAX_PAGE, MagicItemQuery, list,
    list_magic_items, show, show_magic_item, validate_id,
};
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

/// Filters one page locally without changing raw counts, order, or request construction.
#[tokio::test]
async fn magic_filters_match_labels_and_preserve_mixed_page_contract() {
    let session = fixture::session(Scenario::Normal).expect("source");
    let query = MagicItemQuery {
        name: Some("vEnTeD".to_owned()),
        types: vec!["missing".to_owned(), "other GEAR".to_owned()],
        rarities: vec!["rArE".to_owned()],
        tags: vec!["missing".to_owned(), "UTILITY".to_owned()],
        attunement: Attunement::Yes,
        ..Default::default()
    };
    let result = list_magic_items(&session, &query)
        .await
        .expect("filtered page");
    let entries = result["entries"].as_array().expect("entries");
    assert_eq!(entries.len(), 17);
    assert_eq!(entries[0]["id"], "7:18");
    assert_eq!(entries[1]["id"], "7:24");
    assert_eq!(entries[0]["can_attune"], true);
    assert_eq!(entries[0]["rarity"], "Rare");
    assert_eq!(entries[0]["tags"], json!(["Utility", "Combat"]));
    assert_eq!(result["pagination"]["source_count"], 100);
    assert_eq!(result["pagination"]["source_total"], 102);
    assert_eq!(result["pagination"]["next_page"], 2);
    assert_eq!(result["filters"]["name"], "vEnTeD");
    assert_eq!(
        fixture::routes(&session),
        vec![
            "/api/config/json",
            "/character/v5.1/game-data/items?sharingSetting=2&page=0&pageSize=100"
        ]
    );
    let no = MagicItemQuery {
        attunement: Attunement::No,
        ..Default::default()
    };
    let result = list_magic_items(&session, &no)
        .await
        .expect("no attunement");
    assert_eq!(result["entries"].as_array().expect("entries").len(), 33);
    let any = list_magic_items(&session, &Default::default())
        .await
        .expect("any attunement");
    assert_eq!(any["entries"].as_array().expect("entries").len(), 50);
}

/// Keeps empty matches and all-magic pages page-scoped, without automatic continuation.
#[tokio::test]
async fn magic_pages_and_unmatched_filters_do_not_claim_global_absence() {
    for (scenario, page, count, next) in [
        (Scenario::MagicOnly, 1, 100, json!(2)),
        (Scenario::EquipmentOnly, 1, 0, json!(2)),
        (Scenario::Normal, 2, 1, Value::Null),
        (Scenario::Normal, 3, 0, Value::Null),
        (Scenario::Empty, 1, 0, Value::Null),
    ] {
        let session = fixture::session(scenario).expect("source");
        let result = list_magic_items(
            &session,
            &MagicItemQuery {
                page,
                ..Default::default()
            },
        )
        .await
        .expect("checked page");
        assert_eq!(result["entries"].as_array().expect("entries").len(), count);
        assert_eq!(result["pagination"]["next_page"], next);
        assert_eq!(fixture::routes(&session).len(), 2);
    }
    let session = fixture::session(Scenario::Normal).expect("source");
    for query in [
        MagicItemQuery {
            name: Some("absent".to_owned()),
            ..Default::default()
        },
        MagicItemQuery {
            types: vec!["absent".to_owned()],
            ..Default::default()
        },
        MagicItemQuery {
            rarities: vec!["absent".to_owned()],
            ..Default::default()
        },
        MagicItemQuery {
            tags: vec!["absent".to_owned()],
            ..Default::default()
        },
        MagicItemQuery {
            rarities: vec!["Rare".to_owned()],
            attunement: Attunement::No,
            ..Default::default()
        },
    ] {
        let result = list_magic_items(&session, &query)
            .await
            .expect("valid no match");
        assert_eq!(result["entries"], json!([]));
        assert_eq!(result["pagination"]["next_page"], 2);
        assert_eq!(result["pagination"]["source_count"], 100);
    }
}

/// Rejects malformed and excessive local filters before any session can be read.
#[tokio::test]
async fn magic_filter_input_is_strict_and_bounded() {
    for value in [
        json!({"name":null}),
        json!({"types":null}),
        json!({"rarities":[1]}),
        json!({"tags":"Utility"}),
        json!({"attunement":null}),
        json!({"attunement":true}),
        json!({"page":null}),
        json!({"filter-search":"name"}),
        json!({"source_ids":[7]}),
    ] {
        assert!(serde_json::from_value::<MagicItemQuery>(value).is_err());
    }
    for value in [
        json!({"page":0}),
        json!({"page":MAX_PAGE+1}),
        json!({"name":""}),
        json!({"name":" padded"}),
        json!({"name":"tab\tname"}),
        json!({"name":"é".repeat(129)}),
        json!({"types":["Weapon","weapon"]}),
        json!({"rarities":[""]}),
        json!({"tags":[" trailing "]}),
        json!({"types":(0..51).map(|n|format!("Type{n}")).collect::<Vec<_>>()}),
    ] {
        let query: MagicItemQuery = serde_json::from_value(value).expect("typed input");
        assert!(matches!(
            list_magic_items(&Default::default(), &query).await,
            Err(ItemError::InvalidMagicInput)
        ));
    }
    let default: MagicItemQuery = serde_json::from_value(json!({})).expect("defaults");
    assert_eq!(default.page, MagicItemQuery::default().page);
    assert_eq!(default.page, 1);
    let boundary: MagicItemQuery = serde_json::from_value(json!({
        "name":"é".repeat(128),"tags":(0..50).map(|n|format!("Tag{n}")).collect::<Vec<_>>()
    }))
    .expect("boundary");
    assert!(boundary.validate().is_ok());
}

/// Preserves selected magic raw JSON and diagnoses equipment and page-scoped absence.
#[tokio::test]
async fn magic_detail_is_fresh_and_preserves_raw_identity() {
    let session = fixture::session(Scenario::Normal).expect("source");
    let selection = ItemSelection {
        id: "7:18".to_owned(),
        page: 1,
    };
    let result = show_magic_item(&session, &selection)
        .await
        .expect("magic detail");
    assert_eq!(result["raw"], fixture::record(18, true, 7));
    assert_eq!(result["sources"], result["raw"]["sources"]);
    assert_eq!(fixture::routes(&session).len(), 2);
    assert!(matches!(
        show_magic_item(
            &session,
            &ItemSelection {
                id: "7:17".to_owned(),
                page: 1
            }
        )
        .await,
        Err(ItemError::WrongMagicKind)
    ));
    assert!(matches!(
        show_magic_item(
            &session,
            &ItemSelection {
                id: "7:18".to_owned(),
                page: 2
            }
        )
        .await,
        Err(ItemError::MissingRecord)
    ));
    assert!(matches!(
        show_magic_item(
            &Default::default(),
            &ItemSelection {
                id: "18".to_owned(),
                page: 1
            }
        )
        .await,
        Err(ItemError::InvalidMagicInput)
    ));
}

/// Validates unselected magic metadata even when filters would remove every row.
#[tokio::test]
async fn malformed_magic_filter_metadata_is_not_silently_filtered() {
    let session = fixture::session(Scenario::BadFilterMetadata).expect("source");
    let query = MagicItemQuery {
        name: Some("absent".to_owned()),
        ..Default::default()
    };
    assert!(matches!(
        list_magic_items(&session, &query).await,
        Err(ItemError::InvalidResponse)
    ));
    assert!(list(&session, &Default::default()).await.is_ok());
    for (field, value) in [
        ("rarity", Value::Null),
        ("rarity", json!(" padded")),
        ("canAttune", json!("true")),
        ("tags", json!([null])),
        ("tags", json!("Utility")),
    ] {
        let mut row = fixture::record(18, true, 7);
        row[field] = value;
        assert!(
            matches!(query.matches(&row), Err(ItemError::InvalidResponse)),
            "{field}"
        );
    }
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
            list_magic_items(&session, &MagicItemQuery::default())
                .await
                .is_err()
        );
        assert!(
            show_magic_item(
                &session,
                &ItemSelection {
                    id: "7:18".to_owned(),
                    page: 1
                }
            )
            .await
            .is_err()
        );
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
        let error = list_magic_items(&session, &Default::default())
            .await
            .expect_err("magic failure");
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
