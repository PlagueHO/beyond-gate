//! Synthetic contract, transport-budget, and selected-content checks.

use super::{
    fixture::{self, Scenario},
    *,
};
use crate::auth::session::{SessionErrorKind, fixture as auth};
use serde_json::json;

/// Rejects invalid keys and selectors before a session exchange or resource read.
#[tokio::test]
async fn spell_inputs_fail_before_session_access() {
    let session = fixture::session(Scenario::Normal).expect("fixture");
    for slug in [
        "17",
        "0-name",
        "01-name",
        "17-",
        "17-A",
        "17-a--b",
        "17-a/other",
        "17-a%2fb",
        "17-a?x=1",
        "17-a#b",
        " 17-a",
        "https://www.dndbeyond.com/spells/17-a",
    ] {
        assert!(matches!(
            show(&session, slug).await,
            Err(SpellError::InvalidInput)
        ));
    }
    for input in [
        json!({"page":0}),
        json!({"levels":[10]}),
        json!({"levels":[0,0]}),
        json!({"school_ids":[2]}),
        json!({"school_ids":[11]}),
        json!({"class_ids":[0]}),
        json!({"class_ids":[1,1]}),
        json!({"name":" padded"}),
        json!({"name":"x".repeat(257)}),
        json!({"tag_ids":(1..=51).collect::<Vec<_>>()}),
    ] {
        let query: SpellQuery = serde_json::from_value(input).expect("typed input");
        assert!(matches!(
            list(&session, &query).await,
            Err(SpellError::InvalidInput)
        ));
    }
    for input in [
        json!({"name":null}),
        json!({"verbal":null}),
        json!({"levels":null}),
        json!({"page":null}),
        json!({"unknown":1}),
        json!({"partnered_content":null}),
    ] {
        assert!(serde_json::from_value::<SpellQuery>(input).is_err());
    }
    assert_eq!(auth::request_count(&session), 0);
    assert!(fixture::requested_routes(&session).is_empty());
}

/// Preserves website order, cantrips, repeated classes, and explicit continuation.
#[tokio::test]
async fn spell_pages_are_complete_explicit_and_cookie_only() {
    let session = fixture::session(Scenario::Next).expect("fixture");
    let query: SpellQuery = serde_json::from_value(json!({
        "class_ids":[17,18],"levels":[0,9],"school_ids":[3,10],
        "verbal":false,"concentration":true,"ritual":false,
        "name":"Invented"
    }))
    .expect("query");
    let result = list(&session, &query).await.expect("page");
    assert_eq!(result["entries"][0]["id"], "900001");
    assert_eq!(result["entries"][0]["level"], 0);
    assert_eq!(result["entries"][0]["school"], "Abjuration");
    assert_eq!(result["pagination"], json!({"page":1,"next_page":2}));
    assert_eq!(fixture::requested_routes(&session).len(), 1);
    let route = &fixture::requested_routes(&session)[0];
    assert!(route.contains("filter-class=17&filter-class=18"));
    assert!(route.contains("filter-level=0&filter-level=9"));
    assert!(route.contains("filter-concentration=1"));
    assert!(route.contains("filter-ritual=f"));
    assert_eq!(auth::request_count(&session), 1);
    let mut next = query.clone();
    next.page = 2;
    let result = list(&session, &next).await;
    if let Err(SpellError::SessionRead(ref error)) = result {
        panic!(
            "explicit next: {error}; cause: {:?}; routes: {:?}",
            error.source(),
            fixture::requested_routes(&session)
        );
    }
    assert_eq!(result.expect("explicit next")["pagination"]["page"], 2);
    assert_eq!(fixture::requested_routes(&session).len(), 2);
    assert_eq!(auth::request_count(&session), 1);
}

/// Accepts the observed upper school IDs and empty icon elements, not invented text.
#[test]
fn spell_school_icons_use_observed_option_ids() {
    for (id, name) in [(9, "Necromancy"), (10, "Transmutation")] {
        let query = SpellQuery {
            school_ids: vec![id],
            ..SpellQuery::default()
        };
        query.validate().expect("observed school ID");
        let body = fixture::listing(&query, Scenario::Normal);
        let value = website::list(body.as_bytes(), &query).expect("empty icon markup");
        assert_eq!(value["entries"][0]["school"], name);
        let altered = body.replace(
            &format!("school {}", name.to_ascii_lowercase()),
            "school evocation",
        );
        assert!(matches!(
            website::list(altered.as_bytes(), &query),
            Err(SpellError::IncompleteResult)
        ));
    }
}

/// Distinguishes terminal and explicitly empty pages from omitted required rows.
#[tokio::test]
async fn spell_terminal_and_empty_pages_require_the_observed_structure() {
    let query = SpellQuery::default();
    for scenario in [Scenario::Normal, Scenario::Empty] {
        let session = fixture::session(scenario).expect("fixture");
        let result = list(&session, &query).await.expect("valid page");
        assert!(result["pagination"]["next_page"].is_null());
        assert_eq!(
            result["entries"].as_array().expect("entries").len(),
            if matches!(scenario, Scenario::Empty) {
                0
            } else {
                2
            }
        );
    }
    let valid = fixture::listing(&query, Scenario::Normal);
    for altered in [
        valid.replace("data-type='spells'", "data-type='monsters'"),
        valid.replace(
            "data-slug='900001-invented-spell'",
            "data-slug='900099-other'",
        ),
        valid.replace("class='spell-school'", "class='missing-school'"),
        valid.replace(
            "<div class='spell-level'>0</div>",
            "<div class='spell-level'>10</div>",
        ),
        fixture::form(&query)
            + "<ul class='listing listing-rpgspell rpgspell-listing'></ul><div class='listing-footer'></div>",
    ] {
        assert!(website::list(altered.as_bytes(), &query).is_err());
    }
}

/// Rejects unrecognized options, conflicting echoed filters, and inconsistent paging.
#[test]
fn spell_filter_and_paging_checks_never_follow_links() {
    let query: SpellQuery =
        serde_json::from_value(json!({"class_ids":[17],"levels":[0]})).expect("query");
    let valid = fixture::listing(&query, Scenario::Next);
    for changed in [
        valid.replace("value='17' selected", "value='17'"),
        valid.replace("filter-class=17", "filter-class=18"),
        valid.replace("page=2", "page=3"),
        valid.replace(
            "https://www.dndbeyond.com/spells?",
            "https://example.invalid/spells?",
        ),
        valid.replace("rel='next'", "rel='other'"),
        valid.clone()
            + &valid[valid
                .find("<ul class='b-pagination-list'>")
                .expect("paging")..]
                .replace("rel='next'", "rel='other'"),
    ] {
        assert!(website::list(changed.as_bytes(), &query).is_err());
    }
    let query = SpellQuery {
        class_ids: vec![999],
        ..SpellQuery::default()
    };
    assert!(matches!(
        website::list(
            fixture::listing(&query, Scenario::Normal).as_bytes(),
            &query
        ),
        Err(SpellError::InvalidInput)
    ));
}

/// Uses implicit first options for single-select controls, including unrestricted filters.
#[test]
fn spell_filter_checks_follow_single_select_defaults() {
    let query = SpellQuery {
        partnered_content: query::PartneredContent::Any,
        ..SpellQuery::default()
    };
    let body = fixture::listing(&query, Scenario::Normal)
        .replace("<option value='' selected>", "<option value=''>");
    website::list(body.as_bytes(), &query).expect("implicit unrestricted selection");

    let query = SpellQuery::default();
    let body = fixture::listing(&query, Scenario::Normal).replace(
        "<select name='filter-verbal'><option value='' selected>Invented choice</option>",
        "<select name='filter-verbal'>",
    );
    assert!(matches!(
        website::list(body.as_bytes(), &query),
        Err(SpellError::IncompleteResult)
    ));
}

/// Preserves unknown selected elements while excluding surrounding account chrome.
#[tokio::test]
async fn spell_detail_selects_only_checked_untrusted_html() {
    let session = fixture::session(Scenario::Normal).expect("fixture");
    let value = show(&session, "900001-invented-spell")
        .await
        .expect("detail");
    let raw = value["raw_html"].as_str().expect("raw HTML");
    assert!(raw.contains("Unknown invented element"));
    assert!(raw.contains("data-future=\"yes\""));
    assert!(!raw.contains("Outside"));
    assert!(!raw.contains("<form"));
    assert_eq!(value["id"], "900001");
    assert_eq!(
        fixture::requested_routes(&session),
        vec!["/spells/900001-invented-spell"]
    );
    assert_eq!(auth::request_count(&session), 1);
}

/// Fails the whole page or selected detail on typed denial and response failures.
#[tokio::test]
async fn spell_failures_return_no_partial_content_or_retries() {
    for scenario in [
        Scenario::Denied,
        Scenario::Rejected,
        Scenario::Expired,
        Scenario::Malformed,
        Scenario::Incomplete,
        Scenario::Duplicate,
        Scenario::TooManyRecords,
        Scenario::Oversized,
        Scenario::Redirect,
    ] {
        let session = fixture::session(scenario).expect("fixture");
        assert!(list(&session, &SpellQuery::default()).await.is_err());
        assert!(fixture::requested_routes(&session).len() <= 1);
        assert_eq!(auth::request_count(&session), 1);
    }
    for scenario in [
        Scenario::WrongIdentity,
        Scenario::WrongCanonical,
        Scenario::MissingMarkers,
        Scenario::UnsafeDetail,
        Scenario::Unsupported,
        Scenario::LargeResult,
    ] {
        let session = fixture::session(scenario).expect("fixture");
        assert!(show(&session, "900001-invented-spell").await.is_err());
        assert_eq!(fixture::requested_routes(&session).len(), 1);
    }
    let session = fixture::session(Scenario::Missing).expect("fixture");
    assert!(matches!(
        show(&session, "900001-invented-spell").await,
        Err(SpellError::MissingRecord)
    ));
    let session = fixture::session(Scenario::Denied).expect("fixture");
    assert!(matches!(show(&session,"900001-invented-spell").await,
        Err(SpellError::SessionRead(e)) if e.kind()==SessionErrorKind::AccessDenied));
    let session = fixture::session(Scenario::Rejected).expect("fixture");
    assert!(matches!(show(&session,"900001-invented-spell").await,
        Err(SpellError::SessionRead(e)) if e.kind()==SessionErrorKind::Rejected));
}

/// Bounds the entire operation, including its initial session wait.
#[tokio::test(start_paused = true)]
async fn spell_deadline_includes_session_waits() {
    let session = auth::session(auth::Scenario::Waiting).expect("fixture");
    assert!(matches!(
        list(&session, &SpellQuery::default()).await,
        Err(SpellError::TimedOut) | Err(SpellError::SessionRead(_))
    ));
}

/// Keeps book guards unchanged and refuses arbitrary spell routes or query names.
#[test]
fn spell_transport_guard_is_resource_specific() {
    for path in [
        "/spells/900001-invented-spell",
        "/spells?page=1&filter-partnered-content=f",
        "/spells?page=1&filter-level=0&filter-class=17&filter-partnered-content=f",
    ] {
        assert!(query::allowed_url(
            &reqwest::Url::parse(&format!("{WEBSITE}{path}")).expect("URL")
        ));
    }
    for path in [
        "/sources/17",
        "/spells/17",
        "/spells/17-a?x=1",
        "/spells/17-a#x",
        "/spells?page=1&unknown=1",
        "/spells?page=0&filter-partnered-content=f",
        "/spells?page=1&page=2&filter-partnered-content=f",
        "/spells?page=1&filter-level=0&filter-level=0&filter-partnered-content=f",
    ] {
        assert!(!query::allowed_url(
            &reqwest::Url::parse(&format!("{WEBSITE}{path}")).expect("URL")
        ));
    }
    assert!(matches!(
        finish(json!({"raw_html":"\"".repeat(MAX_RESULT_BYTES)})),
        Err(SpellError::LimitExceeded)
    ));
}
