//! Offline source selection and exact selected-page boundaries.

use super::*;

/// Builds an explicit synthetic request without credentials or native storage.
fn request(resource: Resource, id: &str, field: Option<&str>) -> Request {
    Request {
        resource,
        id: id.to_owned(),
        field: field.map(str::to_owned),
        page: None,
        class_id: None,
        markdown: false,
        raw: false,
    }
}

/// Carries the explicit monster backup through Markdown without fetching lookup data.
#[tokio::test]
async fn raw_monster_markdown_skips_lookup_configuration() {
    use crate::monster::fixture::{self, Scenario};
    let session = fixture::session(Scenario::ConfigurationDenied).expect("fixture");
    let mut selected = request(Resource::Monster, "17", None);
    selected.markdown = true;
    selected.raw = true;
    let result = read(&session, &selected).await.expect("raw Markdown");
    assert_eq!(result.record["raw"]["stats"][0]["statId"], 1);
    assert!(result.record.get("resolved").is_none());
    assert_eq!(fixture::routes(&session), ["/v1/Monster/17"]);
    let mut invalid = request(Resource::Feat, "17", None);
    invalid.raw = true;
    assert!(matches!(invalid.validate(), Err(HtmlError::InvalidInput)));
}

/// Exercises only fixed source paths and canonical bounded array positions.
#[test]
fn selectors_reject_arbitrary_paths_and_noncanonical_indices() {
    for field in [
        "source_url",
        "raw.description",
        "raw.classes[999].subclass_definition.class_features[999].more_details_url",
    ] {
        assert!(validate_field(field).is_ok(), "{field}");
    }
    for field in [
        "https://www.dndbeyond.com/feats/17-invented",
        "raw.moreDetailsUrl",
        "raw.classes[01].definition.more_details_url",
        "raw.classes[1000].definition.more_details_url",
        "raw.classes[-1].definition.more_details_url",
        "raw.any_url",
        "",
    ] {
        assert!(
            matches!(validate_field(field), Err(HtmlError::InvalidInput)),
            "{field}"
        );
    }
    assert!(matches!(
        request(Resource::Equipment, "7:17", Some("source_url")).validate(),
        Err(HtmlError::InvalidInput)
    ));
}

/// Uses preserved definition links without scanning unknown source extensions.
fn character_record() -> Value {
    json!({"id":"hero","raw":{
        "background":{"definition":{"id":17,"name":"Invented Origin","moreDetailsUrl":"/backgrounds/17-invented"}},
        "classes":[{"definition":{"id":10,"name":"Invented Fighter","moreDetailsUrl":"/classes/10-invented",
            "classFeatures":[{"moreDetailsUrl":"/classes/10-invented#PrimaryFeature"}]},
            "subclassDefinition":{"id":7002,"name":"Invented Champion","moreDetailsUrl":"/classes/10-invented#InventedChampion"}}],
        "feats":[{"definition":{"id":18,"name":"Invented Feat","moreDetailsUrl":"/feats/18-invented"}}],
        "readonlyUrl":"https://example.com/hero","future":{"url":"https://example.com/unknown"}
    }})
}

/// Requires explicit selection before any page client is created.
#[test]
fn character_links_are_ambiguous_and_explicit_selection_is_exact() {
    let record = character_record();
    let candidates = sources(&record, Resource::Character).expect("synthetic sources");
    let Err(HtmlError::AmbiguousSource(fields)) = choose(&candidates, None, false) else {
        panic!("ambiguous sources");
    };
    assert_eq!(fields.len(), 5);
    let selected = choose(
        &candidates,
        Some("raw.classes[0].subclass_definition.more_details_url"),
        false,
    )
    .expect("explicit selection")
    .expect("selected source");
    assert_eq!(
        selected.target.as_ref().expect("target").fragment(),
        Some("InventedChampion")
    );
    assert!(matches!(
        choose(&candidates, Some("raw.readonly_url"), false),
        Err(HtmlError::UnsupportedSource(_))
    ));
    assert!(matches!(
        choose(
            &candidates,
            Some("raw.feats[999].definition.more_details_url"),
            false
        ),
        Err(HtmlError::MissingSource)
    ));
    assert_eq!(record, character_record());
}

/// Keeps normalized aliases from creating an extra candidate for the same source.
#[test]
fn aliases_and_inline_descriptions_do_not_create_page_ambiguity() {
    let record = json!({"id":"17","name":"Invented Origin","source_url":"https://www.dndbeyond.com/backgrounds/17-invented",
        "raw":{"description":"<p>Synthetic description.</p>"}});
    let candidates = sources(&record, Resource::Background).expect("sources");
    assert_eq!(
        choose(&candidates, None, false)
            .expect("automatic")
            .expect("source")
            .field,
        "source_url"
    );
    assert_eq!(
        choose(&candidates, Some("raw.more_details_url"), false)
            .expect("alias")
            .expect("source")
            .field,
        "source_url"
    );
    assert_eq!(
        choose(&candidates, Some("raw.description"), false)
            .expect("inline")
            .expect("source")
            .inline
            .as_deref(),
        Some("<p>Synthetic description.</p>")
    );
}

/// Preserves campaign descriptions without inventing a selected HTML source contract.
#[test]
fn campaign_description_does_not_create_an_unverified_html_source() {
    let record = json!({"id":"8","raw":{"description":"<p>Synthetic campaign description.</p>"}});
    let candidates = sources(&record, Resource::Campaign).expect("source inventory");
    assert!(matches!(
        choose(&candidates, None, false),
        Err(HtmlError::MissingSource)
    ));
    assert!(
        choose(&candidates, None, true)
            .expect("record Markdown")
            .is_none()
    );
}

/// Never treats foreign links, API attribution, changed IDs, or missing parents as page grants.
#[test]
fn unsupported_and_malformed_source_metadata_is_not_silently_dropped() {
    for url in [
        "/feats/18-invented",
        "https://example.com/feats/17-invented",
        "/feats/17-invented?unverified=1",
        "/feats/17-invented%2fother",
        "/feats/17-invented#fragment",
        "https://www.dndbeyond.com@elsewhere/feats/17-invented",
    ] {
        let record = json!({"id":"17","name":"Invented Feat","source_url":url});
        let candidates = sources(&record, Resource::Feat).expect("unsupported source");
        assert!(
            matches!(
                choose(&candidates, None, false),
                Err(HtmlError::UnsupportedSource(_))
            ),
            "{url}"
        );
        assert!(
            choose(&candidates, None, true)
                .expect("ordinary Markdown")
                .is_none()
        );
    }
    let record = json!({"id":"17","name":"Invented Feat","source_url":42});
    assert!(matches!(
        sources(&record, Resource::Feat),
        Err(HtmlError::InvalidResponse)
    ));
    let mut record = character_record();
    record["raw"]["classes"][0]["definition"]["id"] = Value::Null;
    assert!(matches!(
        sources(&record, Resource::Character),
        Err(HtmlError::InvalidResponse)
    ));
    let mut record = character_record();
    record["raw"]["classes"] = json!({});
    assert!(matches!(
        sources(&record, Resource::Character),
        Err(HtmlError::InvalidResponse)
    ));
    let mut record = character_record();
    record["raw"]["classes"][0]["definition"]["name"] = json!("x".repeat(1025));
    assert!(matches!(
        sources(&record, Resource::Character),
        Err(HtmlError::LimitExceeded)
    ));
}

/// Stops candidate allocation once the established source-count limit is exceeded.
#[test]
fn source_count_is_bounded_during_nested_enumeration() {
    let mut record = character_record();
    let feature = json!({"moreDetailsUrl":"/classes/10-invented#PrimaryFeature"});
    record["raw"]["classes"][0]["definition"]["classFeatures"] = json!(vec![feature; 1_000]);
    assert!(matches!(
        sources(&record, Resource::Character),
        Err(HtmlError::LimitExceeded)
    ));
}

/// Checks background identity, raw preservation, and exactly one additional cookie page.
#[tokio::test]
async fn background_page_and_inline_selection_have_exact_request_counts() {
    use crate::background::fixture::{self, Scenario};
    let session = fixture::session(Scenario::Normal).expect("fixture");
    let value = read(&session, &request(Resource::Background, "17", None))
        .await
        .expect("selected page");
    assert!(value.record.get("raw_html").is_none());
    let selected = value.selected.expect("HTML");
    assert!(selected.raw_html.contains("Synthetic selected content."));
    assert!(!selected.raw_html.contains("comments"));
    assert_eq!(fixture::routes(&session).len(), 3);
    let session = fixture::session(Scenario::Normal).expect("fixture");
    let value = read(
        &session,
        &request(Resource::Background, "17", Some("raw.description")),
    )
    .await
    .expect("inline");
    assert_eq!(
        value.selected.expect("HTML").raw_html,
        value.record["raw"]["description"]
    );
    assert_eq!(fixture::routes(&session).len(), 2);
}

/// Does not fall back to inline text after a selected page denies or fails identity checks.
#[tokio::test]
async fn selected_page_failures_stop_without_fallback_or_retry() {
    use crate::background::fixture::{self, Scenario};
    for scenario in [
        Scenario::HtmlDenied,
        Scenario::HtmlMismatch,
        Scenario::HtmlUnsafe,
    ] {
        let session = fixture::session(scenario).expect("fixture");
        assert!(
            read(&session, &request(Resource::Background, "17", None))
                .await
                .is_err()
        );
        assert_eq!(fixture::routes(&session).len(), 3);
    }
}

/// Selects one feat's observed container and excludes adjacent account and comment UI.
#[tokio::test]
async fn feat_page_uses_cookie_only_after_checked_record_resolution() {
    use crate::feat::fixture::{self, Scenario};
    let session = fixture::session(Scenario::Normal).expect("fixture");
    let result = read(&session, &request(Resource::Feat, "18", None))
        .await
        .expect("feat HTML");
    let selected = result.selected.expect("HTML");
    assert!(selected.raw_html.contains("Synthetic feat paragraph."));
    assert!(!selected.raw_html.contains("Unrelated"));
    assert_eq!(fixture::routes(&session).len(), 3);
    let session = fixture::session(Scenario::HtmlDenied).expect("fixture");
    assert!(matches!(
        read(&session, &request(Resource::Feat, "18", None)).await,
        Err(HtmlError::Session(_))
    ));
    assert_eq!(fixture::routes(&session).len(), 3);
}

/// Keeps class metadata unchanged while excluding subclasses from selected primary HTML.
#[tokio::test]
async fn class_and_subclass_read_exact_supported_boundaries() {
    use crate::class_catalog::fixture::{self, Scenario};
    let session = fixture::session(Scenario::Html).expect("fixture");
    let result = read(&session, &request(Resource::Class, "10", None))
        .await
        .expect("class HTML");
    assert!(result.record.get("raw").is_none());
    let html = result.selected.expect("HTML").raw_html;
    assert!(html.contains("Synthetic primary class introduction."));
    assert!(!html.contains("subclass"));
    assert!(!html.contains("create-character"));
    assert_eq!(fixture::requested_routes(&session).len(), 3);
    let session = fixture::session(Scenario::Html).expect("fixture");
    let mut selected = request(Resource::Subclass, "7002", None);
    selected.class_id = Some("10".to_owned());
    let result = read(&session, &selected).await.expect("subclass HTML");
    let html = result.selected.expect("HTML").raw_html;
    assert!(html.contains("Synthetic subclass introduction."));
    assert!(!html.contains("primary class"));
    assert!(!html.contains("Unrelated subclass"));
    assert_eq!(fixture::requested_routes(&session).len(), 4);
    let session = fixture::session(Scenario::HtmlMissingFragment).expect("fixture");
    assert!(matches!(
        read(&session, &selected).await,
        Err(HtmlError::UnsupportedSource(_))
    ));
    assert_eq!(fixture::requested_routes(&session).len(), 4);
}

/// Selects exact heading and named-anchor fragments without crossing the next peer section.
#[test]
fn fragments_preserve_exact_feature_boundaries_and_subclass_identity() {
    let page = crate::class_catalog::fixture::html_page(10);
    for (fragment, kind, name, paragraph) in [
        (
            "PrimaryFeature",
            PageKind::Class,
            "Invented Fighter",
            "Synthetic primary feature paragraph.",
        ),
        (
            "SubclassFeature",
            PageKind::Subclass,
            "Invented Champion",
            "Synthetic subclass feature paragraph.",
        ),
        (
            "NamedFeature",
            PageKind::Subclass,
            "Invented Champion",
            "Synthetic named feature paragraph.",
        ),
    ] {
        let url = reqwest::Url::parse(&format!(
            "https://www.dndbeyond.com/classes/10-invented#{fragment}"
        ))
        .expect("URL");
        let html = page::selected(page.as_bytes(), kind, &url, name).expect("feature");
        assert!(html.contains(paragraph));
        assert!(!html.contains("Other"));
        assert!(!html.contains("Unrelated"));
        if kind == PageKind::Subclass {
            assert!(matches!(
                page::selected(page.as_bytes(), kind, &url, "Wrong subclass"),
                Err(HtmlError::InvalidResponse)
            ));
        }
    }
    let anchored = page.replace(
        "<h2 id=\"InventedChampion\">",
        "<h2><a name=\"InventedChampion\"></a>",
    );
    let url = reqwest::Url::parse("https://www.dndbeyond.com/classes/10-invented#InventedChampion")
        .expect("URL");
    let html = page::selected(
        anchored.as_bytes(),
        PageKind::Subclass,
        &url,
        "Invented Champion",
    )
    .expect("named subclass");
    assert!(html.contains("Synthetic subclass introduction."));
    assert!(html.contains("Synthetic subclass feature paragraph."));
    assert!(!html.contains("Unrelated subclass"));
}

/// Uses one checked character record and only the explicitly selected nested page.
#[tokio::test]
async fn character_selection_fetches_only_one_nested_source_after_one_record_read() {
    use crate::character_campaign::fixture::{self, Scenario};
    let session = fixture::session(Scenario::HtmlLinks).expect("fixture");
    assert!(matches!(
        read(&session, &request(Resource::Character, "42", None)).await,
        Err(HtmlError::AmbiguousSource(_))
    ));
    assert_eq!(fixture::request_count(&session), 1);
    assert!(crate::background::fixture::routes(&session).is_empty());
    assert!(crate::class_catalog::fixture::requested_routes(&session).is_empty());
    assert!(crate::feat::fixture::routes(&session).is_empty());
    let session = fixture::session(Scenario::HtmlLinks).expect("fixture");
    let result = read(
        &session,
        &request(
            Resource::Character,
            "42",
            Some("raw.classes[0].definition.class_features[0].more_details_url"),
        ),
    )
    .await
    .expect("selected feature");
    assert_eq!(fixture::request_count(&session), 1);
    assert_eq!(
        crate::class_catalog::fixture::requested_routes(&session).len(),
        1
    );
    assert!(crate::background::fixture::routes(&session).is_empty());
    assert!(crate::feat::fixture::routes(&session).is_empty());
    let html = result.selected.expect("HTML").raw_html;
    assert!(html.contains("Synthetic primary feature paragraph."));
    assert!(!html.contains("Other primary"));
    assert!(
        result.record["raw"]["future"]["preserved"]
            .as_bool()
            .expect("extension")
    );
}

/// Keeps raw item descriptions inline without fetching their API attribution as HTML.
#[tokio::test]
async fn item_html_is_inline_and_preserves_original_raw_values() {
    use crate::item_catalog::fixture::{self, Scenario};
    for (resource, id) in [(Resource::Equipment, "7:17"), (Resource::MagicItem, "7:18")] {
        let session = fixture::session(Scenario::HtmlInline).expect("fixture");
        let mut selected = request(resource, id, None);
        selected.page = Some(1);
        let value = read(&session, &selected).await.expect("inline HTML");
        let html = value.selected.expect("HTML");
        assert_eq!(html.raw_html, value.record["raw"]["description"]);
        assert!(html.source_url.is_none());
        assert_eq!(fixture::routes(&session).len(), 2);
    }
}

/// Rejects active/document content and bounds before serialization; passive header/style remains inert.
#[test]
fn inline_html_rejects_active_content_and_document_or_dom_excess() {
    assert!(
        page::validate_fragment(
            "<header>Invented title</header><p style=\"text-align:center\">Synthetic body.</p>"
        )
        .is_ok()
    );
    for html in [
        "<html><body>Synthetic</body></html>",
        "<script>Synthetic</script>",
        "<p onclick=\"alert(1)\">Synthetic</p>",
        "<a href=\"javascript:alert(1)\">Synthetic</a>",
        "<p style=\"background:url(https://example.com)\">Synthetic</p>",
        "<p class=\"paywall\">Synthetic</p>",
        "<p>\u{1b}[31mSynthetic</p>",
    ] {
        assert!(page::validate_fragment(html).is_err(), "{html}");
    }
    assert!(matches!(
        page::validate_fragment(&"x".repeat(1024 * 1024 + 1)),
        Err(HtmlError::LimitExceeded)
    ));
    let deep = format!("{}Synthetic{}", "<div>".repeat(260), "</div>".repeat(260));
    assert!(matches!(
        page::validate_fragment(&deep),
        Err(HtmlError::LimitExceeded)
    ));
    assert!(matches!(
        page::validate_fragment(&"<i></i>".repeat(100_000)),
        Err(HtmlError::LimitExceeded)
    ));
}
