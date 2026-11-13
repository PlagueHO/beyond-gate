//! Synthetic feat bounds, selected definitions, and fail-closed collection validation.

use super::{
    fixture::{self, Scenario},
    *,
};
use serde_json::json;
use std::collections::HashSet;

/// Checks metadata, parent/child records, unknown fields, and exact request paths.
#[tokio::test]
async fn successful_synthetic_reads_preserve_raw_and_repeatable_fields() {
    let session = fixture::session(Scenario::Normal).expect("fixture");
    let query = FeatQuery {
        name: Some("literal & take=10".into()),
        prerequisite_subtype_id: Some(240),
        tag_ids: vec![7, 8],
        ..Default::default()
    };
    let rows = list(&session, &query).await.expect("listing");
    assert_eq!(rows.as_array().expect("rows").len(), 2);
    assert_eq!(rows[0]["name"], rows[1]["name"]);
    assert!(rows[0].get("raw").is_none());
    let value = show(&session, "18").await.expect("selected");
    assert_eq!(value["raw"], fixture::record(18));
    assert_eq!(value["is_repeatable"], true);
    assert_eq!(value["repeatable_parent_id"], "17");
    assert_eq!(value["sources"][0]["future"], true);
    let paths = fixture::routes(&session);
    assert_eq!(paths.len(), 3);
    assert_eq!(
        paths[0],
        "/feats?filter-name=literal+%26+take%3D10&filter-prereq-subtype=240&filter-tags=7&filter-tags=8&filter-partnered-content=f"
    );
    assert_eq!(paths[1], "/api/config/json");
    assert_eq!(paths[2], "/character/v5/game-data/feats?sharingSetting=2");
}

/// Rejects invalid IDs before session or network access.
#[tokio::test]
async fn invalid_show_ids_fail_before_access() {
    for id in [
        "",
        "0",
        "01",
        "-1",
        "18446744073709551616",
        "https://example.com/17",
    ] {
        assert!(matches!(
            show(&Session::default(), id).await,
            Err(FeatError::InvalidInput)
        ));
    }
}

/// Rejects invalid local input before requests and unknown options after one website read.
#[tokio::test]
async fn preflight_validation_and_same_response_options() {
    let session = fixture::session(Scenario::Normal).expect("fixture");
    for query in [
        FeatQuery {
            name: Some(" padded".into()),
            ..Default::default()
        },
        FeatQuery {
            name: Some("".into()),
            ..Default::default()
        },
        FeatQuery {
            name: Some("bad\nname".into()),
            ..Default::default()
        },
        FeatQuery {
            prerequisite_subtype_id: Some(0),
            ..Default::default()
        },
        FeatQuery {
            tag_ids: vec![7, 7],
            ..Default::default()
        },
        FeatQuery {
            source_ids: (1..=51).collect(),
            ..Default::default()
        },
    ] {
        assert!(matches!(
            list(&session, &query).await,
            Err(FeatError::InvalidInput)
        ));
    }
    assert!(fixture::routes(&session).is_empty());
    let query = FeatQuery {
        prerequisite_subtype_id: Some(999),
        ..Default::default()
    };
    assert!(matches!(
        list(&session, &query).await,
        Err(FeatError::InvalidInput)
    ));
    assert_eq!(fixture::routes(&session).len(), 1);
}

/// Keeps valid empty, missing, denied, expired, malformed, and incomplete results distinct.
#[tokio::test]
async fn errors_are_not_empty_or_partial_success() {
    let session = fixture::session(Scenario::Empty).expect("fixture");
    assert_eq!(
        list(&session, &FeatQuery::default()).await.expect("empty"),
        json!([])
    );
    assert!(matches!(
        show(&session, "17").await,
        Err(FeatError::MissingRecord)
    ));
    for (scenario, message) in [
        (Scenario::Denied, "access_denied"),
        (Scenario::Rejected, "session_rejected"),
        (Scenario::Expired, "session_expired"),
        (Scenario::Oversized, "BodyTooLarge"),
        (Scenario::Malformed, "invalid feat response"),
        (Scenario::Incomplete, "not complete"),
        (Scenario::Duplicate, "conflicting"),
        (Scenario::InvalidSource, "invalid feat response"),
        (Scenario::Unsupported, "unsupported"),
    ] {
        let session = fixture::session(scenario).expect("fixture");
        let error = show(&session, "17").await.expect_err("failure");
        assert!(error.to_string().contains(message), "{error}");
        assert!(!format!("{error:?}").contains("Synthetic feat text"));
    }
    for (scenario, message) in [
        (Scenario::Denied, "access_denied"),
        (Scenario::Rejected, "session_rejected"),
        (Scenario::Expired, "session_expired"),
        (Scenario::Oversized, "BodyTooLarge"),
        (Scenario::Malformed, "invalid feat response"),
        (Scenario::Incomplete, "unsupported"),
        (Scenario::Duplicate, "conflicting"),
    ] {
        let session = fixture::session(scenario).expect("fixture");
        let error = list(&session, &FeatQuery::default())
            .await
            .expect_err("failure");
        assert!(error.to_string().contains(message), "{error}");
    }
}

/// Checks exact raw count, reference count, canonical bytes, and metadata row limits.
#[test]
fn exact_counts_and_byte_limits() {
    let sources = HashSet::from([7]);
    let rows = (1..=MAX_RECORDS as u64).map(fixture::record).collect();
    assert!(selected(fixture::envelope(rows), "1", &sources).is_ok());
    let rows = (1..=MAX_RECORDS as u64 + 1).map(fixture::record).collect();
    assert!(matches!(
        selected(fixture::envelope(rows), "1", &sources),
        Err(FeatError::LimitExceeded)
    ));
    assert!(finish(Value::String("x".repeat(MAX_RESULT_BYTES - 2))).is_ok());
    assert!(matches!(
        finish(Value::String("x".repeat(MAX_RESULT_BYTES - 1))),
        Err(FeatError::LimitExceeded)
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
        Err(FeatError::LimitExceeded)
    ));
    let normal = fixture::website(Scenario::Normal, "/feats?filter-partnered-content=f");
    let start = normal.find("<ul class=\"listing ").expect("listing");
    let end = normal.find("</ul>").expect("listing end");
    let rows = (1..=MAX_RECORDS).map(|id| format!("<li><div class=\"list-row-name-primary-text\"><a href=\"/feats/{id}-invented\">Invented</a></div><div class=\"list-row-name-secondary-text\">Source</div></li>")).collect::<String>();
    let html = format!(
        "{}<ul class=\"listing listing-rpgfeat rpgfeat-listing\">{rows}{}",
        &normal[..start],
        &normal[end..]
    );
    assert_eq!(
        website::parse(html.as_bytes(), &FeatQuery::default())
            .expect("max")
            .as_array()
            .expect("rows")
            .len(),
        MAX_RECORDS
    );
    let html = html.replace("</ul>", "<li></li></ul>");
    assert!(matches!(
        website::parse(html.as_bytes(), &FeatQuery::default()),
        Err(FeatError::LimitExceeded)
    ));
}

/// Rejects unselected malformed records and nonterminal envelopes before selected output.
#[test]
fn every_row_must_pass_without_inventing_parent_closure() {
    let sources = HashSet::from([7]);
    let mut without_total_count = fixture::envelope(vec![fixture::record(17)]);
    without_total_count
        .as_object_mut()
        .expect("envelope")
        .remove("totalCount");
    assert!(selected(without_total_count, "17", &sources).is_ok());

    for change in [
        json!({"id":0}),
        json!({"id":"18"}),
        json!({"name":" padded"}),
        json!({"sources":null}),
        json!({"sources":[{"sourceId":999}]}),
        json!({"moreDetailsUrl":"/feats/99-invented"}),
        json!({"accessType":1}),
        json!({"isRepeatable":"true"}),
        json!({"repeatableParentId":0}),
        json!({"repeatableParentId":"17"}),
    ] {
        let mut row = fixture::record(18);
        for (key, value) in change.as_object().expect("changes") {
            row[key] = value.clone()
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
            response[key] = value.clone()
        }
        assert!(selected(response, "17", &sources).is_err());
    }
    let value = selected(fixture::envelope(vec![fixture::record(18)]), "18", &sources)
        .expect("parent may be outside scope");
    assert_eq!(value["repeatable_parent_id"], "17");
    let mut row = fixture::record(17);
    row.as_object_mut().expect("record").remove("isRepeatable");
    assert_eq!(
        selected(fixture::envelope(vec![row]), "17", &sources).expect("absent flag")["is_repeatable"],
        Value::Null
    );
}

/// Requires matching form echoes, known footer, strict links, and a genuine empty marker.
#[test]
fn page_structure_echoes_and_display_urls_fail_closed() {
    let normal = fixture::website(Scenario::Normal, "/feats?filter-partnered-content=f");
    for html in [
        normal.replace("listing-footer", "unknown-footer"),
        normal.replace("<form method=\"get\"", "<form method=\"post\""),
        normal.replace("Invented Feat", ""),
        normal.replace(
            "/feats/18-invented",
            "https://example.com/feats/18-invented",
        ),
        normal.replace("value=\"f\" selected", "value=\"f\""),
        format!("{normal}<form id=\"challenge-form\"></form>"),
        fixture::website(Scenario::Empty, "/feats?filter-partnered-content=f")
            .replace("alert no-results", "no-results"),
    ] {
        assert!(website::parse(html.as_bytes(), &FeatQuery::default()).is_err())
    }
    let empty = fixture::website(Scenario::Empty, "/feats?filter-partnered-content=f").replace("Invented empty result", "<div class=\"list-row-name-primary-text\"><a href=\"/feats/17-invented\">Invented</a></div>");
    assert!(website::parse(empty.as_bytes(), &FeatQuery::default()).is_err());
    for link in [
        "https://evil.example/feats/17-invented",
        "/feats/18-invented",
        "/feats/17-invented?q=x",
        "/feats/17-invented#fragment",
        "/feats/17-a/../17-invented",
        "/feats/17-%69nvented",
    ] {
        assert!(source_url("17", link).is_err())
    }
}

/// Rejects extra selected options in both scalar and repeated requested controls.
#[test]
fn complete_filter_selections_must_match() {
    let query = FeatQuery {
        prerequisite_subtype_id: Some(7),
        tag_ids: vec![7],
        source_ids: vec![7],
        source_category_ids: vec![7],
        ..Default::default()
    };
    let url = query.website_url(WEBSITE).expect("URL");
    let normal = fixture::website(
        Scenario::Normal,
        &format!("{}?{}", url.path(), url.query().expect("query")),
    );
    assert!(website::parse(normal.as_bytes(), &query).is_ok());
    for (field, multiple) in [
        ("filter-prereq-subtype", ""),
        ("filter-tags", " multiple"),
        ("filter-source", " multiple"),
        ("filter-source-category", " multiple"),
        ("filter-partnered-content", ""),
    ] {
        let control = format!("<select name=\"{field}\"{multiple}>");
        let extra = normal.replace(
            &control,
            &format!("{control}<option value=\"8\" selected>Extra selection</option>"),
        );
        assert!(
            website::parse(extra.as_bytes(), &query).is_err(),
            "{field} accepted an extra selection"
        );
        let wrong_cardinality = normal.replace(
            &control,
            &format!(
                "<select name=\"{field}\"{}>",
                if multiple.is_empty() { " multiple" } else { "" }
            ),
        );
        assert!(website::parse(wrong_cardinality.as_bytes(), &query).is_err());
    }
}

/// Requires neutral omitted controls and checks implicit or explicit scalar defaults.
#[test]
fn omitted_filters_must_remain_neutral() {
    let query = FeatQuery::default();
    let normal = fixture::website(Scenario::Normal, "/feats?filter-partnered-content=f");
    assert!(website::parse(normal.as_bytes(), &query).is_ok());
    let explicit_default =
        normal.replacen("<option value=\"\">", "<option value=\"\" selected>", 1);
    assert!(website::parse(explicit_default.as_bytes(), &query).is_ok());
    for (field, multiple) in [
        ("filter-prereq-subtype", ""),
        ("filter-tags", " multiple"),
        ("filter-source", " multiple"),
        ("filter-source-category", " multiple"),
    ] {
        let control = format!("<select name=\"{field}\"{multiple}>");
        let filtered = normal.replace(
            &control,
            &format!("{control}<option value=\"7\" selected>Unexpected filter</option>"),
        );
        assert!(
            website::parse(filtered.as_bytes(), &query).is_err(),
            "{field} accepted an omitted selection"
        );
        let missing = normal.replace(&format!("name=\"{field}\""), "name=\"unknown\"");
        assert!(website::parse(missing.as_bytes(), &query).is_err());
    }
    let nonneutral_default = normal.replacen("<option value=\"\">", "<option value=\"7\">", 1);
    assert!(website::parse(nonneutral_default.as_bytes(), &query).is_err());
    let filtered_name = normal.replace(
        "name=\"filter-name\" value=\"\"",
        "name=\"filter-name\" value=\"unexpected\"",
    );
    assert!(website::parse(filtered_name.as_bytes(), &query).is_err());
}

/// Enforces null rejection, scalar prerequisites, exact text bounds, and narrow cookie URLs.
#[test]
fn selectors_and_cookie_urls_are_strict() {
    for value in [
        json!({"page":1}),
        json!({"name":null}),
        json!({"tag_ids":null}),
        json!({"partnered_content":null}),
        json!({"prerequisite_subtype_id":null}),
        json!({"prerequisite_subtype_id":[7,8]}),
    ] {
        assert!(serde_json::from_value::<FeatQuery>(value).is_err())
    }
    let mut query = FeatQuery {
        name: Some("\u{e9}".repeat(128)),
        tag_ids: (1..=50).collect(),
        ..Default::default()
    };
    assert!(query.validate().is_ok());
    query.name.as_mut().expect("name").push('a');
    assert!(query.validate().is_err());
    for path in [
        "/sources/test",
        "/feats/17-invented",
        "/feats?unknown=1",
        "/feats?page=1",
        "/feats?filter-partnered-content=f&filter-tags=01",
        "/feats?filter-partnered-content=f&filter-prereq-subtype=7&filter-prereq-subtype=8",
        "/feats?filter-partnered-content=f&filter-name=x#fragment",
        "/feats?filter-partnered-content=f&filter-tags=7&filter-tags=7",
        "/feats?filter-partnered-content=f&filter-partnered-content=t",
    ] {
        assert!(!query::allowed_list_url(
            &reqwest::Url::parse(&format!("{WEBSITE}{path}")).expect("URL")
        ))
    }
    for (choice, encoded) in [
        (query::PartneredContent::No, "f"),
        (query::PartneredContent::Yes, "t"),
        (query::PartneredContent::Any, ""),
    ] {
        let query = FeatQuery {
            partnered_content: choice,
            ..Default::default()
        };
        let url = query.website_url(WEBSITE).expect("URL");
        assert_eq!(
            url.query().expect("query"),
            format!("filter-partnered-content={encoded}")
        );
        assert!(query::allowed_list_url(&url));
    }
}

/// Cancellation drops only the waiting future and never starts an alternate read.
#[tokio::test]
async fn cancellation_does_not_follow_or_retry() {
    let session = fixture::session(Scenario::Waiting).expect("fixture");
    assert!(
        timeout(
            Duration::from_millis(100),
            list(&session, &FeatQuery::default())
        )
        .await
        .is_err()
    );
    assert_eq!(fixture::routes(&session).len(), 1);
}
