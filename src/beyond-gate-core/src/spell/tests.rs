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

/// Rejects missing next-link targets, including whitespace-separated relation tokens.
#[test]
fn spell_pagination_requires_next_link_href() {
    let query = SpellQuery::default();
    let valid = fixture::listing(&query, Scenario::Next);
    let mut next = query.clone();
    next.page += 1;
    let href = next
        .website_url(WEBSITE)
        .expect("synthetic next URL")
        .as_str()
        .replace('&', "&amp;");
    for rel in ["next", "prev next", "next alternate"] {
        let altered = valid.replace(
            &format!("rel='next' href='{href}'"),
            &format!("rel='{rel}'"),
        );
        assert_ne!(altered, valid, "fixture mutation must remove href");
        assert!(matches!(
            website::list(altered.as_bytes(), &query),
            Err(SpellError::IncompleteResult)
        ));
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

/// Rejects conflicting single-select choices while preserving valid multiple selections.
#[test]
fn spell_filter_checks_reject_multiple_single_select_choices() {
    let query = SpellQuery {
        class_ids: vec![17, 18],
        ..SpellQuery::default()
    };
    let valid = fixture::listing(&query, Scenario::Normal);
    website::list(valid.as_bytes(), &query).expect("valid multiple selection");
    for replacement in [
        "<select name='filter-class'>",
        "<select name='filter-class' size='2'>",
    ] {
        let body = valid.replace("<select name='filter-class' multiple>", replacement);
        assert!(matches!(
            website::list(body.as_bytes(), &query),
            Err(SpellError::InvalidResponse)
        ));
    }

    let query = SpellQuery::default();
    let body = fixture::listing(&query, Scenario::Normal)
        .replace("<option value='17' >", "<option value='17' selected>");
    let body = body
        .replace("<option value='18' >", "<option value='18' selected>")
        .replace(
            "<select name='filter-class' multiple>",
            "<select name='filter-class'>",
        );
    assert!(matches!(
        website::list(body.as_bytes(), &query),
        Err(SpellError::InvalidResponse)
    ));
}

/// Rejects unexpected submitted hidden filters, but permits empty or disabled inputs.
#[test]
fn spell_filter_checks_include_enabled_hidden_restrictions() {
    let query = SpellQuery::default();
    let valid = fixture::listing(&query, Scenario::Normal);
    for control in [
        "<input type='hidden' name='filter-level' value='9'>",
        "<input type='hidden' name='filter-unknown' value='restricted'>",
        "<input type='HIDDEN' name='filter-level' value='9'>",
    ] {
        let body = valid.replace("</form>", &format!("{control}</form>"));
        assert!(matches!(
            website::list(body.as_bytes(), &query),
            Err(SpellError::IncompleteResult)
        ));
    }
    for control in [
        "<input type='hidden' name='filter-level' value=''>",
        "<input type='hidden' name='filter-level'>",
        "<input type='hidden' name='filter-level' value='9' disabled>",
        "<input type='hidden' name='other-control' value='9'>",
    ] {
        let body = valid.replace("</form>", &format!("{control}</form>"));
        website::list(body.as_bytes(), &query).expect("no submitted hidden restriction");
    }
}

/// Projects exact displayed spell values and preserves unknown HTML without another read.
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
    assert!(!raw.contains("<script"));
    assert!(!raw.contains("cobaltVcmList"));
    assert_eq!(value["id"], "900001");
    assert_eq!(
        value["raw"],
        json!({
            "level": "Invented level",
            "casting_time": "1 invented action",
            "range_area": "30 invented feet",
            "components": "V, S, M (invented material)",
            "duration": "Concentration, 1 invented minute",
            "school": "Invented school",
            "attack_save": "Invented save",
            "damage_effect": "Invented effect",
            "description_html": "<p>Synthetic <strong>spell</strong> detail.</p>",
            "available_for": "Invented Class",
            "source": "Invented Source"
        })
    );
    assert!(value.get("access_code").is_none());
    assert!(value.get("resolved").is_none());
    assert_eq!(
        fixture::requested_routes(&session),
        vec!["/spells/900001-invented-spell"]
    );
    assert_eq!(auth::request_count(&session), 1);
}

/// Removes only the observed registration and keeps unknown selected content unchanged.
#[test]
fn spell_registration_is_not_returned_as_content_or_an_access_grant() {
    let slug = "900001-invented-spell";
    let body = fixture::detail(slug, Scenario::Normal);
    let expected = website::show(body.as_bytes(), slug).expect("checked detail");
    for script_type in [
        "",
        " type='text/javascript'",
        " type='application/javascript'",
    ] {
        let body = body.replace("<script>", &format!("<script{script_type}>"));
        assert_eq!(
            website::show(body.as_bytes(), slug).expect("registration excluded"),
            expected
        );
    }
    let registration = "<script>window.cobaltVcmList = window.cobaltVcmList || []; window.cobaltVcmList.push({type:1118725998, id:900001});</script>";
    let without_registration = body.replace(registration, "");
    assert_ne!(without_registration, body);
    assert_eq!(
        website::show(without_registration.as_bytes(), slug).expect("script-free detail"),
        expected
    );
    let spaced = body.replace(
        "window.cobaltVcmList = window.cobaltVcmList || [];",
        "window.cobaltVcmList =\n window.cobaltVcmList || [];\n",
    );
    assert_eq!(
        website::show(spaced.as_bytes(), slug).expect("ASCII whitespace"),
        expected
    );
    assert!(expected.get("access_code").is_none());
}

/// Rejects modified registration code, external scripts, and protected registration wrappers.
#[test]
fn spell_registration_exclusion_does_not_relax_other_guards() {
    let slug = "900001-invented-spell";
    let body = fixture::detail(slug, Scenario::Normal);
    for (before, after) in [
        ("id:900001", "id:900099"),
        ("type:1118725998", "type:17"),
        ("</script>", "alert('synthetic');</script>"),
        ("<script>", "<script src='/synthetic.js'>"),
        ("<script>", "<script type='application/json'>"),
        ("<script>", "<script type='module'>"),
        ("<script>", "<script class='account'>"),
        ("<script>", "<script class='login'>"),
        ("</script>", "</script><script></script>"),
    ] {
        let changed = body.replace(before, after);
        assert!(matches!(
            website::show(changed.as_bytes(), slug),
            Err(SpellError::UnsupportedDetailContent(_))
        ));
    }
}

/// Distinguishes unsupported spell pages without disclosing returned HTML.
#[test]
fn spell_detail_rejection_identifies_the_failed_check() {
    let slug = "900001-invented-spell";
    for (body, message) in [
        (
            "<form id='challenge-form'>Synthetic challenge</form>".to_owned(),
            "spell page contains a browser challenge; no content was returned",
        ),
        (
            fixture::detail(slug, Scenario::Unsupported),
            "spell page does not contain the supported detail container; no content was returned",
        ),
        (
            fixture::detail(slug, Scenario::UnsafeDetail),
            "spell detail contains active or protected markup (form); no content was returned",
        ),
    ] {
        let error = website::show(body.as_bytes(), slug).expect_err("rejected");
        assert_eq!(error.to_string(), message);
        assert!(!format!("{error:?}").contains("Synthetic"));
    }
}

/// Checks every rejected marker without disclosing its attributes or text.
#[test]
fn spell_detail_rejection_reports_only_fixed_marker_names() {
    let slug = "900001-invented-spell";
    for (markup, marker) in [
        ("<script>Synthetic private text</script>", "script"),
        ("<form>Synthetic private text</form>", "form"),
        ("<iframe></iframe>", "iframe"),
        ("<object></object>", "object"),
        ("<embed>", "embed"),
        ("<nav>Synthetic private text</nav>", "nav"),
        (
            "<aside id='synthetic-comment-private'></aside>",
            "comment marker",
        ),
        (
            "<aside class='synthetic-comment-private'></aside>",
            "comment marker",
        ),
        (
            "<aside id='synthetic-account-private'></aside>",
            "account marker",
        ),
        (
            "<aside class='synthetic-account-private'></aside>",
            "account marker",
        ),
        (
            "<aside class='synthetic-purchase-private'></aside>",
            "purchase marker",
        ),
        (
            "<aside class='synthetic-unlock-private'></aside>",
            "unlock marker",
        ),
        (
            "<aside class='synthetic-login-private'></aside>",
            "login marker",
        ),
    ] {
        let body = fixture::detail(slug, Scenario::Normal)
            .replace("</section></div>", &format!("{markup}</section></div>"));
        let error = website::show(body.as_bytes(), slug).expect_err("rejected");
        assert!(matches!(
            error,
            SpellError::UnsupportedDetailContent(actual) if actual == marker
        ));
        assert_eq!(
            error.to_string(),
            format!(
                "spell detail contains active or protected markup ({marker}); no content was returned"
            )
        );
    }
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
