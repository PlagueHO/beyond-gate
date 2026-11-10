//! Offline filter serialization, website structure, fresh access, and exact bounds.

use super::{fixture::Scenario, *};

/// Builds strict queries from invented JSON, with no production session access.
fn query(value: Value) -> MonsterQuery {
    serde_json::from_value(value).expect("invented query")
}

/// Preserves website order despite reversed API records and reads only explicit pages.
#[tokio::test]
async fn pages_and_detail_preserve_shared_contracts() {
    let session = fixture::session(Scenario::Normal).expect("fixture");
    let first = list(&session, &query(json!({"name":"beast & take=100"})))
        .await
        .expect("first");
    assert_eq!(first["entries"][0]["id"], "17");
    assert_eq!(first["entries"][1]["id"], "18");
    assert_eq!(first["pagination"], json!({"page":1,"next_page":2}));
    assert!(first["entries"][0].get("raw").is_none());
    let routes = fixture::routes(&session);
    assert_eq!(routes.len(), 2);
    assert!(routes[0].contains("filter-search=beast+%26+take%3D100"));
    assert_eq!(routes[1], "/v1/Monster?ids=17&ids=18");
    let second = list(&session, &query(json!({"page":2})))
        .await
        .expect("second");
    assert_eq!(second["entries"][0]["id"], "19");
    assert_eq!(second["pagination"], json!({"page":2,"next_page":null}));
    let detail = show(&session, "17").await.expect("detail");
    assert_eq!(detail["raw"], fixture::record(17));
    assert_eq!(fixture::routes(&session).len(), 5);
}

/// Empty pages use an explicit marker and never call the selected-ID collection.
#[tokio::test]
async fn empty_missing_and_mismatched_results_are_distinct() {
    let session = fixture::session(Scenario::Empty).expect("fixture");
    let value = list(&session, &query(json!({"page":2})))
        .await
        .expect("empty");
    assert_eq!(
        value,
        json!({"entries":[],"pagination":{"page":2,"next_page":null}})
    );
    assert_eq!(fixture::routes(&session).len(), 1);
    assert!(matches!(
        show(&session, "99").await,
        Err(MonsterError::MissingRecord)
    ));
    let changed = fixture::session(Scenario::ChangedIdentity).expect("fixture");
    assert!(matches!(
        show(&changed, "17").await,
        Err(MonsterError::InvalidResponse)
    ));
}

/// Rejects invalid ranges, array values, text, and pages before touching a session.
#[tokio::test]
async fn invalid_input_never_touches_the_session() {
    let session = fixture::session(Scenario::Normal).expect("fixture");
    for value in [
        json!({"page":0}),
        json!({"name":""}),
        json!({"name":" padded"}),
        json!({"name":"line\nbreak"}),
        json!({"name":"x".repeat(257)}),
        json!({"cr_min":"0.5"}),
        json!({"cr_min":"2","cr_max":"1/2"}),
        json!({"armor_class_min":10,"armor_class_max":9}),
        json!({"average_hp_min":2,"average_hp_max":1}),
        json!({"type_ids":[0]}),
        json!({"source_ids":[1,1]}),
        json!({"size_ids":(1..=51).collect::<Vec<_>>()}),
    ] {
        assert!(matches!(
            list(&session, &query(value)).await,
            Err(MonsterError::InvalidInput)
        ));
    }
    for id in [
        "",
        "0",
        "01",
        "+1",
        "-1",
        " 1",
        "1/2",
        "18446744073709551616",
    ] {
        assert!(matches!(
            show(&session, id).await,
            Err(MonsterError::InvalidInput)
        ));
    }
    for value in [
        json!({"skip":1}),
        json!({"take":1}),
        json!({"unknown":true}),
        json!({"name":null}),
        json!({"page":null}),
        json!({"size_ids":null}),
        json!({"size_ids":[-1]}),
        json!({"legendary":null}),
        json!({"legendary":"false"}),
        json!({"armor_class_min":null}),
        json!({"cr_min":null}),
        json!({"partnered_content":null}),
    ] {
        assert!(serde_json::from_value::<MonsterQuery>(value).is_err());
    }
    assert!(
        query(json!({"name":"\u{e9}".repeat(128)}))
            .validate()
            .is_ok()
    );
    assert!(
        query(json!({"name":"\u{e9}".repeat(129)}))
            .validate()
            .is_err()
    );
    assert!(fixture::routes(&session).is_empty());
    assert_eq!(crate::auth::session::fixture::request_count(&session), 0);
}

/// Serializes every observed selector, preserving repeated keys and literal search.
#[tokio::test]
async fn all_filter_fields_reach_the_website_not_the_collection() {
    let mut value = json!({
        "name":"beast & page=99", "page":1, "cr_min":"1/8", "cr_max":"24",
        "armor_class_min":0,"armor_class_max":99,"average_hp_min":1,"average_hp_max":1000,
        "legendary":false,"mythic":true,"has_lair":false,"partnered_content":"any"
    });
    for (field, _, _) in query::ID_FILTERS {
        value[field] = json!([1, 2]);
    }
    let selected_query = query(value);
    let session = fixture::session(Scenario::Normal).expect("fixture");
    list(&session, &selected_query).await.expect("all filters");
    let routes = fixture::routes(&session);
    let url = reqwest::Url::parse(&format!("http://127.0.0.1{}", routes[0])).expect("query");
    let pairs: Vec<_> = url.query_pairs().collect();
    for (_, _, field) in query::ID_FILTERS {
        assert_eq!(
            pairs
                .iter()
                .filter(|(k, _)| k == field)
                .map(|(_, v)| v.as_ref())
                .collect::<Vec<_>>(),
            ["1", "2"]
        );
    }
    for (field, expected) in [
        ("filter-search", "beast & page=99"),
        ("filter-cr-min", "2"),
        ("filter-cr-max", "29"),
        ("filter-armor-class-min", "0"),
        ("filter-armor-class-max", "99"),
        ("filter-average-hp-min", "1"),
        ("filter-average-hp-max", "1000"),
        ("filter-is-legendary", "f"),
        ("filter-is-mythic", "t"),
        ("filter-has-lair", "f"),
        ("filter-partnered-content", ""),
    ] {
        assert!(
            pairs.iter().any(|(k, v)| k == field && v == expected),
            "{field}"
        );
    }
    assert_eq!(routes[1], "/v1/Monster?ids=17&ids=18");
    for (choice, expected) in [("yes", "t"), ("no", "f"), ("any", "")] {
        let url = query(json!({"partnered_content":choice}))
            .website_url(WEBSITE)
            .expect("URL");
        assert!(
            url.query_pairs()
                .any(|(k, v)| k == "filter-partnered-content" && v == expected)
        );
    }
}

/// Checks aggregate and byte limits and the full observed CR mapping gap.
#[test]
fn selector_bounds_and_cr_mapping_are_exact() {
    let serialized = serde_json::to_value(MonsterQuery::default()).expect("query");
    assert!(
        serde_json::from_value::<MonsterQuery>(serialized)
            .expect("round trip")
            .validate()
            .is_ok()
    );
    let mut value = json!({});
    for (field, _, _) in query::ID_FILTERS.iter().take(6) {
        value[*field] = json!((1..=50).collect::<Vec<_>>());
    }
    assert!(query(value).validate().is_err());
    let mut boundary = json!({});
    for (field, _, _) in query::ID_FILTERS {
        boundary[field] = json!((1..=15).collect::<Vec<_>>());
    }
    boundary["type_ids"] = json!((1..=16).collect::<Vec<_>>());
    assert!(query(boundary.clone()).validate().is_ok());
    boundary["type_ids"] = json!((1..=17).collect::<Vec<_>>());
    assert!(query(boundary).validate().is_err());
    assert!(
        query(json!({"source_ids":(1..=50).collect::<Vec<_>>()}))
            .validate()
            .is_ok()
    );
    for (literal, id) in query::CHALLENGE_RATINGS {
        let q = query(json!({"cr_min":literal,"cr_max":literal}));
        q.validate().expect("rating");
        assert!(
            q.website_url(WEBSITE)
                .expect("URL")
                .query_pairs()
                .any(|(key, value)| key == "filter-cr-min" && value == id.to_string())
        );
    }
    for url in [
        "https://www.dndbeyond.com/sources/example?filter-type=0",
        "https://www.dndbeyond.com/monsters?unknown=1",
        "https://www.dndbeyond.com/monsters?page=0",
        "https://www.dndbeyond.com/monsters?page=01",
        "https://www.dndbeyond.com/monsters?page=1#fragment",
    ] {
        assert!(!query::allowed_list_url(
            &reqwest::Url::parse(url).expect("URL")
        ));
    }
    let url = format!(
        "https://www.dndbeyond.com/monsters?filter-search={}",
        "x".repeat(query::MAX_QUERY_BYTES)
    );
    assert!(!query::allowed_list_url(
        &reqwest::Url::parse(&url).expect("URL")
    ));
}

/// Requires the known form, explicit empty marker, exact identities, and matching paging.
#[test]
fn malformed_website_pages_fail_closed() {
    let q = MonsterQuery::default();
    let (_, body) = fixture::website(
        Scenario::Normal,
        "/monsters?page=1&filter-type=0&filter-partnered-content=f",
    );
    assert!(website::parse(body.as_bytes(), &q).is_ok());
    for mutated in [
        body.replace("listing-body", "unknown-body"),
        body.replace("listing-footer", "missing-footer"),
        body.replace("method=\"get\"", "method=\"post\""),
        body.replace(
            "<select name=\"filter-partnered-content\">",
            "<input name=\"filter-partnered-content\"><select name=\"unrelated\">",
        ),
        body.replace(
            "<select name=\"filter-partnered-content\">",
            "<div name=\"filter-partnered-content\" type=\"checkbox\" value=\"f\"></div><select name=\"unrelated\">",
        ),
        body.replace("/monsters/18-invented", "/monsters/17-invented"),
        body.replace(
            "/monsters/17-invented",
            "https://evil.example/monsters/17-invented",
        ),
        body.replace("page=2", "page=3"),
        body.replace("rel=\"next\"", "rel=\"unknown\""),
        body.replace("page=2", "page=2&amp;filter-search=changed"),
        body.replace("active\">1", "active\">2"),
        body.replace("</ul></div>", "<li></li></ul></div>"),
    ] {
        assert!(website::parse(mutated.as_bytes(), &q).is_err());
    }
    let (_, empty) = fixture::website(Scenario::Empty, "/monsters?page=1");
    let with_metadata = body.replace(
        "</a></div>",
        "</a><a href=\"/sources/invented\">Source metadata</a></div>",
    );
    assert!(website::parse(with_metadata.as_bytes(), &q).is_ok());
    for mutated in [
        empty.replace("alert no-results", "unknown"),
        empty.replace("You failed your Investigation check.", "Unexpected error"),
    ] {
        assert!(website::parse(mutated.as_bytes(), &q).is_err());
    }
    let unsupported = query(json!({"size_ids":[99]}));
    assert!(matches!(
        website::parse(body.as_bytes(), &unsupported),
        Err(MonsterError::InvalidInput)
    ));
    let (_, unsupported_page) = fixture::website(
        Scenario::Normal,
        "/monsters?page=1&filter-type=0&filter-partnered-content=f&filter-size=99",
    );
    let missing_options = unsupported_page.replace(
        "<select multiple name=\"filter-size\">",
        "<input name=\"filter-size\"><select multiple name=\"unrelated\">",
    );
    assert!(matches!(
        website::parse(missing_options.as_bytes(), &unsupported),
        Err(MonsterError::InvalidResponse)
    ));
    let (_, maximum) = fixture::website(
        Scenario::WideTable,
        "/monsters?page=1&filter-type=0&filter-partnered-content=f",
    );
    assert_eq!(
        website::parse(maximum.as_bytes(), &q)
            .expect("exact record ceiling")
            .ids
            .len(),
        MAX_PAGE_RECORDS
    );
    let oversized = maximum.replace("</ul></div>", "<li></li></ul></div>");
    assert!(matches!(
        website::parse(oversized.as_bytes(), &q),
        Err(MonsterError::LimitExceeded)
    ));
}

/// Keeps HTTP denial, rejected/expired sessions, grants, malformed HTML, and bounds distinct.
#[tokio::test]
async fn source_failures_never_return_partial_success() {
    for scenario in [
        Scenario::Denied,
        Scenario::Rejected,
        Scenario::Expired,
        Scenario::DeniedCode,
        Scenario::UnsupportedCode,
        Scenario::SharedCode,
        Scenario::MixedAccess,
        Scenario::MissingAccess,
        Scenario::Malformed,
        Scenario::Incomplete,
        Scenario::Oversized,
    ] {
        let session = fixture::session(scenario).expect("fixture");
        let error = list(&session, &MonsterQuery::default())
            .await
            .expect_err("failure");
        match scenario {
            Scenario::Denied => assert!(
                matches!(error, MonsterError::SessionRead(ref e) if e.kind() == SessionErrorKind::AccessDenied)
            ),
            Scenario::Rejected => assert!(
                matches!(error, MonsterError::SessionRead(ref e) if e.kind() == SessionErrorKind::Rejected)
            ),
            Scenario::Expired => assert!(
                matches!(error, MonsterError::SessionRead(ref e) if e.kind() == SessionErrorKind::Expired)
            ),
            Scenario::DeniedCode | Scenario::MixedAccess => {
                assert!(matches!(error, MonsterError::AccessDenied))
            }
            Scenario::UnsupportedCode | Scenario::SharedCode => {
                assert!(matches!(error, MonsterError::UnsupportedAccess))
            }
            Scenario::Incomplete => assert!(matches!(error, MonsterError::IncompletePage)),
            Scenario::Oversized => assert!(matches!(error, MonsterError::SessionRead(_))),
            _ => assert!(matches!(error, MonsterError::InvalidResponse)),
        }
        assert!(fixture::routes(&session).len() <= 2);
        assert!(!format!("{error:?}").contains("Synthetic action"));
    }
    let session = fixture::session(Scenario::Normal).expect("fixture");
    assert!(matches!(
        list(&session, &query(json!({"size_ids":[99]}))).await,
        Err(MonsterError::InvalidInput)
    ));
    let routes = fixture::routes(&session);
    assert_eq!(routes.len(), 1);
    assert!(routes[0].starts_with("/monsters?"));
}

/// Rejects incomplete, extra, duplicate, or mismatched collection records and access maps.
#[test]
fn selected_collection_requires_an_exact_set_and_fresh_grants() {
    let ids = vec!["17".to_owned(), "18".to_owned()];
    let valid = fixture::collection(vec![fixture::record(18), fixture::record(17)]);
    assert_eq!(
        parse_collection(valid.clone(), &ids, 1, None).expect("ordered")["entries"][0]["id"],
        "17"
    );
    let mut missing = valid.clone();
    missing["data"].as_array_mut().expect("data").pop();
    let mut extra = valid.clone();
    extra["data"]
        .as_array_mut()
        .expect("data")
        .push(fixture::record(19));
    let mut duplicate = valid.clone();
    duplicate["data"][1] = fixture::record(18);
    let mut mismatch = valid.clone();
    mismatch["data"][1] = fixture::record(19);
    mismatch["accessType"] = json!({"18":1,"19":1});
    let mut access = valid.clone();
    access["accessType"] = json!({"17":1,"99":1});
    for invalid in [missing, extra, duplicate, mismatch, access] {
        assert!(parse_collection(invalid, &ids, 1, None).is_err());
    }
    let entries = (1..=MAX_PAGE_RECORDS + 1)
        .map(|id| fixture::record(id as u64))
        .collect();
    assert!(matches!(
        parse_collection(fixture::collection(entries), &ids, 1, None),
        Err(MonsterError::LimitExceeded)
    ));
    for code in [0, 2, 3, 5, u64::MAX] {
        assert!(matches!(
            super::access(Some(&json!(code))),
            Err(MonsterError::UnsupportedAccess)
        ));
    }
    for code in [Value::Null, json!("1"), json!(-1), json!(1.5)] {
        assert!(matches!(
            super::access(Some(&code)),
            Err(MonsterError::InvalidResponse)
        ));
    }
}

/// Detail rechecks its own grant even after a successful filtered list.
#[tokio::test]
async fn detail_access_is_checked_independently() {
    let session = fixture::session(Scenario::AccessLost).expect("fixture");
    list(&session, &MonsterQuery::default())
        .await
        .expect("earlier grant");
    assert!(matches!(
        show(&session, "17").await,
        Err(MonsterError::AccessDenied)
    ));
    assert_eq!(fixture::routes(&session).len(), 3);
}

/// Admits exact transport bounds without increasing the shared cap.
#[tokio::test]
async fn exact_transport_body_bound_is_accepted() {
    let session = fixture::session(Scenario::ExactBody).expect("fixture");
    list(&session, &MonsterQuery::default())
        .await
        .expect("exact page");
    show(&session, "17").await.expect("exact detail");
    assert_eq!(fixture::routes(&session).len(), 3);
}

/// Checks fixed origins, URL identity, and malformed names before output.
#[test]
fn links_never_become_fetch_routes_or_unchecked_attribution() {
    for url in [
        "https://evil.example/monsters/17-invented",
        "http://www.dndbeyond.com/monsters/17-invented",
        "https://user@www.dndbeyond.com/monsters/17-invented",
        "https://www.dndbeyond.com/monsters/18-invented",
        "https://www.dndbeyond.com/monsters/170",
        "https://www.dndbeyond.com/monsters/17-invented?q=x",
        "https://www.dndbeyond.com/monsters/17-invented#fragment",
        "https://www.dndbeyond.com/monsters/17-inv\nented",
        "https://www.dndbeyond.com/monsters/17-old/../17-invented",
        "https://www.dndbeyond.com/monsters/17-old\\..\\17-invented",
    ] {
        let mut record = fixture::record(17);
        record["url"] = json!(url);
        assert!(summary(&record).is_err());
    }
    let mut record = fixture::record(17);
    record["name"] = json!("padded ");
    assert!(summary(&record).is_err());
}

/// Measures exact output-byte and synchronous deadline thresholds.
#[test]
fn exact_result_bytes_and_whole_deadline_are_enforced() {
    assert!(finish(json!("x".repeat(MAX_RESULT_BYTES - 2)), Instant::now()).is_ok());
    assert!(matches!(
        finish(json!("x".repeat(MAX_RESULT_BYTES - 1)), Instant::now()),
        Err(MonsterError::LimitExceeded)
    ));
    assert!(matches!(
        finish(json!({}), Instant::now() - DEADLINE),
        Err(MonsterError::TimedOut)
    ));
}
