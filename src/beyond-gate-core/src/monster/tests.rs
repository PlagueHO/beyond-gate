//! Offline metadata search, exact pagination, fresh detail access, and output bounds.

use super::{fixture::Scenario, *};

/// Builds strict queries from invented JSON, with no production session access.
fn query(value: Value) -> MonsterQuery {
    serde_json::from_value(value).expect("invented query")
}

/// Resolves synthetic lookup data with the full result allowance.
fn resolve_detail(raw: &Value, configuration: &Value) -> Result<Value, MonsterError> {
    resolve::detail(raw, configuration, MAX_RESULT_BYTES)
}

/// Preserves API order and follows offsets only when the caller explicitly requests them.
#[tokio::test]
async fn batches_and_detail_preserve_shared_contracts() {
    let session = fixture::session(Scenario::Normal).expect("fixture");
    let first = list(
        &session,
        &query(json!({"name":"beast & take=100","take":2})),
    )
    .await
    .expect("first");
    assert_eq!(first["entries"][0]["id"], "17");
    assert_eq!(first["entries"][1]["id"], "18");
    assert_eq!(
        first["pagination"],
        json!({"skip":0,"take":2,"total":3,"next_skip":2})
    );
    assert!(first["entries"][0].get("raw").is_none());
    assert_eq!(
        fixture::routes(&session),
        ["/v1/Monster?skip=0&take=2&showHomebrew=f&search=beast+%26+take%3D100"]
    );
    let second = list(&session, &query(json!({"skip":2,"take":2})))
        .await
        .expect("second");
    assert_eq!(second["entries"][0]["id"], "19");
    assert_eq!(
        second["pagination"],
        json!({"skip":2,"take":2,"total":3,"next_skip":null})
    );
    let detail = show(&session, "17", false).await.expect("detail");
    assert_eq!(detail["access"], "available");
    assert_eq!(detail["access_code"], 1);
    assert_eq!(detail["raw"], fixture::record(17));
    assert_eq!(fixture::routes(&session).len(), 4);
    assert_eq!(
        &fixture::routes(&session)[2..],
        ["/v1/Monster/17", "/api/config/json"]
    );
    assert_eq!(
        detail["resolved"]["stats"][0],
        json!({
            "statId":1,"value":12,"future_stat":null,
            "name":"Invented ability","lookup_status":"resolved"
        })
    );
    assert_eq!(
        detail["resolved"]["alignment"],
        json!({
            "id":8,"name":"Invented alignment","lookup_status":"resolved"
        })
    );
    assert_eq!(detail["resolved"]["challenge_rating"]["name"], "1/4");
    for (field, name) in [("size", "Invented size"), ("type", "Invented type")] {
        assert_eq!(detail["resolved"][field]["name"], name);
    }
    for (field, name) in [
        ("condition_immunities", "Invented condition"),
        ("damage_adjustments", "Invented adjustment"),
        ("environments", "Invented environment"),
        ("languages", "Invented language"),
        ("movements", "Invented movement"),
        ("saving_throws", "Invented save"),
        ("senses", "Invented sense"),
        ("skills", "Invented skill"),
        ("subtypes", "Invented subtype"),
    ] {
        assert_eq!(detail["resolved"][field][0]["name"], name, "{field}");
    }
    assert_eq!(
        detail["resolved"]["environments"][1],
        json!({
            "id":999,"name":null,"lookup_status":"unresolved"
        })
    );
    assert_eq!(detail["resolved"]["movements"][0]["speed"], 20);
    assert_eq!(detail["resolved"]["skills"][0]["value"], 4);
    assert!(detail["resolved"].get("sources").is_none());
    assert!(!detail["resolved"].to_string().contains("Not a label"));
    let default = list(&session, &MonsterQuery::default())
        .await
        .expect("default");
    assert_eq!(default["entries"].as_array().expect("entries").len(), 3);
    assert_eq!(
        default["pagination"],
        json!({"skip":0,"take":10,"total":3,"next_skip":null})
    );
}

/// Lists metadata without promoting reported access codes to selected-detail grants.
#[tokio::test]
async fn metadata_lists_preserve_access_codes_without_granting_detail() {
    for (scenario, codes, labels) in [
        (Scenario::DeniedCode, [4, 4, 4], ["stat_block_locked"; 3]),
        (Scenario::UnsupportedCode, [2, 2, 2], ["available"; 3]),
        (Scenario::SharedCode, [3, 3, 3], ["available"; 3]),
        (
            Scenario::MixedAccess,
            [1, 4, 1],
            ["available", "stat_block_locked", "available"],
        ),
    ] {
        let session = fixture::session(scenario).expect("fixture");
        let result = list(&session, &MonsterQuery::default())
            .await
            .expect("metadata page");
        let entries: Vec<_> = (17..=19)
            .zip(codes)
            .zip(labels)
            .map(|((id, code), label)| {
                json!({
                    "id":id.to_string(), "name":"Invented Beast",
                    "source_url":format!("https://www.dndbeyond.com/monsters/{id}-invented"),
                    "access_code":code, "access":label
                })
            })
            .collect();
        assert_eq!(
            result,
            json!({
                "entries":entries,
                "pagination":{"skip":0,"take":10,"total":3,"next_skip":null}
            })
        );
        let error = show(&session, "17", false)
            .await
            .expect_err("detail stays protected");
        match scenario {
            Scenario::DeniedCode | Scenario::MixedAccess => {
                assert!(matches!(error, MonsterError::AccessDenied));
            }
            _ => assert!(matches!(error, MonsterError::UnsupportedAccess)),
        }
        assert_eq!(
            fixture::routes(&session),
            [
                "/v1/Monster?skip=0&take=10&showHomebrew=f",
                "/v1/Monster/17"
            ]
        );
    }
}

/// Accepts consistent JSON emptiness while preserving totals beyond the final offset.
#[tokio::test]
async fn empty_missing_and_mismatched_results_are_distinct() {
    for (scenario, total) in [(Scenario::Empty, 0), (Scenario::Normal, 3)] {
        let session = fixture::session(scenario).expect("fixture");
        let value = list(&session, &query(json!({"skip":20})))
            .await
            .expect("empty");
        assert_eq!(
            value,
            json!({
                "entries":[], "pagination":{"skip":20,"take":10,"total":total,"next_skip":null}
            })
        );
        assert_eq!(fixture::routes(&session).len(), 1);
        assert!(matches!(
            show(&session, "99", false).await,
            Err(MonsterError::MissingRecord)
        ));
    }
    let changed = fixture::session(Scenario::ChangedIdentity).expect("fixture");
    assert!(matches!(
        show(&changed, "17", false).await,
        Err(MonsterError::InvalidResponse)
    ));
}

/// Rejects invalid ranges, array values, text, removed selectors, and limits before auth.
#[tokio::test]
async fn invalid_input_never_touches_the_session() {
    let session = fixture::session(Scenario::Normal).expect("fixture");
    for value in [
        json!({"take":0}),
        json!({"take":101}),
        json!({"name":""}),
        json!({"name":" padded"}),
        json!({"name":"line\nbreak"}),
        json!({"name":"x".repeat(257)}),
        json!({"cr_min":"0.5"}),
        json!({"cr_min":"2","cr_max":"1/2"}),
        json!({"armor_class_min":10,"armor_class_max":9}),
        json!({"average_hp_min":2,"average_hp_max":1}),
        json!({"size_min_id":1}),
        json!({"size_max_id":8}),
        json!({"size_min_id":5,"size_max_id":4}),
        json!({"type_ids":[0]}),
        json!({"source_ids":[1,1]}),
        json!({"type_ids":(1..=51).collect::<Vec<_>>()}),
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
            show(&session, id, false).await,
            Err(MonsterError::InvalidInput)
        ));
    }
    for value in [
        json!({"page":1}),
        json!({"size_ids":[4]}),
        json!({"mythic":false}),
        json!({"partnered_content":"no"}),
        json!({"unknown":true}),
        json!({"name":null}),
        json!({"skip":null}),
        json!({"skip":-1}),
        json!({"skip":u64::MAX}),
        json!({"take":1.5}),
        json!({"take":null}),
        json!({"size_min_id":null}),
        json!({"size_max_id":null}),
        json!({"source_ids":null}),
        json!({"source_ids":[-1]}),
        json!({"legendary":null}),
        json!({"legendary":"false"}),
        json!({"armor_class_min":null}),
        json!({"cr_min":null}),
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

/// Sends all selectors in one JSON request with independent expected service keys.
#[tokio::test]
async fn all_filter_fields_reach_the_json_service() {
    let mut value = json!({
        "name":"beast & skip=99","skip":0,"take":2,"cr_min":"1/8","cr_max":"24",
        "size_min_id":2,"size_max_id":7,
        "armor_class_min":0,"armor_class_max":99,"average_hp_min":1,"average_hp_max":1000,
        "legendary":false,"has_lair":true
    });
    let selectors = [
        ("type_ids", "monsterTypes"),
        ("subtype_ids", "monsterSubtypes"),
        ("environment_ids", "environments"),
        ("alignment_ids", "alignments"),
        ("sense_ids", "senses"),
        ("save_proficiency_ids", "saveProficiencies"),
        ("skill_proficiency_ids", "skillProficiencies"),
        ("resistance_ids", "resistances"),
        ("damage_immunity_ids", "damageImmunities"),
        ("condition_immunity_ids", "conditionImmunities"),
        ("vulnerability_ids", "vulnerabilities"),
        ("language_ids", "languages"),
        ("movement_ids", "movementTypes"),
        ("tag_ids", "monsterTags"),
        ("source_category_ids", "sourceCategories"),
        ("source_ids", "sources"),
    ];
    for (index, (field, _)) in selectors.iter().enumerate() {
        value[*field] = json!([index + 1, index + 101]);
    }
    let session = fixture::session(Scenario::Normal).expect("fixture");
    list(&session, &query(value)).await.expect("all filters");
    let routes = fixture::routes(&session);
    assert_eq!(routes.len(), 1);
    let url = reqwest::Url::parse(&format!("http://127.0.0.1{}", routes[0])).expect("query");
    assert_eq!(url.path(), "/v1/Monster");
    let pairs: Vec<_> = url.query_pairs().collect();
    for (index, (_, field)) in selectors.iter().enumerate() {
        assert_eq!(
            pairs
                .iter()
                .filter(|(k, _)| k == *field)
                .map(|(_, v)| v.as_ref())
                .collect::<Vec<_>>(),
            [(index + 1).to_string(), (index + 101).to_string()],
            "{field}"
        );
    }
    for (field, expected) in [
        ("skip", "0"),
        ("take", "2"),
        ("showHomebrew", "f"),
        ("search", "beast & skip=99"),
        ("challengeRatingMin", "2"),
        ("challengeRatingMax", "29"),
        ("monsterSizeMin", "2"),
        ("monsterSizeMax", "7"),
        ("armorClassMin", "0"),
        ("armorClassMax", "99"),
        ("averageHpMin", "1"),
        ("averageHpMax", "1000"),
        ("isLegendary", "f"),
        ("hasLair", "t"),
    ] {
        assert!(
            pairs.iter().any(|(k, v)| k == field && v == expected),
            "{field}"
        );
    }
    assert_eq!(pairs.len(), 46);
    let defaults = MonsterQuery::default().service_url(SERVICE).expect("URL");
    assert_eq!(
        defaults.as_str(),
        "https://monster-service.dndbeyond.com/v1/Monster?skip=0&take=10&showHomebrew=f"
    );
}

/// Checks aggregate and byte limits and the full source-verified CR mapping gap.
#[test]
fn selector_bounds_and_cr_mapping_are_exact() {
    let serialized = serde_json::to_value(MonsterQuery::default()).expect("query");
    serde_json::from_value::<MonsterQuery>(serialized)
        .expect("round trip")
        .validate()
        .expect("valid");
    let mut boundary = json!({});
    for (field, _, _, _) in query::ID_FILTERS {
        boundary[field] = json!((1..=16).collect::<Vec<_>>());
    }
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
        let url = q.service_url(SERVICE).expect("URL");
        assert!(
            url.query_pairs()
                .any(|(k, v)| k == "challengeRatingMin" && v == id.to_string())
        );
    }
    assert!(
        query(json!({"size_min_id":2,"size_max_id":7,"skip":u32::MAX,"take":100}))
            .validate()
            .is_ok()
    );
}

/// Keeps HTTP denial, rejected/expired sessions, grants, malformed JSON, and bounds distinct.
#[tokio::test]
async fn source_failures_never_return_partial_success_or_fallback() {
    for scenario in [
        Scenario::Denied,
        Scenario::Rejected,
        Scenario::Expired,
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
            Scenario::Incomplete => assert!(matches!(error, MonsterError::IncompletePage)),
            Scenario::Oversized => assert!(matches!(error, MonsterError::SessionRead(_))),
            _ => assert!(matches!(error, MonsterError::InvalidResponse)),
        }
        let routes = fixture::routes(&session);
        assert!(routes.len() <= 1);
        assert!(
            routes
                .iter()
                .all(|route| route == "/v1/Monster?skip=0&take=10&showHomebrew=f")
        );
        assert!(!format!("{error:?}").contains("Synthetic action"));
    }
}

/// Rejects malformed source fields, mismatched counts, and invented continuation.
#[test]
fn pagination_requires_exact_complete_batches_and_checked_offsets() {
    let valid = fixture::collection(vec![fixture::record(17), fixture::record(18)]);
    let q = MonsterQuery::default();
    for pagination in [
        Value::Null,
        json!({}),
        json!([]),
        json!({"skip":1,"take":10,"total":2}),
        json!({"skip":0,"take":9,"total":2}),
        json!({"skip":0,"take":10,"total":1}),
        json!({"skip":0,"take":10,"total":3}),
        json!({"skip":0,"take":10,"total":-1}),
        json!({"skip":0,"take":10,"total":2.5}),
        json!({"skip":0,"take":10,"total":"2"}),
        json!({"skip":0,"take":10,"total":u64::from(u32::MAX)+1}),
        json!({"skip":u64::MAX,"take":10,"total":2}),
        json!({"skip":0,"take":u64::MAX,"total":2}),
    ] {
        let mut value = valid.clone();
        value["pagination"] = pagination;
        assert!(parse_collection(value, &q).is_err());
    }
    for (skip, take, total, count, next) in [
        (0, 2, 5, 2, Some(2)),
        (2, 2, 5, 2, Some(4)),
        (4, 2, 5, 1, None),
        (5, 2, 5, 0, None),
        (6, 2, 5, 0, None),
        (0, 10, 0, 0, None),
        (u32::MAX - 2, 1, u32::MAX, 1, Some(u32::MAX - 1)),
        (u32::MAX - 1, 100, u32::MAX, 1, None),
        (u32::MAX, 100, u32::MAX, 0, None),
    ] {
        let mut value = fixture::collection((17..17 + count).map(fixture::record).collect());
        value["pagination"] = json!({"skip":skip,"take":take,"total":total});
        let output =
            parse_collection(value, &query(json!({"skip":skip,"take":take}))).expect("page");
        assert_eq!(
            output["pagination"],
            json!({"skip":skip,"take":take,"total":total,"next_skip":next})
        );
    }
}

/// Requires exact numeric access keys without filtering rows or granting detail access.
#[test]
fn collection_requires_exact_identities_and_reported_access_codes() {
    let valid = fixture::collection(vec![fixture::record(18), fixture::record(17)]);
    let q = MonsterQuery::default();
    assert_eq!(
        parse_collection(valid.clone(), &q).expect("source order")["entries"][0]["id"],
        "18"
    );
    for grants in [
        json!({"17":1}),
        json!({"17":1,"99":1}),
        json!({"17":1,"18":1,"19":1}),
        Value::Null,
    ] {
        let mut value = valid.clone();
        value["accessType"] = grants;
        assert!(parse_collection(value, &q).is_err());
    }
    let mut duplicate = valid.clone();
    duplicate["data"][1] = fixture::record(18);
    assert!(matches!(
        parse_collection(duplicate, &q),
        Err(MonsterError::ConflictingIdentity)
    ));
    for (code, label) in [
        (0, "unknown"),
        (1, "available"),
        (2, "available"),
        (3, "available"),
        (4, "stat_block_locked"),
        (5, "unknown"),
        (u64::MAX, "unknown"),
    ] {
        let mut mixed = valid.clone();
        mixed["accessType"]["17"] = json!(code);
        let result = parse_collection(mixed, &q).expect("reported numeric code");
        assert_eq!(result["entries"][1]["access_code"], code);
        assert_eq!(result["entries"][1]["access"], label);
        assert!(result["entries"][1].get("raw").is_none());
        if code != 1 {
            assert!(
                parse_detail(json!({"data":fixture::record(17), "accessType":code}), "17").is_err()
            );
        }
    }
    for code in [Value::Null, json!("1"), json!(-1), json!(1.5), json!(true)] {
        let mut mixed = valid.clone();
        mixed["accessType"]["17"] = code.clone();
        assert!(matches!(
            parse_collection(mixed, &q),
            Err(MonsterError::InvalidResponse)
        ));
        assert!(matches!(
            detail_access(Some(&code)),
            Err(MonsterError::InvalidResponse)
        ));
    }
    let mut empty = fixture::collection(vec![]);
    empty["accessType"] = json!({"17":1});
    assert!(parse_collection(empty, &q).is_err());
    let maximum = fixture::collection((1..=100).map(fixture::record).collect());
    assert!(parse_collection(maximum, &query(json!({"take":100}))).is_err());
    let mut maximum = fixture::collection((1..=100).map(fixture::record).collect());
    maximum["pagination"]["take"] = json!(100);
    assert_eq!(
        parse_collection(maximum.clone(), &query(json!({"take":100}))).expect("ceiling")["entries"]
            .as_array()
            .expect("entries")
            .len(),
        100
    );
    maximum["data"]
        .as_array_mut()
        .expect("data")
        .push(fixture::record(101));
    assert!(matches!(
        parse_collection(maximum, &query(json!({"take":100}))),
        Err(MonsterError::LimitExceeded)
    ));
}

/// Detail rechecks its own grant even after a successful metadata list.
#[tokio::test]
async fn detail_access_is_checked_independently() {
    let session = fixture::session(Scenario::AccessLost).expect("fixture");
    list(&session, &MonsterQuery::default())
        .await
        .expect("earlier grant");
    assert!(matches!(
        show(&session, "17", false).await,
        Err(MonsterError::AccessDenied)
    ));
    assert_eq!(fixture::routes(&session).len(), 2);
}

/// Fails explicitly on public lookup errors without returning partial raw detail.
#[tokio::test]
async fn lookup_failures_keep_detail_atomic_and_do_not_affect_lists() {
    for scenario in [
        Scenario::ConfigurationDenied,
        Scenario::InvalidConfiguration,
    ] {
        let session = fixture::session(scenario).expect("fixture");
        list(&session, &MonsterQuery::default())
            .await
            .expect("metadata");
        let error = show(&session, "17", false)
            .await
            .expect_err("lookup failure");
        assert!(match scenario {
            Scenario::ConfigurationDenied => matches!(error, MonsterError::SourceRead(_)),
            _ => matches!(error, MonsterError::InvalidConfiguration),
        });
        assert_eq!(
            fixture::routes(&session),
            [
                "/v1/Monster?skip=0&take=10&showHomebrew=f",
                "/v1/Monster/17",
                "/api/config/json"
            ]
        );
    }
}

/// Raw mode returns the authorized envelope without consulting lookup configuration.
#[tokio::test]
async fn raw_detail_skips_lookup_reads_and_preserves_source_fields() {
    for scenario in [
        Scenario::Normal,
        Scenario::ConfigurationDenied,
        Scenario::InvalidConfiguration,
    ] {
        let session = fixture::session(scenario).expect("fixture");
        let result = show(&session, "17", true).await.expect("raw detail");
        assert_eq!(
            result,
            json!({
                "id":"17","name":"Invented Beast",
                "source_url":"https://www.dndbeyond.com/monsters/17-invented",
                "access":"available","access_code":1,"raw":fixture::record(17)
            })
        );
        assert_eq!(fixture::routes(&session), ["/v1/Monster/17"]);
    }
}

/// Disabling lookup mapping never bypasses selected access or identity checks.
#[tokio::test]
async fn raw_detail_keeps_access_identity_and_input_checks() {
    for scenario in [
        Scenario::DeniedCode,
        Scenario::UnsupportedCode,
        Scenario::SharedCode,
        Scenario::ChangedIdentity,
    ] {
        let session = fixture::session(scenario).expect("fixture");
        let error = show(&session, "17", true)
            .await
            .expect_err("protected detail");
        assert!(match scenario {
            Scenario::DeniedCode => matches!(error, MonsterError::AccessDenied),
            Scenario::ChangedIdentity => matches!(error, MonsterError::InvalidResponse),
            _ => matches!(error, MonsterError::UnsupportedAccess),
        });
        assert_eq!(fixture::routes(&session), ["/v1/Monster/17"]);
    }
    let session = fixture::session(Scenario::Normal).expect("fixture");
    assert!(matches!(
        show(&session, "01", true).await,
        Err(MonsterError::InvalidInput)
    ));
    assert!(fixture::routes(&session).is_empty());
}

/// Keeps unknown, empty, absent, and nullable references distinct without source-ID mapping.
#[test]
fn unknown_lookup_ids_are_not_guessed_or_dropped() {
    let raw = json!({
        "alignmentId":999,"stats":[{"statId":999,"value":8}],
        "languages":[],"environments":null,"sourceId":7,"entityTypeId":123,
        "futureId":77
    });
    let before = raw.clone();
    assert_eq!(
        resolve_detail(&raw, &fixture::configuration()).expect("resolve"),
        json!({
            "alignment":{"id":999,"name":null,"lookup_status":"unresolved"},
            "stats":[{"statId":999,"value":8,"name":null,"lookup_status":"unresolved"}],
            "languages":[],"environments":null
        })
    );
    assert_eq!(raw, before);
    assert_eq!(
        resolve_detail(&json!({}), &json!({})).expect("absent"),
        json!({})
    );
}

/// Reads configured CR values rather than treating service IDs as CR ordinals.
#[test]
fn challenge_rating_labels_use_configured_values() {
    for (id, value, expected) in [
        (3, 0.25, "1/4"),
        (29, 24.0, "24"),
        (777, 0.125, "1/8"),
        (888, 0.5, "1/2"),
        (999, 0.0, "0"),
    ] {
        let resolved = resolve_detail(
            &json!({"challengeRatingId":id}),
            &json!({"challengeRatings":[{"id":id,"value":value}]}),
        )
        .expect("configured CR");
        assert_eq!(resolved["challenge_rating"]["name"], expected);
    }
}

/// Rejects malformed references and all malformed rows in a selected lookup table.
#[test]
fn malformed_lookup_data_fails_instead_of_becoming_unknown() {
    for raw in [
        json!({"alignmentId":"8"}),
        json!({"sizeId":-1}),
        json!({"typeId":1.5}),
        json!({"stats":[{}]}),
        json!({"stats":[{"statId":null}]}),
        json!({"languages":[17]}),
        json!({"environments":false}),
        json!({"conditionImmunities":[-1]}),
    ] {
        assert!(
            matches!(
                resolve_detail(&raw, &fixture::configuration()),
                Err(MonsterError::InvalidResponse)
            ),
            "{raw}"
        );
    }
    for configuration in [
        json!(null),
        json!({}),
        json!({"stats":{}}),
        json!({"stats":[{"id":1}]}),
        json!({"stats":[{"id":"1","name":"Invalid"}]}),
        json!({"stats":[{"id":1,"name":" padded"}]}),
        json!({"stats":[{"id":1,"name":"Valid"},{"id":1,"name":"Duplicate"}]}),
        json!({"stats":[{"id":1,"name":"Valid"},{"id":999,"name":null}]}),
    ] {
        assert!(
            matches!(
                resolve_detail(&json!({"stats":[{"statId":1}]}), &configuration),
                Err(MonsterError::InvalidConfiguration)
            ),
            "{configuration}"
        );
    }
    assert!(matches!(
        resolve_detail(
            &json!({"conditionImmunities":[4]}),
            &json!({"conditions":[{"id":4,"name":"Wrong shape"}]})
        ),
        Err(MonsterError::InvalidConfiguration)
    ));
    assert!(matches!(
        resolve_detail(
            &json!({"challengeRatingId":3}),
            &json!({"challengeRatings":[{"id":3,"value":-1}]})
        ),
        Err(MonsterError::InvalidConfiguration)
    ));
}

/// Measures reference and lookup-table ceilings independently of transport size.
#[test]
fn lookup_counts_are_bounded() {
    let mut raw = json!({"stats":vec![json!({"statId":1,"value":12});1_000]});
    assert_eq!(
        resolve_detail(&raw, &fixture::configuration()).expect("ceiling")["stats"]
            .as_array()
            .expect("references")
            .len(),
        1_000
    );
    raw["stats"]
        .as_array_mut()
        .expect("references")
        .push(json!({"statId":1}));
    assert!(matches!(
        resolve_detail(&raw, &fixture::configuration()),
        Err(MonsterError::LimitExceeded)
    ));
    let entries: Vec<_> = (1..=1_000)
        .map(|id| json!({"id":id,"name":"Invented"}))
        .collect();
    let mut config = json!({"stats":entries});
    let raw = json!({"stats":[{"statId":1}]});
    resolve_detail(&raw, &config).expect("lookup ceiling");
    config["stats"]
        .as_array_mut()
        .expect("table")
        .push(json!({"id":1_001,"name":"Invented"}));
    assert!(matches!(
        resolve_detail(&raw, &config),
        Err(MonsterError::LimitExceeded)
    ));
}

/// Rejects repeated large configured labels before they expand across references.
#[test]
fn repeated_large_lookup_labels_cannot_exceed_encoded_budget() {
    let label = "L".repeat(900_000);
    let configuration = json!({"stats":[{"id":1,"name":label}]});
    let raw = json!({"stats":vec![json!({"statId":1});1_000]});

    assert!(matches!(
        resolve::detail(&raw, &configuration, MAX_RESULT_BYTES),
        Err(MonsterError::LimitExceeded)
    ));
}

/// Counts escaped labels and enforces the exact cumulative serialized-size threshold.
#[test]
fn lookup_encoded_budget_counts_escaping_and_exact_thresholds() {
    let raw = json!({"alignmentId":1,"stats":[{"statId":1}]});
    let configuration = json!({
        "alignments":[{"id":1,"name":"quote\" slash\\"}],
        "stats":[{"id":1,"name":"quote\" slash\\"}]
    });
    let resolved = resolve_detail(&raw, &configuration).expect("resolved");
    let exact_size = encoded_len(&resolved).expect("encoded size");
    assert!(exact_size > 2 * "quote\" slash\\".len());
    assert_eq!(
        resolve::detail(&raw, &configuration, exact_size).expect("exact limit"),
        resolved
    );
    assert!(matches!(
        resolve::detail(&raw, &configuration, exact_size - 1),
        Err(MonsterError::LimitExceeded)
    ));
}

/// Reserves the raw envelope bytes before lookup metadata consumes the result budget.
#[test]
fn lookup_budget_includes_envelope_and_raw_json_bytes() {
    let raw = json!({
        "alignmentId":1,
        "padding":"R".repeat(MAX_RESULT_BYTES - 300_000)
    });
    let mut envelope = json!({
        "id":"17",
        "name":"Synthetic",
        "source_url":"https://www.dndbeyond.com/monsters/17",
        "access":"available",
        "access_code":1,
        "raw":raw,
        "resolved":{}
    });
    let base_size = encoded_len(&envelope).expect("placeholder size");
    let resolved_limit = resolution_limit(&envelope).expect("remaining budget");
    assert_eq!(base_size - 2 + resolved_limit, MAX_RESULT_BYTES);

    let short_configuration = json!({"alignments":[{"id":1,"name":"X"}]});
    let short_result = resolve::detail(&raw, &short_configuration, usize::MAX).expect("short");
    let short_size = encoded_len(&short_result).expect("short size");
    let label_size = resolved_limit - (short_size - 1);
    let configuration = json!({
        "alignments":[{"id":1,"name":"N".repeat(label_size)}]
    });
    let resolved = resolve::detail(&raw, &configuration, resolved_limit).expect("exact limit");
    envelope["resolved"] = resolved.clone();
    assert_eq!(
        encoded_len(&envelope).expect("result size"),
        MAX_RESULT_BYTES
    );
    assert!(matches!(
        resolve::detail(&raw, &configuration, resolved_limit - 1),
        Err(MonsterError::LimitExceeded)
    ));
}

/// Admits exact transport bounds without increasing the shared cap.
#[tokio::test]
async fn exact_transport_body_bound_is_accepted() {
    let session = fixture::session(Scenario::ExactBody).expect("fixture");
    list(&session, &MonsterQuery::default())
        .await
        .expect("exact batch");
    show(&session, "17", false).await.expect("exact detail");
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
