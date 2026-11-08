//! Offline catalog contracts and transport thresholds using invented definitions only.

use super::*;
use crate::auth::session::{SessionErrorKind, fixture as auth};
use fixture::{Scenario, envelope, record};
use serde_json::json;

/// Builds the configured source identity set without visibility or ownership flags.
fn sources() -> HashSet<u64> {
    HashSet::from([7, 8])
}

/// Keeps duplicate names, local matching, source extensions, and parent-scoped identities.
#[tokio::test]
async fn metadata_reads_preserve_candidates_not_rules_text() {
    let session = fixture::session(Scenario::Normal).expect("invented services");
    let classes = list(&session, ClassKind::Class, Some("GHT"), None)
        .await
        .expect("classes");
    assert_eq!(classes.as_array().expect("array").len(), 2);
    assert_eq!(classes[0]["id"], "10");
    assert_eq!(classes[1]["id"], "2190879");
    assert_eq!(classes[1]["source_ids"], json!([8]));
    assert_eq!(classes[0].as_object().expect("summary").len(), 5);
    let selected = show(&session, ClassKind::Subclass, "7001", Some("2190879"))
        .await
        .expect("metadata");
    assert_eq!(selected["parent_class_id"], "2190879");
    assert_eq!(selected["sources"][0]["future_reference_flag"], true);
    assert_eq!(
        selected["more_details_url"],
        "https://www.dndbeyond.com/classes/invented#Example"
    );
    let output = selected.to_string();
    assert!(!output.contains("DISCARDED"));
    assert!(!output.contains("raw"));
    assert_eq!(auth::request_count(&session), 1);
    assert_eq!(fixture::requested_routes(&session).len(), 5);
}

/// Distinguishes empty metadata, missing selections, and cross-parent ambiguity.
#[tokio::test]
async fn empty_missing_and_ambiguous_results_are_distinct() {
    let empty = fixture::session(Scenario::Empty).expect("empty services");
    assert_eq!(
        list(&empty, ClassKind::Class, None, None)
            .await
            .expect("empty list"),
        json!([])
    );
    assert!(matches!(
        show(&empty, ClassKind::Class, "10", None).await,
        Err(ClassCatalogError::MissingRecord)
    ));
    let ambiguous = fixture::session(Scenario::Ambiguous).expect("ambiguous services");
    let records = list(&ambiguous, ClassKind::Subclass, None, None)
        .await
        .expect("both parents");
    assert_eq!(records.as_array().expect("subclasses").len(), 2);
    assert!(matches!(
        show(&ambiguous, ClassKind::Subclass, "7001", None).await,
        Err(ClassCatalogError::AmbiguousRecord)
    ));
    assert!(
        show(&ambiguous, ClassKind::Subclass, "7001", Some("10"))
            .await
            .is_ok()
    );
    let empty_subs = fixture::session(Scenario::EmptySubclasses).expect("empty subclasses");
    assert_eq!(
        list(&empty_subs, ClassKind::Subclass, None, Some("10"))
            .await
            .expect("empty"),
        json!([])
    );
}

/// Rejects invalid selectors before either public requests or protected exchanges.
#[tokio::test]
async fn invalid_input_has_no_source_or_storage_activity() {
    let session = fixture::session(Scenario::Normal).expect("services");
    for id in [
        "",
        "0",
        "01",
        "+1",
        "-1",
        "1.0",
        "18446744073709551616",
        " 1",
        "1\n",
    ] {
        assert!(matches!(
            show(&session, ClassKind::Class, id, None).await,
            Err(ClassCatalogError::InvalidInput)
        ));
    }
    for name in ["", " ", "\n", " padded", "padded "] {
        assert!(matches!(
            list(&session, ClassKind::Class, Some(name), None).await,
            Err(ClassCatalogError::InvalidInput)
        ));
    }
    assert!(
        list(&session, ClassKind::Class, Some(&"x".repeat(256)), None)
            .await
            .is_ok()
    );
    let count = fixture::requested_routes(&session).len();
    assert!(matches!(
        list(&session, ClassKind::Class, Some(&"x".repeat(257)), None).await,
        Err(ClassCatalogError::InvalidInput)
    ));
    assert!(matches!(
        list(&session, ClassKind::Class, None, Some("10")).await,
        Err(ClassCatalogError::InvalidInput)
    ));
    assert_eq!(fixture::requested_routes(&session).len(), count);
    assert_eq!(auth::request_count(&session), 1);
}

/// Stops on malformed, denied, or rejected collections without retry or partial results.
#[tokio::test]
async fn source_failures_stop_at_the_responsible_request() {
    for (scenario, expected_count) in [
        (Scenario::Malformed, 2),
        (Scenario::Incomplete, 2),
        (Scenario::Conflict, 2),
        (Scenario::InvalidSource, 2),
        (Scenario::InvalidConfiguration, 1),
        (Scenario::ChangedParent, 3),
        (Scenario::Denied, 3),
        (Scenario::Rejected, 2),
        (Scenario::Expired, 1),
        (Scenario::Oversized, 2),
    ] {
        let session = fixture::session(scenario).expect("services");
        let error = list(&session, ClassKind::Subclass, None, None)
            .await
            .expect_err("explicit failure");
        assert_eq!(fixture::requested_routes(&session).len(), expected_count);
        assert!(!error.to_string().contains("DISCARDED"));
        match scenario {
            Scenario::Denied => assert!(
                matches!(error, ClassCatalogError::SessionRead(ref error) if error.kind() == SessionErrorKind::AccessDenied)
            ),
            Scenario::Rejected => assert!(
                matches!(error, ClassCatalogError::SessionRead(ref error) if error.kind() == SessionErrorKind::Rejected)
            ),
            Scenario::Expired => assert!(
                matches!(error, ClassCatalogError::SessionRead(ref error) if error.kind() == SessionErrorKind::Expired)
            ),
            Scenario::InvalidConfiguration => assert_eq!(auth::request_count(&session), 0),
            _ => {}
        }
    }
}

/// The global parent cap does not prevent class or explicitly scoped reads.
#[tokio::test]
async fn parent_caps_apply_only_to_global_traversal() {
    let exact = fixture::session(Scenario::ParentLimit).expect("32 parents");
    assert_eq!(
        list(&exact, ClassKind::Subclass, None, None)
            .await
            .expect("complete")
            .as_array()
            .expect("array")
            .len(),
        MAX_PARENTS
    );
    assert_eq!(fixture::requested_routes(&exact).len(), 34);
    assert_eq!(auth::request_count(&exact), 1);
    let over = fixture::session(Scenario::TooManyParents).expect("33 parents");
    assert!(matches!(
        list(&over, ClassKind::Subclass, None, None).await,
        Err(ClassCatalogError::LimitExceeded)
    ));
    assert_eq!(fixture::requested_routes(&over).len(), 2);
    assert_eq!(
        list(&over, ClassKind::Class, None, None)
            .await
            .expect("classes")
            .as_array()
            .expect("array")
            .len(),
        33
    );
    assert!(
        list(&over, ClassKind::Subclass, None, Some("1"))
            .await
            .is_ok()
    );
}

/// Counts admitted whitespace bytes, not the compact re-serialized JSON value.
#[tokio::test]
async fn aggregate_bytes_and_raw_records_apply_before_filters() {
    let bytes = fixture::session(Scenario::AggregateBytes).expect("padded sources");
    assert!(matches!(
        list(&bytes, ClassKind::Subclass, Some("unmatched"), None).await,
        Err(ClassCatalogError::LimitExceeded)
    ));
    assert_eq!(fixture::requested_routes(&bytes).len(), 11);
    let records = fixture::session(Scenario::TooManyRecords).expect("1001 records");
    assert!(matches!(
        list(&records, ClassKind::Class, Some("unmatched"), None).await,
        Err(ClassCatalogError::LimitExceeded)
    ));
    let mut budget = Budget::default();
    budget.admit_bytes(MAX_RESOURCE_BYTES).expect("exact bound");
    assert!(matches!(
        budget.admit_bytes(1),
        Err(ClassCatalogError::LimitExceeded)
    ));
}

/// Validates exact record and canonical result thresholds and their first invalid values.
#[test]
fn exact_record_and_output_bounds() {
    let entries: Vec<_> = (1..=1_000).map(|id| record(id, None, "Invented")).collect();
    let mut count = 0;
    assert_eq!(
        model::collection(
            envelope(entries),
            ClassKind::Class,
            None,
            &sources(),
            &mut count
        )
        .expect("1000 records")
        .len(),
        MAX_RECORDS
    );
    assert!(matches!(
        model::collection(
            envelope(vec![record(1001, None, "Invented")]),
            ClassKind::Class,
            None,
            &sources(),
            &mut count
        ),
        Err(ClassCatalogError::LimitExceeded)
    ));
    result_value(&"x".repeat(MAX_RESULT_BYTES - 2)).expect("exact JSON string bound");
    assert!(matches!(
        result_value(&"x".repeat(MAX_RESULT_BYTES - 1)),
        Err(ClassCatalogError::LimitExceeded)
    ));
}

/// Checks every raw source/parent even when unknown definition fields are discarded.
#[test]
fn parser_rejects_invalid_metadata_and_continuations() {
    let original = envelope(vec![record(10, None, "Invented")]);
    for (field, bad) in [
        ("id", json!(0)),
        ("id", json!(1.5)),
        ("name", json!(" padded")),
        ("parentClassId", json!(10)),
        ("sources", json!([{"sourceId":999}])),
        ("sources", json!([{"sourceId":"7"}])),
        ("sources", json!(null)),
        ("isHomebrew", json!("false")),
        ("slug", json!(false)),
        ("moreDetailsUrl", json!("https://example.com/classes/a")),
        (
            "moreDetailsUrl",
            json!("https://user@www.dndbeyond.com/classes/a"),
        ),
        (
            "moreDetailsUrl",
            json!("https://www.dndbeyond.com/classes/a?token=test"),
        ),
        ("moreDetailsUrl", json!("//example.com/classes/a")),
    ] {
        let mut response = original.clone();
        response["data"][0][field] = bad;
        assert!(model::collection(response, ClassKind::Class, None, &sources(), &mut 0).is_err());
    }
    for (field, bad) in [
        ("pagination", json!({})),
        ("next", json!("cursor")),
        ("totalCount", json!(2)),
        ("totalCount", json!("1")),
    ] {
        let mut response = original.clone();
        response[field] = bad;
        assert!(matches!(
            model::collection(response, ClassKind::Class, None, &sources(), &mut 0),
            Err(ClassCatalogError::IncompleteDiscovery)
        ));
    }
}

/// Measures name filters in UTF-8 bytes rather than characters.
#[test]
fn unicode_name_filters_have_exact_byte_bounds() {
    let exact = "\u{e9}".repeat(MAX_NAME_BYTES / 2);
    validate_input(ClassKind::Class, Some(&exact), None, None).expect("256 UTF-8 bytes");
    assert!(matches!(
        validate_input(ClassKind::Class, Some(&(exact + "x")), None, None),
        Err(ClassCatalogError::InvalidInput)
    ));
}

/// Preserves repeated source references up to their exact per-definition bound.
#[test]
fn source_reference_count_is_bounded_before_projection() {
    let mut definition = record(10, None, "Invented");
    definition["sources"] = json!(vec![json!({"sourceId": 7}); MAX_RECORDS]);
    let records = model::collection(
        envelope(vec![definition.clone()]),
        ClassKind::Class,
        None,
        &sources(),
        &mut 0,
    )
    .expect("1000 source references");
    let selected = result_value(&records[0]).expect("metadata");
    assert_eq!(
        selected["sources"].as_array().expect("references").len(),
        MAX_RECORDS
    );
    assert_eq!(selected["source_ids"], json!([7]));
    definition["sources"] = json!(vec![json!({"sourceId": 7}); MAX_RECORDS + 1]);
    assert!(matches!(
        model::collection(
            envelope(vec![definition]),
            ClassKind::Class,
            None,
            &sources(),
            &mut 0
        ),
        Err(ClassCatalogError::LimitExceeded)
    ));
}

/// Emits null for missing optional metadata and rejects duplicate IDs within one parent.
#[test]
fn optional_metadata_and_parent_local_duplicates_are_explicit() {
    for missing in [false, true] {
        let mut definition = record(7001, Some(10), "Invented");
        let fields = definition.as_object_mut().expect("definition");
        for field in ["slug", "moreDetailsUrl"] {
            if missing {
                fields.remove(field);
            } else {
                fields.insert(field.to_owned(), Value::Null);
            }
        }
        let records = model::collection(
            envelope(vec![definition.clone()]),
            ClassKind::Subclass,
            Some(10),
            &sources(),
            &mut 0,
        )
        .expect("optional metadata");
        let selected = result_value(&records[0]).expect("metadata");
        assert_eq!(selected["slug"], Value::Null);
        assert_eq!(selected["more_details_url"], Value::Null);
        assert!(matches!(
            model::collection(
                envelope(vec![definition.clone(), definition]),
                ClassKind::Subclass,
                Some(10),
                &sources(),
                &mut 0
            ),
            Err(ClassCatalogError::ConflictingMetadata)
        ));
    }
}

/// Dropping the pending read cancels it before any later parent request.
#[tokio::test]
async fn cancellation_stops_waiting_discovery() {
    let session = fixture::session(Scenario::Waiting).expect("waiting source");
    let operation = list(&session, ClassKind::Subclass, None, None);
    assert!(
        tokio::time::timeout(Duration::from_millis(100), operation)
            .await
            .is_err()
    );
    assert_eq!(fixture::requested_routes(&session).len(), 2);
    assert_eq!(auth::request_count(&session), 1);
}
