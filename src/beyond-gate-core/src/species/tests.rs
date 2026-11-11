//! Synthetic species contracts, access failures, and explicit local bounds.

use super::*;
use crate::species::fixture::{self, Scenario};

/// Returns one invented species record with an unknown nested field.
fn record(id: u64, full_name: &str, legacy: bool, subrace: bool) -> Value {
    json!({
        "entityRaceId": id,
        "entityRaceTypeId": 4,
        "fullName": full_name,
        "baseName": "Invented Origin",
        "baseRaceName": if subrace { "Invented Origin" } else { "" },
        "description": "Synthetic species detail.",
        "isHomebrew": false,
        "isLegacy": legacy,
        "isSubRace": subrace,
        "size": "Medium",
        "sources": [{"sourceId": 7, "future_reference_flag": true}],
        "future_definition": {"nested": [true, null, 3]}
    })
}

/// Wraps invented records in the source's terminal response envelope.
fn envelope(records: Vec<Value>) -> Value {
    json!({"id":0,"success":true,"message":"","data":records,"pagination":null})
}

/// Lists return website metadata while show preserves the complete selected record.
#[tokio::test]
async fn successful_reads_preserve_variant_and_unknown_fields() {
    let session = fixture::session(Scenario::Normal).expect("synthetic session");
    let entries = list(&session).await.unwrap_or_else(|error| {
        panic!("{error}; routes={:?}", fixture::requested_routes(&session))
    });
    assert_eq!(entries.as_array().expect("entries").len(), 2);
    assert_eq!(entries[0]["id"], "17");
    assert_eq!(entries[0]["name"], "Invented Origin 17");
    assert_eq!(entries[0]["source"], "Invented Source");
    assert_eq!(
        entries[0]["source_url"],
        "https://www.dndbeyond.com/species/17-invented"
    );
    assert_eq!(entries[0]["is_legacy"], false);
    assert_eq!(entries[1]["is_legacy"], true);

    let selected = show(&session, "18").await.expect("selected species");
    assert_eq!(selected["id"], "18");
    assert_eq!(
        selected["raw"],
        record(18, "Invented Coastal Origin", true, true)
    );
    assert_eq!(
        selected["raw"]["future_definition"]["nested"],
        json!([true, null, 3])
    );
    assert_eq!(
        fixture::requested_routes(&session),
        vec!["/species".to_owned(), COLLECTION.to_owned()]
    );
}

/// Keeps missing listing structure, missing records, and unsupported pages distinct.
#[tokio::test]
async fn empty_missing_and_unsupported_are_distinct() {
    let session = fixture::session(Scenario::Empty).expect("synthetic session");
    assert!(matches!(
        list(&session).await,
        Err(SpeciesError::IncompleteResult)
    ));
    assert!(matches!(
        show(&session, "17").await,
        Err(SpeciesError::MissingRecord)
    ));

    let session = fixture::session(Scenario::Unsupported).expect("synthetic session");
    assert!(matches!(
        list(&session).await,
        Err(SpeciesError::UnsupportedOperation)
    ));
    assert!(matches!(
        show(&session, "17").await,
        Err(SpeciesError::UnsupportedOperation)
    ));
}

/// Rejects invalid identifiers before any protected request starts.
#[tokio::test]
async fn invalid_identifiers_do_not_read_the_source() {
    let session = fixture::session(Scenario::Normal).expect("synthetic session");
    for id in ["", "0", "01", "-1", " 17", "18446744073709551616"] {
        assert!(matches!(
            show(&session, id).await,
            Err(SpeciesError::InvalidInput)
        ));
    }
    assert!(fixture::requested_routes(&session).is_empty());
}

/// Separates denied access, rejected sessions, malformed pages, and unsupported paging.
#[tokio::test]
async fn source_failures_keep_their_classification() {
    let session = fixture::session(Scenario::Denied).expect("synthetic session");
    assert!(
        list(&session)
            .await
            .expect_err("denied")
            .to_string()
            .contains("access_denied")
    );

    let session = fixture::session(Scenario::Rejected).expect("synthetic session");
    assert!(
        list(&session)
            .await
            .expect_err("rejected")
            .to_string()
            .contains("session_rejected")
    );

    let session = fixture::session(Scenario::Expired).expect("synthetic session");
    assert!(
        list(&session)
            .await
            .expect_err("expired")
            .to_string()
            .contains("session_expired")
    );

    let session = fixture::session(Scenario::Malformed).expect("synthetic session");
    assert!(matches!(
        list(&session).await,
        Err(SpeciesError::IncompleteResult)
    ));

    let session = fixture::session(Scenario::Incomplete).expect("synthetic session");
    assert!(matches!(
        list(&session).await,
        Err(SpeciesError::UnsupportedOperation)
    ));

    let session = fixture::session(Scenario::Oversized).expect("synthetic session");
    assert!(matches!(
        list(&session).await,
        Err(SpeciesError::SessionRead(error)) if error.kind() == SessionErrorKind::ReadFailed
    ));
}

/// Rejects duplicate, excessive, and invalid HTML rows without partial results.
#[tokio::test]
async fn website_listing_rejects_ambiguous_or_unbounded_pages() {
    let duplicate = fixture::session(Scenario::Duplicate).expect("synthetic session");
    assert!(matches!(
        list(&duplicate).await,
        Err(SpeciesError::ConflictingIdentity)
    ));

    let too_many = fixture::session(Scenario::TooManyRecords).expect("synthetic session");
    assert!(matches!(
        list(&too_many).await,
        Err(SpeciesError::LimitExceeded)
    ));

    let html =
        fixture::website(Scenario::Normal).replace("listing-card__title", "unexpected-title");
    assert!(matches!(
        website::parse(html.as_bytes()),
        Err(SpeciesError::InvalidResponse)
    ));
}

/// Restricts cookie-backed listing reads to the fixed page without a query.
#[test]
fn website_url_guard_accepts_only_the_fixed_listing_path() {
    for (path, allowed) in [
        ("/species", true),
        ("/species?search=invented", false),
        ("/species#content", false),
        ("/species/", false),
        ("/species/17-invented", false),
    ] {
        let url = reqwest::Url::parse(&format!("{WEBSITE}{path}")).expect("URL");
        assert_eq!(website::allowed_list_url(&url), allowed, "{path}");
    }
}

/// Rejects duplicate, invalid, and access-bearing records without partial output.
#[test]
fn malformed_and_unsupported_records_fail() {
    assert!(matches!(
        parse_collection(envelope(vec![
            record(17, "Invented Origin", false, false),
            record(17, "Invented Duplicate", true, false)
        ])),
        Err(SpeciesError::ConflictingIdentity)
    ));

    let mut invalid_id = record(17, "Invented Origin", false, false);
    invalid_id["entityRaceId"] = json!(0);
    assert!(matches!(
        parse_collection(envelope(vec![invalid_id])),
        Err(SpeciesError::InvalidResponse)
    ));

    let mut invalid_name = record(17, "Invented Origin", false, false);
    invalid_name["fullName"] = json!("");
    invalid_name["baseName"] = json!("");
    assert!(matches!(
        parse_collection(envelope(vec![invalid_name])),
        Err(SpeciesError::InvalidResponse)
    ));

    let mut unexpected_access = record(17, "Invented Origin", false, false);
    unexpected_access["accessType"] = json!(1);
    assert!(matches!(
        parse_collection(envelope(vec![unexpected_access])),
        Err(SpeciesError::UnsupportedOperation)
    ));

    assert!(matches!(
        parse_collection(json!({"id":0,"data": [], "success": true, "message":"",
            "pagination": {"next": 2}})),
        Err(SpeciesError::IncompleteResult)
    ));
    assert!(matches!(
        parse_collection(json!({"id":0,"data": [], "success": true, "message":"",
            "pagination": null, "accessType": {}})),
        Err(SpeciesError::UnsupportedOperation)
    ));
}

/// Enforces the complete raw-record count and canonical-result byte limits.
#[test]
fn collection_and_result_bounds_are_explicit() {
    let maximum = (1..=MAX_RECORDS as u64)
        .map(|id| record(id, "Invented Origin", false, false))
        .collect::<Vec<_>>();
    assert_eq!(
        parse_collection(envelope(maximum))
            .expect("maximum collection")
            .len(),
        MAX_RECORDS
    );

    let too_many = (1..=MAX_RECORDS as u64 + 1)
        .map(|id| record(id, "Invented Origin", false, false))
        .collect::<Vec<_>>();
    assert!(matches!(
        parse_collection(envelope(too_many)),
        Err(SpeciesError::LimitExceeded)
    ));

    assert!(finish(Value::String("x".repeat(MAX_RESULT_BYTES - 2))).is_ok());
    assert!(matches!(
        finish(Value::String("x".repeat(MAX_RESULT_BYTES - 1))),
        Err(SpeciesError::LimitExceeded)
    ));

    let too_many_rows = fixture::website(Scenario::TooManyRecords);
    assert!(matches!(
        website::parse(too_many_rows.as_bytes()),
        Err(SpeciesError::LimitExceeded)
    ));
}

/// Uses the source's supported name fallback and rejects malformed optional variants.
#[test]
fn optional_variant_metadata_is_checked() {
    let mut fallback = record(17, "Invented Origin", false, false);
    fallback["fullName"] = json!("");
    assert_eq!(
        metadata(&fallback).expect("base-name fallback")["name"],
        "Invented Origin"
    );

    let mut invalid_flag = record(18, "Invented Origin", false, false);
    invalid_flag["isLegacy"] = json!("yes");
    assert!(matches!(
        metadata(&invalid_flag),
        Err(SpeciesError::InvalidResponse)
    ));
}
