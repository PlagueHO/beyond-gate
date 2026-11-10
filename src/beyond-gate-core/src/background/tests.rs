//! Synthetic background contracts, exact bounds, and request isolation.

use super::{
    fixture::{self, Scenario},
    *,
};

/// Preserves metadata duplicates, raw extensions, attribution, and bounded request paths.
#[tokio::test]
async fn successful_reads_and_unknown_fields() {
    let session = fixture::session(Scenario::Normal).expect("fixture");
    let query = BackgroundQuery {
        skill_ids: vec![240, 241],
        ..Default::default()
    };
    let rows = list(&session, &query).await.expect("listing");
    assert_eq!(rows.as_array().expect("rows").len(), 2);
    assert_eq!(rows[0]["name"], rows[1]["name"]);
    assert!(rows[0].get("raw").is_none());
    let value = show(&session, "17").await.expect("selected");
    assert_eq!(value["raw"], fixture::record(17));
    assert_eq!(value["sources"][0]["future"], true);
    let paths = fixture::routes(&session);
    assert_eq!(paths.len(), 3);
    assert!(paths[0].contains("filter-skills=240&filter-skills=241"));
    assert_eq!(paths[1], "/api/config/json");
    assert_eq!(
        paths[2],
        "/character/v5/game-data/backgrounds?sharingSetting=2"
    );
    assert!(
        show(&session, "999")
            .await
            .err()
            .is_some_and(|e| matches!(e, BackgroundError::MissingRecord))
    );
}

/// Rejects invalid input and unknown options without issuing a collection request.
#[tokio::test]
async fn preflight_and_option_bounds() {
    let session = fixture::session(Scenario::Normal).expect("fixture");
    for id in [
        "",
        "0",
        "01",
        "-1",
        "18446744073709551616",
        "https://example.com/17",
    ] {
        assert!(show(&session, id).await.is_err());
    }
    for name in ["", " padded", "bad\nname"] {
        assert!(
            list(
                &session,
                &BackgroundQuery {
                    name: Some(name.into()),
                    ..Default::default()
                }
            )
            .await
            .is_err()
        );
    }
    assert!(fixture::routes(&session).is_empty());
    let query = BackgroundQuery {
        skill_ids: vec![999],
        ..Default::default()
    };
    assert!(matches!(
        list(&session, &query).await,
        Err(BackgroundError::InvalidInput)
    ));
    assert_eq!(fixture::routes(&session).len(), 1);
    for ids in [vec![0], vec![7, 7], (1..=51).collect()] {
        assert!(
            BackgroundQuery {
                skill_ids: ids,
                ..Default::default()
            }
            .validate()
            .is_err()
        );
    }
    assert!(
        BackgroundQuery {
            skill_ids: (1..=50).collect(),
            ..Default::default()
        }
        .validate()
        .is_ok()
    );
}

/// Keeps empty, missing, denied, malformed, incomplete, and unsupported results distinct.
#[tokio::test]
async fn source_failures_are_not_successful_empty_results() {
    let session = fixture::session(Scenario::Empty).expect("fixture");
    assert_eq!(
        list(&session, &BackgroundQuery::default())
            .await
            .expect("empty"),
        json!([])
    );
    assert!(matches!(
        show(&session, "17").await,
        Err(BackgroundError::MissingRecord)
    ));
    for (scenario, category) in [
        (Scenario::Denied, "access_denied"),
        (Scenario::Rejected, "session_rejected"),
        (Scenario::Expired, "session_expired"),
        (Scenario::Oversized, "BodyTooLarge"),
        (Scenario::Malformed, "invalid background response"),
        (Scenario::Incomplete, "not complete"),
        (Scenario::Duplicate, "conflicting"),
        (Scenario::InvalidSource, "invalid background response"),
        (Scenario::Unsupported, "unsupported"),
    ] {
        let session = fixture::session(scenario).expect("fixture");
        let error = show(&session, "17").await.expect_err("failure");
        assert!(error.to_string().contains(category), "{error}");
        assert!(!format!("{error:?}").contains("Synthetic background text"));
    }
}

/// Checks byte/count boundaries before filtering or selected-record output.
#[test]
fn exact_result_and_collection_limits() {
    let sources = HashSet::from([7]);
    let rows = (1..=MAX_RECORDS as u64).map(fixture::record).collect();
    assert!(selected(fixture::envelope(rows), "1", &sources).is_ok());
    let rows = (1..=MAX_RECORDS as u64 + 1).map(fixture::record).collect();
    assert!(matches!(
        selected(fixture::envelope(rows), "1", &sources),
        Err(BackgroundError::LimitExceeded)
    ));
    assert!(finish(Value::String("x".repeat(MAX_RESULT_BYTES - 2))).is_ok());
    assert!(matches!(
        finish(Value::String("x".repeat(MAX_RESULT_BYTES - 1))),
        Err(BackgroundError::LimitExceeded)
    ));
    let mut row = fixture::record(17);
    row["sources"] = Value::Array(vec![json!({"sourceId":7}); MAX_RECORDS]);
    assert!(selected(fixture::envelope(vec![row.clone()]), "17", &sources).is_ok());
    row["sources"]
        .as_array_mut()
        .expect("references")
        .push(json!({"sourceId":7}));
    assert!(matches!(
        selected(fixture::envelope(vec![row]), "17", &sources),
        Err(BackgroundError::LimitExceeded)
    ));
    let html = fixture::website(Scenario::WideTable, "/backgrounds?");
    let rows = website::parse(html.as_bytes(), &BackgroundQuery::default()).expect("maximum rows");
    assert_eq!(rows.as_array().expect("array").len(), MAX_RECORDS);
    let html = html.replace(
        "</ul><div class=\"listing-footer\">",
        "<li></li></ul><div class=\"listing-footer\">",
    );
    assert!(matches!(
        website::parse(html.as_bytes(), &BackgroundQuery::default()),
        Err(BackgroundError::LimitExceeded)
    ));
}

/// Uses literal encoded names, repeated public option keys, and all partnered choices.
#[test]
fn all_filter_encodings_and_text_byte_limits() {
    let query = BackgroundQuery {
        name: Some("invented & take=10".to_owned()),
        skill_ids: vec![240, 241],
        tag_ids: vec![7, 8],
        source_ids: vec![7],
        source_category_ids: vec![8],
        partnered_content: query::PartneredContent::Any,
    };
    query.validate().expect("valid");
    let url = query.website_url(WEBSITE).expect("URL");
    assert_eq!(
        url.query().expect("query"),
        "filter-name=invented+%26+take%3D10&filter-skills=240&filter-skills=241&filter-tags=7&filter-tags=8&filter-source=7&filter-source-category=8&filter-partnered-content="
    );
    assert!(query::allowed_list_url(&url));
    for (choice, encoded) in [
        (query::PartneredContent::No, "f"),
        (query::PartneredContent::Yes, "t"),
        (query::PartneredContent::Any, ""),
    ] {
        let query = BackgroundQuery {
            partnered_content: choice,
            ..Default::default()
        };
        assert_eq!(
            query
                .website_url(WEBSITE)
                .expect("URL")
                .query()
                .expect("query"),
            format!("filter-partnered-content={encoded}")
        );
    }
    let mut query = BackgroundQuery {
        name: Some("\u{e9}".repeat(128)),
        ..Default::default()
    };
    assert!(query.validate().is_ok());
    query.name.as_mut().expect("name").push('a');
    assert!(query.validate().is_err());
    let roundtrip = serde_json::to_value(BackgroundQuery::default()).expect("serialize");
    assert!(serde_json::from_value::<BackgroundQuery>(roundtrip).is_ok());
}

/// Fails the entire result for partial envelopes, invalid unselected rows, and malformed HTML.
#[test]
fn every_row_and_terminal_structure_must_be_valid() {
    let sources = HashSet::from([7]);
    for change in [
        json!({"id":0}),
        json!({"id":"18"}),
        json!({"name":" padded"}),
        json!({"sources":null}),
        json!({"sources":[{"sourceId":999}]}),
        json!({"moreDetailsUrl":"/backgrounds/99-invented"}),
        json!({"accessType":1}),
    ] {
        let mut row = fixture::record(18);
        for (key, value) in change.as_object().expect("changes") {
            row[key] = value.clone();
        }
        assert!(
            selected(
                fixture::envelope(vec![fixture::record(17), row]),
                "17",
                &sources
            )
            .is_err()
        );
    }
    for change in [
        json!({"success":false}),
        json!({"pagination":{"next":2}}),
        json!({"pagination":false}),
        json!({"totalCount":2}),
        json!({"hasMore":true}),
        json!({"accessTypes":{"17":1}}),
    ] {
        let mut response = fixture::envelope(vec![fixture::record(17)]);
        for (key, value) in change.as_object().expect("changes") {
            response[key] = value.clone();
        }
        assert!(selected(response, "17", &sources).is_err());
    }
    let normal = fixture::website(Scenario::Normal, "/backgrounds?");
    for html in [
        normal.replace("listing-footer", "unknown-footer"),
        normal.replace("<form method=\"get\"", "<form method=\"post\""),
        normal.replace("Invented Origin", ""),
        normal.replace(
            "/backgrounds/18-invented",
            "https://example.com/backgrounds/18-invented",
        ),
        normal.replace("<option value=\"f\">No</option>", ""),
        normal.replace(
            "</form>",
            "</form><form method=\"get\" action=\"/backgrounds\"></form>",
        ),
        format!("{normal}<form id=\"challenge-form\"></form>"),
        fixture::website(Scenario::Empty, "").replace("alert no-results", "no-results"),
    ] {
        assert!(website::parse(html.as_bytes(), &BackgroundQuery::default()).is_err());
    }
}

/// Rejects a contradictory empty marker instead of silently discarding its record.
#[test]
fn empty_marker_cannot_hide_background_records() {
    let html = fixture::website(Scenario::Empty, "").replace(
        "Invented empty result",
        "<div class=\"list-row-name-primary-text\"><a href=\"/backgrounds/17-invented\">Invented Origin</a></div><div class=\"list-row-name-secondary-text\">Invented Source</div>",
    );
    assert!(matches!(
        website::parse(html.as_bytes(), &BackgroundQuery::default()),
        Err(BackgroundError::InvalidResponse)
    ));
}

/// Rejects null/unknown input fields, forged display links, and widened cookie URLs.
#[test]
fn strict_schemas_and_url_guards() {
    for value in [
        json!({"page":1}),
        json!({"name":null}),
        json!({"skill_ids":null}),
        json!({"partnered_content":null}),
    ] {
        assert!(serde_json::from_value::<BackgroundQuery>(value).is_err());
    }
    for path in [
        "/sources/test",
        "/backgrounds/17-invented",
        "/backgrounds?unknown=1",
        "/backgrounds?page=1",
        "/backgrounds?filter-skills=01",
        "/backgrounds?filter-name=x#fragment",
        "/backgrounds?filter-partnered-content=f&filter-partnered-content=t",
        "/backgrounds?filter-partnered-content=f&filter-name=x&filter-name=y",
        "/backgrounds?filter-partnered-content=f&filter-skills=7&filter-skills=7",
    ] {
        assert!(!query::allowed_list_url(
            &reqwest::Url::parse(&format!("{WEBSITE}{path}")).expect("url")
        ));
    }
    for link in [
        "https://evil.example/backgrounds/17-invented",
        "/backgrounds/18-invented",
        "/backgrounds/17-invented?q=x",
        "/backgrounds/17-invented#fragment",
        "/backgrounds/17-a/../17-invented",
    ] {
        assert!(source_url("17", link).is_err(), "{link}");
    }
}

/// Verifies cancellation releases a waiting source operation.
#[tokio::test]
async fn waiting_read_can_be_cancelled() {
    let session = fixture::session(Scenario::Waiting).expect("fixture");
    assert!(
        tokio::time::timeout(Duration::from_millis(500), show(&session, "17"))
            .await
            .is_err()
    );
    assert_eq!(
        fixture::routes(&session).last().map(String::as_str),
        Some("/character/v5/game-data/backgrounds?sharingSetting=2")
    );
}
