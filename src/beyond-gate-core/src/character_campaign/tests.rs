//! Synthetic record and pagination checks; no live account fixtures.

use super::*;
use serde_json::{Value, json};
use std::{cell::Cell, error::Error};

/// Exercises the actual shared source paths through an isolated loopback service.
#[tokio::test]
async fn live_operations_use_bounded_fixed_routes_and_preserve_source_values() {
    let session = fixture::session(fixture::Scenario::Normal).expect("synthetic services");

    let characters = list_characters(&session).await.unwrap_or_else(|error| {
        let source = error
            .source()
            .and_then(|cause| cause.source())
            .map(ToString::to_string);
        panic!(
            "complete synthetic character list: {error}; cause={source:?}; routes={:?}",
            fixture::requested_routes(&session)
        )
    });
    assert_eq!(characters.characters().len(), 2);
    assert_eq!(
        characters.characters()[0].raw()["unknown"]["preserved"],
        true
    );
    assert_eq!(fixture::request_count(&session), 1);

    let character = get_character(&session, "42")
        .await
        .expect("synthetic selected character");
    assert_eq!(character.raw()["campaign"]["id"], 8);
    assert_eq!(fixture::request_count(&session), 2);

    let campaigns = list_campaigns(&session, false)
        .await
        .expect("synthetic campaign list");
    assert_eq!(campaigns.campaigns()[0].character_ids(), None);
    assert_eq!(campaigns.campaigns()[0].raw()["unknown"]["kept"], true);
    assert_eq!(fixture::request_count(&session), 3);

    let active = list_campaigns(&session, true)
        .await
        .expect("synthetic active campaign list");
    assert_eq!(active.campaigns().len(), 1);
    assert_eq!(fixture::request_count(&session), 4);

    let campaign = get_campaign(&session, "8")
        .await
        .expect("synthetic selected campaign and roster");
    assert_eq!(campaign.character_ids().expect("loaded roster"), ["42"]);
    assert_eq!(campaign.raw()["summary"]["unknown"]["kept"], true);
    assert_eq!(campaign.raw()["characters"][0]["unknown"]["kept"], true);
    assert_eq!(fixture::request_count(&session), 6);
    assert_eq!(
        fixture::requested_routes(&session),
        [
            "/character/v5/characters/list?userid=42",
            "/character/v5/character/42?includecustomitems=true",
            "/api/campaign/stt/user-campaigns",
            "/api/campaign/stt/active-campaigns",
            "/api/campaign/stt/user-campaigns",
            "/api/campaign/stt/active-short-characters/8",
        ]
    );
}

/// Stops before roster fan-out when the selected campaign is not in the complete list.
#[tokio::test]
async fn missing_campaign_stops_before_roster_request() {
    let session = fixture::session(fixture::Scenario::MissingCampaign).expect("synthetic services");
    assert!(matches!(
        get_campaign(&session, "8").await,
        Err(CharacterCampaignError::MissingRecord)
    ));
    assert_eq!(fixture::request_count(&session), 1);
}

/// Maps an HTTP 404 on the selected character route to an explicit missing record.
#[tokio::test]
async fn missing_character_is_not_reported_as_a_session_failure() {
    let session =
        fixture::session(fixture::Scenario::MissingCharacter).expect("synthetic services");
    assert!(matches!(
        get_character(&session, "42").await,
        Err(CharacterCampaignError::MissingRecord)
    ));
    assert_eq!(fixture::request_count(&session), 1);
}

/// Rejects incomplete, changed, and denied source observations without partial values.
#[tokio::test]
async fn live_source_failures_remain_explicit() {
    let incomplete =
        fixture::session(fixture::Scenario::IncompleteCharacters).expect("synthetic services");
    let incomplete_result = list_characters(&incomplete).await;
    assert!(
        matches!(
            incomplete_result,
            Err(CharacterCampaignError::IncompleteDiscovery)
        ),
        "unexpected incomplete-list result: {incomplete_result:?}; cause={:?}; routes={:?}",
        incomplete_result
            .as_ref()
            .err()
            .and_then(|error| error.source())
            .and_then(|cause| cause.source())
            .map(ToString::to_string),
        fixture::requested_routes(&incomplete),
    );

    let changed =
        fixture::session(fixture::Scenario::ChangedCharacter).expect("synthetic services");
    let changed_result = get_character(&changed, "42").await;
    assert!(
        matches!(changed_result, Err(CharacterCampaignError::InvalidResponse)),
        "unexpected selected-character result: {changed_result:?}; routes={:?}",
        fixture::requested_routes(&changed),
    );

    let denied = fixture::session(fixture::Scenario::DeniedRoster).expect("synthetic services");
    let denied_result = get_campaign(&denied, "8").await;
    assert!(
        matches!(
            &denied_result,
        Err(CharacterCampaignError::SessionRead(error))
            if error.kind() == crate::auth::session::SessionErrorKind::AccessDenied
        ),
        "unexpected denied-roster result: {denied_result:?}; routes={:?}",
        fixture::requested_routes(&denied),
    );
    assert_eq!(fixture::request_count(&denied), 2);
}

/// Creates synthetic character metadata with one unknown JSON field.
fn character(id: &str) -> CharacterRecord {
    CharacterRecord {
        id: id.to_owned(),
        name: Some(format!("Invented character {id}")),
        source_url: Some(format!("https://example.com/characters/{id}")),
        raw: json!({"unknown": {"nested": [1, true, "preserved"]}}),
    }
}

/// Creates a synthetic campaign and preserves its association order.
fn campaign(id: &str, character_ids: &[&str]) -> CampaignRecord {
    CampaignRecord {
        id: id.to_owned(),
        name: Some(format!("Invented campaign {id}")),
        source_url: Some(format!("https://example.com/campaigns/{id}")),
        character_ids: Some(
            character_ids
                .iter()
                .map(|value| (*value).to_owned())
                .collect(),
        ),
        raw: json!({"future_campaign_field": {"enabled": true}}),
    }
}

/// Builds one normalized page with an optional opaque continuation.
fn page<T>(cursor: Option<&str>, entries: Vec<T>, next_cursor: Option<&str>) -> ResourcePage<T> {
    ResourcePage {
        cursor: cursor.map(str::to_owned),
        entries,
        next_cursor: next_cursor.map(str::to_owned),
    }
}

/// Assembles a complete one-page character result.
fn characters(entries: Vec<CharacterRecord>) -> Result<CharacterSnapshot, CharacterCampaignError> {
    assemble_characters([Ok(page(None, entries, None))])
}

/// Assembles a complete one-page campaign result.
fn campaigns(entries: Vec<CampaignRecord>) -> Result<CampaignSnapshot, CharacterCampaignError> {
    assemble_campaigns([Ok(page(None, entries, None))])
}

/// Preserves unknown character and campaign JSON without interpreting its fields.
#[test]
fn supplied_raw_json_and_campaign_associations_are_preserved() {
    let character_raw = character("character-1").raw;
    let character_snapshot =
        characters(vec![character("character-1")]).expect("valid synthetic character");
    let found_character = character_snapshot
        .get("character-1")
        .expect("character exists");
    assert_eq!(found_character.raw(), &character_raw);
    assert_eq!(
        found_character.name(),
        Some("Invented character character-1")
    );
    assert_eq!(
        found_character.source_url().map(reqwest::Url::as_str),
        Some("https://example.com/characters/character-1")
    );

    let campaign_raw = campaign("campaign-1", &["character-2", "character-1"]).raw;
    let campaign_snapshot = campaigns(vec![campaign(
        "campaign-1",
        &["character-2", "character-1"],
    )])
    .expect("valid synthetic campaign");
    let found_campaign = campaign_snapshot
        .get("campaign-1")
        .expect("campaign exists");
    assert_eq!(found_campaign.raw(), &campaign_raw);
    assert_eq!(
        found_campaign
            .character_ids()
            .expect("loaded associations")
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        vec!["character-2", "character-1"]
    );
}

/// Distinguishes a complete empty result and a missing record.
#[test]
fn empty_results_and_missing_records_are_distinct() {
    let empty_characters = characters(vec![]).expect("terminal empty character page");
    assert!(empty_characters.characters().is_empty());
    assert!(empty_characters.get("not-present").is_none());
    let empty_campaigns = campaigns(vec![]).expect("terminal empty campaign page");
    assert!(empty_campaigns.campaigns().is_empty());
    assert!(empty_campaigns.get("not-present").is_none());
}

/// Rejects invalid identifiers, optional names, source URLs, and associations.
#[test]
fn malformed_metadata_is_rejected() {
    for invalid in ["", " ", " padded", "padded ", "control\nvalue", "\0"] {
        let mut bad_character = character("valid");
        bad_character.id = invalid.to_owned();
        assert!(matches!(
            characters(vec![bad_character]),
            Err(CharacterCampaignError::InvalidMetadata)
        ));

        let mut bad_campaign = campaign("valid", &[]);
        bad_campaign.character_ids = Some(vec![invalid.to_owned()]);
        assert!(matches!(
            campaigns(vec![bad_campaign]),
            Err(CharacterCampaignError::InvalidMetadata)
        ));
    }

    let mut bad_name = character("valid");
    bad_name.name = Some(" padded".to_owned());
    assert!(matches!(
        characters(vec![bad_name]),
        Err(CharacterCampaignError::InvalidMetadata)
    ));

    for invalid in [
        "not a URL",
        "/relative",
        "http://example.com/character",
        "https://user@example.com/character",
        "https://example.com/character#fragment",
        " https://example.com/character",
    ] {
        let mut bad_character = character("valid");
        bad_character.source_url = Some(invalid.to_owned());
        assert!(matches!(
            characters(vec![bad_character]),
            Err(CharacterCampaignError::InvalidMetadata)
        ));
    }
}

/// Collapses exact duplicate records and rejects conflicting records.
#[test]
fn duplicate_records_must_match_exactly() {
    let snapshot =
        characters(vec![character("one"), character("one")]).expect("identical duplicate records");
    assert_eq!(snapshot.characters().len(), 1);
    assert!(matches!(
        characters(vec![
            character("one"),
            CharacterRecord {
                raw: json!({"different": "value"}),
                ..character("one")
            }
        ]),
        Err(CharacterCampaignError::ConflictingRecord)
    ));

    assert!(matches!(
        campaigns(vec![
            campaign("one", &["character-a"]),
            campaign("one", &["character-b"])
        ]),
        Err(CharacterCampaignError::ConflictingRecord)
    ));
}

/// Assembles multiple pages in stable order and permits empty intermediate pages.
#[test]
fn complete_pages_keep_first_seen_order() {
    let snapshot = assemble_characters([
        Ok(page(
            None,
            vec![character("two"), character("one")],
            Some("opaque-next"),
        )),
        Ok(page(Some("opaque-next"), vec![], None)),
    ])
    .expect("complete page chain");
    assert_eq!(
        snapshot
            .characters()
            .iter()
            .map(Character::id)
            .collect::<Vec<_>>(),
        vec!["two", "one"]
    );

    let campaign_snapshot = assemble_campaigns([
        Ok(page(None, vec![campaign("second", &[])], Some("next"))),
        Ok(page(
            Some("next"),
            vec![campaign("first", &["opaque-character"])],
            None,
        )),
    ])
    .expect("complete campaign page chain");
    assert_eq!(
        campaign_snapshot
            .campaigns()
            .iter()
            .map(Campaign::id)
            .collect::<Vec<_>>(),
        vec!["second", "first"]
    );
}

/// Requires a consistent terminal page and rejects invalid continuation chains.
#[test]
fn incomplete_and_invalid_pages_fail_explicitly() {
    assert!(matches!(
        assemble_characters([]),
        Err(CharacterCampaignError::IncompleteDiscovery)
    ));
    assert!(matches!(
        assemble_campaigns([Ok(page(None, vec![], Some("next")))]),
        Err(CharacterCampaignError::IncompleteDiscovery)
    ));
    for pages in [
        vec![page(Some("not-initial"), vec![], None)],
        vec![page(None, vec![], Some(""))],
        vec![
            page(None, vec![], Some("repeat")),
            page(Some("repeat"), vec![], Some("repeat")),
        ],
        vec![page(None, vec![], None), page(None, vec![], None)],
    ] {
        assert!(matches!(
            assemble_characters(pages.into_iter().map(Ok)),
            Err(CharacterCampaignError::InvalidPagination)
        ));
    }
}

/// Enforces the same fixed page and raw-entry caps for both record types.
#[test]
fn page_and_entry_limits_are_bounded() {
    let pages = (0..MAX_PAGES)
        .map(|index| {
            Ok(ResourcePage {
                cursor: (index > 0).then(|| format!("cursor-{}", index - 1)),
                entries: (0..MAX_ENTRIES_PER_PAGE)
                    .map(|entry| character(&format!("{index}-{entry}")))
                    .collect(),
                next_cursor: (index + 1 < MAX_PAGES).then(|| format!("cursor-{index}")),
            })
        })
        .collect::<Vec<Result<CharacterPage, crate::http::HttpError>>>();
    let snapshot = assemble_characters(pages).expect("exact record and page limits");
    assert_eq!(snapshot.characters().len(), MAX_ENTRIES);

    assert!(matches!(
        assemble_characters([Ok(page(
            None,
            (0..=MAX_ENTRIES_PER_PAGE)
                .map(|index| character(&index.to_string()))
                .collect(),
            None
        ))]),
        Err(CharacterCampaignError::LimitExceeded)
    ));

    let extra_pages = (0..=MAX_PAGES)
        .map(|index| {
            Ok(ResourcePage {
                cursor: (index > 0).then(|| format!("cursor-{}", index - 1)),
                entries: vec![],
                next_cursor: Some(format!("cursor-{index}")),
            })
        })
        .collect::<Vec<Result<CampaignPage, crate::http::HttpError>>>();
    assert!(matches!(
        assemble_campaigns(extra_pages),
        Err(CharacterCampaignError::LimitExceeded)
    ));
}

/// Stops consuming pages when the page cap is exceeded.
#[test]
fn page_iteration_stops_at_first_page_over_limit() {
    let reads = Cell::new(0);
    let pages = (0..).map(|index| {
        reads.set(reads.get() + 1);
        Ok(ResourcePage {
            cursor: (index > 0).then(|| format!("cursor-{}", index - 1)),
            entries: vec![],
            next_cursor: Some(format!("cursor-{index}")),
        })
    });
    assert!(matches!(
        assemble_campaigns(pages),
        Err(CharacterCampaignError::LimitExceeded)
    ));
    assert_eq!(reads.get(), MAX_PAGES + 1);
}

/// Redacts supplied account metadata and raw values from debug output.
#[test]
fn debug_and_error_output_do_not_include_record_data() {
    let private = "synthetic-private-character";
    let record = CharacterRecord {
        id: private.to_owned(),
        name: Some(private.to_owned()),
        source_url: Some(format!("https://example.com/{private}")),
        raw: json!({"secret_like_unknown_field": private}),
    };
    let page = page(None, vec![record.clone()], Some(private));
    let snapshot = characters(vec![record.clone()]).expect("valid synthetic data");
    for diagnostic in [
        format!("{record:?}"),
        format!("{page:?}"),
        format!("{snapshot:?}"),
        format!("{:?}", snapshot.characters()[0]),
    ] {
        assert!(!diagnostic.contains(private));
    }
    let error = CharacterCampaignError::InvalidMetadata;
    assert!(!format!("{error} {error:?}").contains(private));
    assert!(error.source().is_none());
}

/// Preserves valid JSON values regardless of their top-level shape.
#[test]
fn raw_value_can_be_any_json_shape() {
    let value = Value::Array(vec![json!("unknown"), json!(null)]);
    let snapshot = characters(vec![CharacterRecord {
        id: "synthetic".to_owned(),
        name: None,
        source_url: None,
        raw: value.clone(),
    }])
    .expect("JSON shapes are preserved");
    assert_eq!(snapshot.characters()[0].raw(), &value);
}
