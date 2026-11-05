//! Original synthetic metadata and actual loopback discovery transport checks.

use super::*;
use crate::auth::session::fixture::{self, Scenario};
use serde_json::json;
use std::{
    collections::{BTreeMap, HashSet},
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    task::JoinHandle,
};

/// Identifies metadata rejection without exposing values or arbitrary field names.
fn catalog_diagnostic(source: &Source) -> Vec<&'static str> {
    if source.record(AccessEvidence::CatalogOnly).is_ok() {
        return vec![];
    }
    let mut flags = Vec::new();
    if source.id == 0 {
        flags.push("invalid_source_id");
    }
    if !valid_text(&source.name) {
        flags.push("invalid_title");
        flags.extend(title_diagnostic(Some(&source.name)));
    }
    let path = source
        .relative_path
        .as_deref()
        .filter(|path| !path.is_empty())
        .map(str::to_owned)
        .or_else(|| {
            source
                .slug
                .as_deref()
                .filter(|slug| !slug.is_empty())
                .map(|slug| format!("sources/dnd/{slug}"))
        });
    if let Some(path) = path {
        if path.starts_with('/') {
            flags.push("book_path_has_leading_slash");
        }
        if !path.starts_with("sources/") {
            flags.push("book_path_missing_source_prefix");
        }
        if path.split('/').any(|segment| {
            segment.is_empty()
                || !segment
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        }) {
            flags.push("book_path_has_invalid_segment");
        }
    } else {
        match source.marketplace_url.as_deref() {
            None | Some("") => flags.push("no_book_path_or_marketplace_attribution"),
            Some(supplied) => match reqwest::Url::parse(supplied) {
                Err(_) => flags.push("marketplace_url_parse_failed"),
                Ok(url) => {
                    if url.host_str() == Some("www.dndbeyond.com") {
                        flags.push("marketplace_url_uses_www_host");
                    }
                    if url.host_str() != Some("marketplace.dndbeyond.com") {
                        flags.push("marketplace_url_host_not_approved");
                    }
                    if url.scheme() != "https" || url.port().is_some_and(|port| port != 443) {
                        flags.push("marketplace_url_scheme_or_port_invalid");
                    }
                    if !url.username().is_empty()
                        || url.password().is_some()
                        || url.fragment().is_some()
                        || supplied.trim() != supplied
                        || supplied.chars().any(char::is_control)
                    {
                        flags.push("marketplace_url_metadata_invalid");
                    }
                }
            },
        }
    }
    if flags.is_empty() {
        flags.push("unclassified_metadata_invalid");
    }
    flags
}

/// Gates only the new two-attempt diagnostic, never reuses an older acknowledgment.
fn catalog_diagnostic_approved(value: Option<&str>, isolated: bool) -> bool {
    value == Some("issue-32-catalog-attribution-two-attempts") && !isolated
}

/// Covers rejection categories with invented values and checks value-free output.
#[test]
fn catalog_diagnostic_flags_are_fixed() {
    let mut missing = source(1);
    missing["slug"] = Value::Null;
    let missing: Source = serde_json::from_value(missing).expect("invented source");
    assert_eq!(
        catalog_diagnostic(&missing),
        ["no_book_path_or_marketplace_attribution"]
    );
    let mut rooted = source(1);
    rooted["relativeSlugPath"] = json!("/sources/dnd/invented");
    let rooted: Source = serde_json::from_value(rooted).expect("invented source");
    assert!(catalog_diagnostic(&rooted).contains(&"book_path_has_leading_slash"));
    let mut www = source(1);
    www["slug"] = Value::Null;
    www["marketplaceUrl"] = json!("https://www.dndbeyond.com/marketplace/invented-private");
    let www: Source = serde_json::from_value(www).expect("invented source");
    assert!(catalog_diagnostic(&www).contains(&"marketplace_url_uses_www_host"));
    assert!(!format!("{:?}", catalog_diagnostic(&www)).contains("invented-private"));
    assert!(
        catalog_diagnostic(&serde_json::from_value(source(1)).expect("invented source")).is_empty()
    );
}

/// Rejects missing, consumed-scope, and isolated acknowledgments before native access.
#[test]
fn catalog_diagnostic_requires_separate_approval() {
    assert!(!catalog_diagnostic_approved(None, false));
    assert!(!catalog_diagnostic_approved(
        Some("issue-32-metadata-three-attempts"),
        false
    ));
    assert!(!catalog_diagnostic_approved(
        Some("issue-32-catalog-attribution-two-attempts"),
        true
    ));
    assert!(catalog_diagnostic_approved(
        Some("issue-32-catalog-attribution-two-attempts"),
        false
    ));
}

/// Validates the proposed display normalization and explicit legacy attribution without fetching it.
fn proposed_released_metadata(source: &Source) -> Result<Book, LibraryError> {
    if source.id == 0 {
        return Err(LibraryError::InvalidMetadata);
    }
    let url = source.url().or_else(|error| {
        if source
            .relative_path
            .as_deref()
            .is_some_and(|path| !path.is_empty())
            || source.slug.as_deref().is_some_and(|slug| !slug.is_empty())
        {
            return Err(error);
        }
        let supplied = source.marketplace_url.as_ref().ok_or(error)?;
        let parsed = reqwest::Url::parse(supplied).map_err(|_| LibraryError::InvalidMetadata)?;
        if parsed.scheme() != "https"
            || parsed.host_str() != Some("www.dndbeyond.com")
            || !parsed.path().starts_with("/marketplace/")
            || parsed.port().is_some_and(|port| port != 443)
        {
            return Err(LibraryError::InvalidMetadata);
        }
        Ok(supplied.clone())
    })?;
    Book::from_record(BookRecord {
        id: source.id.to_string(),
        title: source.name.split_whitespace().collect::<Vec<_>>().join(" "),
        source_url: url,
        evidence: AccessEvidence::CatalogOnly,
    })
}

/// Reports only the conditions required by the newly approved release-boundary scope.
fn released_catalog_conditions(catalog: &Collection<Value>) -> [bool; 3] {
    let mut all_boolean = true;
    let mut missing_only_unreleased = true;
    let mut released_valid = true;
    for value in &catalog.items {
        let released = value.get("isReleased").and_then(Value::as_bool);
        all_boolean &= released.is_some();
        let Ok(source) = serde_json::from_value::<Source>(value.clone()) else {
            released_valid = false;
            continue;
        };
        let missing = source.relative_path.as_deref().is_none_or(str::is_empty)
            && source.slug.as_deref().is_none_or(str::is_empty)
            && source.marketplace_url.as_deref().is_none_or(str::is_empty);
        if missing {
            missing_only_unreleased &= released == Some(false);
        }
        if released == Some(true) {
            released_valid &= proposed_released_metadata(&source).is_ok();
        }
    }
    [all_boolean, missing_only_unreleased, released_valid]
}

/// Verifies that the release-boundary check cannot hide malformed released metadata or a missing flag.
#[test]
fn released_catalog_contract_conditions_are_strict() {
    let mut valid = source(1);
    valid["isReleased"] = json!(true);
    valid["name"] = json!("  Invented\n volume\t ");
    let mut unreleased = source(2);
    unreleased["isReleased"] = json!(false);
    unreleased["name"] = json!("");
    unreleased["slug"] = Value::Null;
    let catalog: Collection<Value> =
        serde_json::from_value(collection(vec![valid.clone(), unreleased.clone()]))
            .expect("invented catalog");
    assert_eq!(released_catalog_conditions(&catalog), [true, true, true]);
    unreleased["isReleased"] = json!(true);
    let catalog = serde_json::from_value(collection(vec![valid.clone(), unreleased]))
        .expect("invented catalog");
    assert_eq!(released_catalog_conditions(&catalog), [true, false, false]);
    valid
        .as_object_mut()
        .expect("source object")
        .remove("isReleased");
    let catalog = serde_json::from_value(collection(vec![valid])).expect("invented catalog");
    assert_eq!(released_catalog_conditions(&catalog), [false, true, true]);
}

/// Runs only the newly approved diagnostic phase, before the conditional fix and remaining acceptance calls.
#[cfg(windows)]
#[tokio::test]
#[ignore = "Requires the separate conditional released-library ten-attempt scope"]
async fn approved_live_released_catalog_contract() -> Result<(), LibraryError> {
    let approval = std::env::var("BEYOND_GATE_LIBRARY_PROBE_APPROVAL").ok();
    assert!(
        approval.as_deref() == Some("issue-32-released-library-ten-attempts")
            && !cfg!(feature = "e2e-isolation"),
        "separate release-boundary scope required"
    );
    let client = HttpClient::single_attempt(&[ORIGIN]).map_err(LibraryError::PageRead)?;
    let session = Session::default();
    let catalog: Collection<Value> =
        timeout(Duration::from_secs(60), session.get_json(&client, SOURCES))
            .await
            .map_err(|_| LibraryError::TimedOut)?
            .map_err(LibraryError::SessionRead)?;
    catalog.validate(MAX_CATALOG_RECORDS)?;
    let [boolean, missing, valid] = released_catalog_conditions(&catalog);
    eprintln!("all_records_have_boolean_is_released={boolean}");
    eprintln!("all_missing_attribution_records_are_unreleased={missing}");
    eprintln!("all_released_records_pass_proposed_metadata_validation={valid}");
    eprintln!(
        "ownership_requested=false; raw_results_persisted=false; diagnostic_phase_consumed=true"
    );
    if !boolean || !missing || !valid {
        return Err(LibraryError::InvalidMetadata);
    }
    Ok(())
}

/// Runs one separately approved exchange/catalog scope with fixed classification output only.
#[cfg(windows)]
#[tokio::test]
#[ignore = "Requires separate native credential-access and catalog diagnostic approval"]
async fn approved_live_catalog_diagnostic() -> Result<(), LibraryError> {
    let approval = std::env::var("BEYOND_GATE_LIBRARY_PROBE_APPROVAL").ok();
    assert!(
        catalog_diagnostic_approved(approval.as_deref(), cfg!(feature = "e2e-isolation")),
        "separate catalog diagnostic approval required"
    );
    let client = HttpClient::single_attempt(&[ORIGIN]).map_err(LibraryError::PageRead)?;
    let session = Session::default();
    let catalog: Collection<Source> =
        timeout(Duration::from_secs(60), session.get_json(&client, SOURCES))
            .await
            .map_err(|_| LibraryError::TimedOut)?
            .map_err(LibraryError::SessionRead)?;
    catalog.validate(MAX_CATALOG_RECORDS)?;
    let flags = catalog
        .items
        .iter()
        .flat_map(catalog_diagnostic)
        .collect::<std::collections::BTreeSet<_>>();
    eprintln!("catalog_metadata_valid={}", flags.is_empty());
    for flag in flags {
        eprintln!("{flag}=true");
    }
    eprintln!(
        "ownership_requested=false; raw_results_persisted=false; diagnostic_scope_consumed=true"
    );
    Ok(())
}

/// Gates the new catalog comparison independently of every consumed scope.
fn metadata_comparison_approved(value: Option<&str>, isolated: bool) -> bool {
    value == Some("issue-32-corrected-config-comparison-three-attempts") && !isolated
}

/// Validates an explicitly supplied configuration title and link, without fetching the link.
fn config_book(value: &Value) -> Result<Book, LibraryError> {
    let id = value["id"]
        .as_u64()
        .filter(|id| *id != 0)
        .ok_or(LibraryError::InvalidMetadata)?;
    let title = value["description"]
        .as_str()
        .ok_or(LibraryError::InvalidMetadata)?;
    let supplied = value["sourceURL"]
        .as_str()
        .ok_or(LibraryError::InvalidMetadata)?;
    if !valid_text(supplied) {
        return Err(LibraryError::InvalidMetadata);
    }
    let absolute = if supplied.starts_with("sources/") {
        Source {
            id,
            name: title.to_owned(),
            relative_path: Some(supplied.to_owned()),
            slug: None,
            marketplace_url: None,
        }
        .url()?
    } else if supplied.starts_with('/') && !supplied.starts_with("//") {
        format!("https://www.dndbeyond.com{supplied}")
    } else {
        supplied.to_owned()
    };
    let url = reqwest::Url::parse(&absolute).map_err(|_| LibraryError::InvalidMetadata)?;
    if url.scheme() != "https"
        || !config_location_approved(&url)
        || url.port().is_some_and(|port| port != 443)
    {
        return Err(LibraryError::InvalidMetadata);
    }
    Book::from_record(BookRecord {
        id: id.to_string(),
        title: title.to_owned(),
        source_url: absolute,
        evidence: AccessEvidence::CatalogOnly,
    })
}

/// Shares the configuration diagnostic's allowed metadata locations without fetching them.
fn config_location_approved(url: &reqwest::Url) -> bool {
    match url.host_str() {
        Some("www.dndbeyond.com") => {
            url.path().starts_with("/sources/") || url.path().starts_with("/marketplace/")
        }
        Some("marketplace.dndbeyond.com") => true,
        _ => false,
    }
}

/// Classifies invalid titles using fixed labels, never their text.
fn title_diagnostic(title: Option<&str>) -> Vec<&'static str> {
    let Some(title) = title else {
        return vec!["title_missing_or_wrong_type"];
    };
    let mut flags = Vec::new();
    if title.is_empty() {
        flags.push("title_empty");
    }
    if title.trim() != title {
        flags.push("title_padded");
    }
    if title.chars().any(char::is_control) {
        flags.push("title_control_character");
    }
    flags
}

/// Identifies configuration title, path, and link failures without exposing supplied values.
fn config_diagnostic(value: &Value) -> Vec<&'static str> {
    if config_book(value).is_ok() {
        return vec![];
    }
    let mut flags = title_diagnostic(value["description"].as_str());
    if value["id"].as_u64().is_none_or(|id| id == 0) {
        flags.push("invalid_source_id");
    }
    if let Some(supplied) = value["sourceURL"].as_str() {
        if !valid_text(supplied) {
            flags.push("link_invalid_text");
        }
        if supplied.starts_with("//") {
            flags.push("link_scheme_relative");
        }
        if supplied.starts_with("sources/")
            && supplied.split('/').any(|segment| {
                segment.is_empty()
                    || !segment
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
            })
        {
            flags.push("source_path_invalid");
        }
        let absolute = if supplied.starts_with("sources/") {
            format!("https://www.dndbeyond.com/{supplied}")
        } else if supplied.starts_with('/') && !supplied.starts_with("//") {
            format!("https://www.dndbeyond.com{supplied}")
        } else {
            supplied.to_owned()
        };
        match reqwest::Url::parse(&absolute) {
            Err(_) => flags.push("link_parse_failed"),
            Ok(url) => {
                if !config_location_approved(&url) {
                    flags.push("link_location_not_approved");
                }
                if url.scheme() != "https" || url.port().is_some_and(|port| port != 443) {
                    flags.push("link_scheme_or_port_invalid");
                }
                if !url.username().is_empty()
                    || url.password().is_some()
                    || url.fragment().is_some()
                {
                    flags.push("link_credentials_or_fragment");
                }
            }
        }
    } else {
        flags.push("link_missing_or_wrong_type");
    }
    if flags.is_empty() {
        flags.push("unclassified_metadata_invalid");
    }
    flags
}

/// Tests only whitespace cleanup of a supplied configuration title, with its original link unchanged.
fn proposed_config_metadata(value: &Value) -> Result<Book, LibraryError> {
    let title = value["description"]
        .as_str()
        .ok_or(LibraryError::InvalidMetadata)?;
    let mut normalized = value.clone();
    normalized["description"] = json!(title.split_whitespace().collect::<Vec<_>>().join(" "));
    config_book(&normalized)
}

/// Requires bounded, uniquely identified configuration records before protected storage is read.
fn config_sources(value: &Value) -> Result<HashMap<u64, &Value>, LibraryError> {
    let sources = value["sources"]
        .as_array()
        .ok_or(LibraryError::InvalidMetadata)?;
    let categories = value["sourceCategories"]
        .as_array()
        .ok_or(LibraryError::InvalidMetadata)?;
    if sources.len() > MAX_CATALOG_RECORDS || categories.len() > MAX_CATALOG_RECORDS {
        return Err(LibraryError::LimitExceeded);
    }
    let mut indexed = HashMap::new();
    for source in sources {
        let id = source["id"]
            .as_u64()
            .filter(|id| *id != 0)
            .ok_or(LibraryError::InvalidMetadata)?;
        if indexed.insert(id, source).is_some() {
            return Err(LibraryError::ConflictingMetadata);
        }
    }
    Ok(indexed)
}

/// Returns only fixed numeric diagnostics; source values never enter the output.
fn compare_catalogs(
    config: &HashMap<u64, &Value>,
    catalog: &Collection<Source>,
) -> Result<[usize; 5], LibraryError> {
    catalog.validate(MAX_CATALOG_RECORDS)?;
    let mut ids = std::collections::HashSet::new();
    let mut counts = [0; 5];
    for source in &catalog.items {
        if source.id == 0 || !ids.insert(source.id) {
            return Err(LibraryError::ConflictingMetadata);
        }
        let matched = config.get(&source.id).copied();
        let valid_config = matched.is_some_and(|value| config_book(value).is_ok());
        counts[0] += usize::from(matched.is_some());
        counts[1] += usize::from(valid_config);
        counts[2] += usize::from(source.record(AccessEvidence::CatalogOnly).is_err());
        let missing =
            catalog_diagnostic(source).contains(&"no_book_path_or_marketplace_attribution");
        counts[3] += usize::from(missing);
        counts[4] += usize::from(missing && valid_config);
    }
    Ok(counts)
}

/// Checks approval isolation, strict links, bounded IDs, and missing-link coverage offline.
#[test]
fn metadata_comparison_guards_use_synthetic_records() {
    assert!(!metadata_comparison_approved(None, false));
    assert!(!metadata_comparison_approved(
        Some("issue-32-released-library-ten-attempts"),
        false
    ));
    assert!(!metadata_comparison_approved(
        Some("issue-32-config-comparison-three-attempts"),
        false
    ));
    assert!(!metadata_comparison_approved(
        Some("issue-32-corrected-config-comparison-three-attempts"),
        true
    ));
    assert!(metadata_comparison_approved(
        Some("issue-32-corrected-config-comparison-three-attempts"),
        false
    ));
    let valid = json!({
        "id": 1, "name": "INV", "description": "Invented book",
        "sourceURL": "sources/dnd/invented", "sourceCategoryId": 2
    });
    assert!(config_book(&valid).is_ok());
    for url in [
        "/sources/dnd/invented",
        "https://www.dndbeyond.com/sources/dnd/invented",
    ] {
        let mut supported = valid.clone();
        supported["sourceURL"] = json!(url);
        assert!(config_book(&supported).is_ok());
    }
    for url in [
        "//example.invalid/sources/invented",
        "https://example.invalid/sources/invented",
        "http://www.dndbeyond.com/sources/invented",
        "https://www.dndbeyond.com/account",
        "https://www.dndbeyond.com/sources/invented#fragment",
        "sources/dnd/../account",
        "sources/dnd/invented?token=untrusted",
    ] {
        let mut invalid = valid.clone();
        invalid["sourceURL"] = json!(url);
        assert!(config_book(&invalid).is_err());
    }
    let config = json!({"sources": [valid.clone()], "sourceCategories": [{"id": 2}]});
    let indexed = config_sources(&config).expect("invented config");
    let mut missing = source(1);
    missing["slug"] = Value::Null;
    let catalog = serde_json::from_value(collection(vec![missing])).expect("invented catalog");
    assert_eq!(
        compare_catalogs(&indexed, &catalog).expect("invented comparison"),
        [1, 1, 1, 1, 1]
    );
    let duplicate = json!({"sources": [valid.clone(), valid], "sourceCategories": []});
    assert!(config_sources(&duplicate).is_err());
    assert!(config_sources(&json!({"sources": []})).is_err());
    let oversized =
        json!({"sources": vec![json!({"id": 1}); MAX_CATALOG_RECORDS + 1], "sourceCategories": []});
    assert!(config_sources(&oversized).is_err());
}

/// Compares fixed metadata routes in memory; the public request precedes any credential access.
#[cfg(windows)]
#[tokio::test]
#[ignore = "Requires separate approval for the three-attempt configuration comparison"]
async fn approved_live_metadata_comparison() -> Result<(), LibraryError> {
    let approval = std::env::var("BEYOND_GATE_LIBRARY_PROBE_APPROVAL").ok();
    assert!(
        metadata_comparison_approved(approval.as_deref(), cfg!(feature = "e2e-isolation")),
        "separate metadata comparison approval required"
    );
    timeout(Duration::from_secs(60), async {
        let public = HttpClient::single_attempt(&["https://www.dndbeyond.com"])
            .map_err(LibraryError::PageRead)?;
        eprintln!("public_config_requested=true; protected_storage_accessed=false");
        let config: Value = public
            .get_json("https://www.dndbeyond.com/api/config/json", None)
            .await
            .map_err(LibraryError::PageRead)?;
        let indexed = config_sources(&config)?;
        eprintln!("config_sources_count={}", indexed.len());
        eprintln!(
            "config_categories_count={}",
            config["sourceCategories"].as_array().map_or(0, Vec::len)
        );
        eprintln!(
            "config_sources_with_valid_metadata_count={}",
            indexed
                .values()
                .filter(|value| config_book(value).is_ok())
                .count()
        );
        eprintln!(
            "config_sources_with_valid_title_count={}",
            indexed
                .values()
                .filter(|value| value["description"].as_str().is_some_and(valid_text))
                .count()
        );
        eprintln!(
            "config_sources_with_nonblank_link_count={}",
            indexed
                .values()
                .filter(|value| value["sourceURL"]
                    .as_str()
                    .is_some_and(|url| !url.is_empty()))
                .count()
        );
        eprintln!(
            "config_sources_with_relative_source_path_count={}",
            indexed
                .values()
                .filter(|value| value["sourceURL"]
                    .as_str()
                    .is_some_and(|url| url.starts_with("sources/")))
                .count()
        );
        let client = HttpClient::single_attempt(&[ORIGIN]).map_err(LibraryError::PageRead)?;
        let session = Session::default();
        eprintln!("protected_session_catalog_requested=true");
        let catalog: Collection<Source> = session
            .get_json(&client, SOURCES)
            .await
            .map_err(LibraryError::SessionRead)?;
        let [matched, valid, invalid, missing, repaired] = compare_catalogs(&indexed, &catalog)?;
        eprintln!("catalog_sources_count={}", catalog.items.len());
        eprintln!("catalog_ids_with_config_match_count={matched}");
        eprintln!("catalog_ids_with_valid_config_metadata_count={valid}");
        eprintln!("catalog_invalid_metadata_count={invalid}");
        eprintln!("catalog_missing_attribution_count={missing}");
        eprintln!("missing_attribution_with_valid_config_metadata_count={repaired}");
        eprintln!("ownership_requested=false; raw_results_persisted=false");
        if valid != catalog.items.len() {
            return Err(LibraryError::InvalidMetadata);
        }
        Ok(())
    })
    .await
    .map_err(|_| LibraryError::TimedOut)?
}

/// Gates this category diagnostic separately from every consumed live scope.
fn category_comparison_approved(value: Option<&str>, isolated: bool) -> bool {
    value == Some("issue-32-category-failure-classes-three-attempts") && !isolated
}

/// Resolves hidden source IDs through unique named categories, not hardcoded IDs or missing links.
fn hidden_source_ids(
    categories: &Value,
    sources: &HashMap<u64, &Value>,
) -> Result<HashSet<u64>, LibraryError> {
    let categories = categories.as_array().ok_or(LibraryError::InvalidMetadata)?;
    if categories.len() > MAX_CATALOG_RECORDS {
        return Err(LibraryError::LimitExceeded);
    }
    let mut known = HashSet::new();
    let mut hidden_category = None;
    for category in categories {
        let id = category["id"]
            .as_u64()
            .filter(|id| *id != 0)
            .ok_or(LibraryError::InvalidMetadata)?;
        let name = category["name"]
            .as_str()
            .filter(|name| valid_text(name))
            .ok_or(LibraryError::InvalidMetadata)?;
        if !known.insert(id) {
            return Err(LibraryError::ConflictingMetadata);
        }
        if name.eq_ignore_ascii_case("Hidden Sources") && hidden_category.replace(id).is_some() {
            return Err(LibraryError::ConflictingMetadata);
        }
    }
    let hidden_category = hidden_category.ok_or(LibraryError::InvalidMetadata)?;
    let mut hidden = HashSet::new();
    for (id, source) in sources {
        let category = source["sourceCategoryId"]
            .as_u64()
            .filter(|id| known.contains(id))
            .ok_or(LibraryError::InvalidMetadata)?;
        if category == hidden_category {
            hidden.insert(*id);
        }
    }
    Ok(hidden)
}

/// Contains only counts and fixed failure labels, never source metadata or account identifiers.
#[derive(Debug, Default, Eq, PartialEq)]
struct CategoryComparison {
    catalog: usize,
    hidden: usize,
    missing: usize,
    missing_hidden: usize,
    remaining_invalid_library: usize,
    remaining_invalid_config: usize,
    remaining_unresolved: usize,
    remaining_invalid_proposed_library: usize,
    remaining_invalid_proposed_config: usize,
    remaining_unrepaired: usize,
    library_failures: BTreeMap<&'static str, usize>,
    config_failures: BTreeMap<&'static str, usize>,
}

impl CategoryComparison {
    /// Requires observed missing links, their category coverage, and nonempty valid remaining metadata.
    fn hypothesis_holds(&self) -> bool {
        self.missing != 0
            && self.missing == self.missing_hidden
            && self.hidden < self.catalog
            && self.remaining_unresolved == 0
    }

    /// Requires observed category coverage and complete metadata under only the approved trial repairs.
    fn proposal_holds(&self) -> bool {
        self.missing != 0
            && self.missing == self.missing_hidden
            && self.hidden < self.catalog
            && self.remaining_unrepaired == 0
    }
}

/// Compares every raw catalog record without dropping duplicates, unmatched IDs, or invalid metadata.
fn compare_categories(
    config: &HashMap<u64, &Value>,
    hidden: &HashSet<u64>,
    catalog: &Collection<Source>,
) -> Result<CategoryComparison, LibraryError> {
    catalog.validate(MAX_CATALOG_RECORDS)?;
    let mut ids = HashSet::new();
    let mut counts = CategoryComparison {
        catalog: catalog.items.len(),
        ..CategoryComparison::default()
    };
    for source in &catalog.items {
        if source.id == 0 || !ids.insert(source.id) {
            return Err(LibraryError::ConflictingMetadata);
        }
        let matched = config
            .get(&source.id)
            .ok_or(LibraryError::InvalidMetadata)?;
        let is_hidden = hidden.contains(&source.id);
        let missing =
            catalog_diagnostic(source).contains(&"no_book_path_or_marketplace_attribution");
        counts.hidden += usize::from(is_hidden);
        counts.missing += usize::from(missing);
        counts.missing_hidden += usize::from(missing && is_hidden);
        if !is_hidden {
            let library_valid = source.record(AccessEvidence::CatalogOnly).is_ok();
            let config_valid = config_book(matched).is_ok();
            counts.remaining_invalid_library += usize::from(!library_valid);
            counts.remaining_invalid_config += usize::from(!config_valid);
            counts.remaining_unresolved += usize::from(!library_valid && !config_valid);
            let library_proposed = library_valid || proposed_released_metadata(source).is_ok();
            let config_proposed = config_valid || proposed_config_metadata(matched).is_ok();
            counts.remaining_invalid_proposed_library += usize::from(!library_proposed);
            counts.remaining_invalid_proposed_config += usize::from(!config_proposed);
            counts.remaining_unrepaired += usize::from(!library_proposed && !config_proposed);
            for flag in catalog_diagnostic(source) {
                *counts.library_failures.entry(flag).or_default() += 1;
            }
            for flag in config_diagnostic(matched) {
                *counts.config_failures.entry(flag).or_default() += 1;
            }
        }
    }
    Ok(counts)
}

/// Supplies only invented categories and records for the category-join checks.
fn category_config() -> Value {
    json!({
        "sourceCategories": [
            {"id": 41, "name": "hIdDeN sOuRcEs"},
            {"id": 73, "name": "Invented visible category"}
        ],
        "sources": [
            {"id": 1, "sourceCategoryId": 41, "description": "Invented hidden source"},
            {"id": 2, "sourceCategoryId": 73, "description": "Invented visible source",
             "sourceURL": "sources/dnd/invented-2"}
        ]
    })
}

/// Checks independent approval and exact category matching without credentials or network requests.
#[test]
fn category_comparison_guards_use_synthetic_records() {
    for value in [
        None,
        Some("issue-32-config-comparison-three-attempts"),
        Some("issue-32-corrected-config-comparison-three-attempts"),
        Some("issue-32-released-library-ten-attempts"),
        Some("issue-32-hidden-category-three-attempts"),
    ] {
        assert!(!category_comparison_approved(value, false));
    }
    let approval = Some("issue-32-category-failure-classes-three-attempts");
    assert!(!category_comparison_approved(approval, true));
    assert!(category_comparison_approved(approval, false));
    let config = category_config();
    let indexed = config_sources(&config).expect("invented config");
    let hidden =
        hidden_source_ids(&config["sourceCategories"], &indexed).expect("invented categories");
    assert_eq!(hidden, HashSet::from([1]));
    let mut missing = source(1);
    missing["slug"] = Value::Null;
    let catalog =
        serde_json::from_value(collection(vec![missing, source(2)])).expect("invented catalog");
    let counts = compare_categories(&indexed, &hidden, &catalog).expect("invented comparison");
    assert_eq!(
        counts,
        CategoryComparison {
            catalog: 2,
            hidden: 1,
            missing: 1,
            missing_hidden: 1,
            ..CategoryComparison::default()
        }
    );
    assert!(counts.hypothesis_holds());
    assert!(counts.proposal_holds());
}

/// Rejects missing, duplicate, malformed, oversized, or unjoined categories before native access.
#[test]
fn category_comparison_rejects_invalid_config() {
    for categories in [
        Value::Null,
        json!([]),
        json!([{"id": 41, "name": "Hidden Sources"}]),
        json!([{"id": 0, "name": "Hidden Sources"}]),
        json!([{"id": "41", "name": "Hidden Sources"}]),
        json!([{"id": 41, "name": " Hidden Sources "}]),
        json!([{"id": 41, "name": "Hidden\nSources"}]),
        json!([{"id": 41, "name": true}]),
        json!([
            {"id": 41, "name": "Hidden Sources"},
            {"id": 41, "name": "Invented category"}
        ]),
        json!([
            {"id": 41, "name": "Hidden Sources"},
            {"id": 73, "name": "hidden sources"}
        ]),
        json!(vec![
            json!({"id": 41, "name": "Hidden Sources"});
            MAX_CATALOG_RECORDS + 1
        ]),
    ] {
        let mut config = category_config();
        config["sourceCategories"] = categories;
        assert!(
            config_sources(&config)
                .and_then(|indexed| hidden_source_ids(&config["sourceCategories"], &indexed))
                .is_err()
        );
    }
    for category in [Value::Null, json!(0), json!(-1), json!("73"), json!(999)] {
        let mut config = category_config();
        config["sources"][1]["sourceCategoryId"] = category;
        let indexed = config_sources(&config).expect("invented config");
        assert!(hidden_source_ids(&config["sourceCategories"], &indexed).is_err());
    }
    let mut duplicate = category_config();
    duplicate["sources"][1]["id"] = json!(1);
    assert!(config_sources(&duplicate).is_err());
}

/// Keeps unresolved visible records, whole-response limits, and completeness in the diagnostic.
#[test]
fn category_comparison_does_not_hide_invalid_records() {
    let mut config = category_config();
    config["sources"][1]["sourceURL"] = Value::Null;
    let indexed = config_sources(&config).expect("invented config");
    let hidden =
        hidden_source_ids(&config["sourceCategories"], &indexed).expect("invented categories");
    let mut missing = source(2);
    missing["slug"] = Value::Null;
    let catalog =
        serde_json::from_value(collection(vec![source(1), missing])).expect("invented catalog");
    let counts = compare_categories(&indexed, &hidden, &catalog).expect("invented comparison");
    assert_eq!(counts.missing, 1);
    assert_eq!(counts.missing_hidden, 0);
    assert_eq!(counts.remaining_invalid_library, 1);
    assert_eq!(counts.remaining_invalid_config, 1);
    assert_eq!(counts.remaining_unresolved, 1);
    assert!(!counts.hypothesis_holds());
    assert_eq!(counts.remaining_unrepaired, 1);
    assert!(!counts.proposal_holds());
    for items in [
        vec![source(1), source(1)],
        vec![source(0)],
        vec![source(999)],
        vec![source(1); MAX_CATALOG_RECORDS + 1],
    ] {
        let catalog = serde_json::from_value(collection(items)).expect("invented catalog");
        assert!(compare_categories(&indexed, &hidden, &catalog).is_err());
    }
    let mut incomplete = collection(vec![source(1)]);
    incomplete["totalCount"] = json!(2);
    let incomplete = serde_json::from_value(incomplete).expect("invented catalog");
    assert!(compare_categories(&indexed, &hidden, &incomplete).is_err());
    assert!(!CategoryComparison::default().hypothesis_holds());
    assert!(!CategoryComparison::default().proposal_holds());
    assert!(
        !CategoryComparison {
            catalog: 1,
            hidden: 1,
            missing: 1,
            missing_hidden: 1,
            ..CategoryComparison::default()
        }
        .hypothesis_holds()
    );
}

/// Rejects invalid visible metadata even when the category explains every missing link.
#[test]
fn category_comparison_requires_valid_remaining_metadata() {
    let mut config = category_config();
    config["sources"][1]["description"] = json!(" Invalid invented title ");
    let indexed = config_sources(&config).expect("invented config");
    let hidden =
        hidden_source_ids(&config["sourceCategories"], &indexed).expect("invented categories");
    let mut missing = source(1);
    missing["slug"] = Value::Null;
    let mut invalid = source(2);
    invalid["name"] = json!(" Invalid invented title ");
    let catalog =
        serde_json::from_value(collection(vec![missing, invalid])).expect("invented catalog");
    let counts = compare_categories(&indexed, &hidden, &catalog).expect("invented comparison");
    assert_eq!(counts.missing, counts.missing_hidden);
    assert_eq!(counts.remaining_invalid_library, 1);
    assert_eq!(counts.remaining_invalid_config, 1);
    assert_eq!(counts.remaining_unresolved, 1);
    assert!(!counts.hypothesis_holds());
    assert_eq!(counts.library_failures.get("title_padded"), Some(&1));
    assert_eq!(counts.config_failures.get("title_padded"), Some(&1));
    assert_eq!(counts.remaining_unrepaired, 0);
    assert!(counts.proposal_holds());
}

/// Covers actionable title/link classes and rejects repairs that would invent missing metadata.
#[test]
fn metadata_failure_classes_and_trial_repairs_are_strict() {
    let config = category_config()["sources"][1].clone();
    for (title, flag, repairable) in [
        ("", "title_empty", false),
        ("  Invented\n title\t ", "title_padded", true),
        ("Invented\u{1b} title", "title_control_character", false),
    ] {
        let mut value = config.clone();
        value["description"] = json!(title);
        assert!(config_diagnostic(&value).contains(&flag));
        assert_eq!(proposed_config_metadata(&value).is_ok(), repairable);
        let mut value = source(2);
        value["name"] = json!(title);
        let source: Source = serde_json::from_value(value).expect("invented source");
        assert!(catalog_diagnostic(&source).contains(&flag));
        assert_eq!(proposed_released_metadata(&source).is_ok(), repairable);
    }
    for (link, flag) in [
        (Value::Null, "link_missing_or_wrong_type"),
        (json!("//example.invalid/invented"), "link_scheme_relative"),
        (json!("sources/dnd/../invented"), "source_path_invalid"),
        (
            json!("https://example.invalid/sources/invented"),
            "link_location_not_approved",
        ),
        (
            json!("http://www.dndbeyond.com/sources/invented"),
            "link_scheme_or_port_invalid",
        ),
        (
            json!("https://www.dndbeyond.com:444/sources/invented"),
            "link_scheme_or_port_invalid",
        ),
        (
            json!("https://www.dndbeyond.com/sources/invented#fragment"),
            "link_credentials_or_fragment",
        ),
        (
            json!("https://invented@www.dndbeyond.com/sources/invented"),
            "link_credentials_or_fragment",
        ),
        (json!("invalid supplied link"), "link_parse_failed"),
        (
            json!("https://www.dndbeyond.com/sources/invented\n"),
            "link_invalid_text",
        ),
    ] {
        let mut value = config.clone();
        value["sourceURL"] = link;
        assert!(config_diagnostic(&value).contains(&flag));
        assert!(proposed_config_metadata(&value).is_err());
    }
    let mut missing_title = config.clone();
    missing_title["description"] = Value::Null;
    assert!(config_diagnostic(&missing_title).contains(&"title_missing_or_wrong_type"));
    assert!(proposed_config_metadata(&missing_title).is_err());
    let mut legacy = source(2);
    legacy["slug"] = Value::Null;
    legacy["marketplaceUrl"] = json!("https://www.dndbeyond.com/marketplace/invented");
    let value: Source = serde_json::from_value(legacy.clone()).expect("invented source");
    assert!(value.record(AccessEvidence::CatalogOnly).is_err());
    assert!(catalog_diagnostic(&value).contains(&"marketplace_url_uses_www_host"));
    assert!(proposed_released_metadata(&value).is_ok());
    legacy["relativeSlugPath"] = json!("sources/dnd/../invented");
    let value = serde_json::from_value(legacy).expect("invented source");
    assert!(proposed_released_metadata(&value).is_err());
}

/// Ensures diagnostic labels contain no title, URL, or arbitrary supplied value.
#[test]
fn metadata_failure_classes_are_value_free() {
    let mut value = category_config()["sources"][1].clone();
    value["description"] = json!("  invented-private-marker ");
    value["sourceURL"] = json!("https://example.invalid/invented-private-marker");
    let flags = config_diagnostic(&value);
    assert!(flags.contains(&"title_padded"));
    assert!(flags.contains(&"link_location_not_approved"));
    assert!(!format!("{flags:?}").contains("invented-private-marker"));
    let mut value = source(2);
    value["name"] = json!("  invented-private-marker ");
    value["slug"] = Value::Null;
    value["marketplaceUrl"] = json!("https://example.invalid/invented-private-marker");
    let value = serde_json::from_value(value).expect("invented source");
    assert!(!format!("{:?}", catalog_diagnostic(&value)).contains("invented-private-marker"));
    assert!(config_diagnostic(&category_config()["sources"][1]).is_empty());
}

/// Uses only the separately approved public-config, exchange, and catalog attempts.
#[cfg(windows)]
#[tokio::test]
#[ignore = "Requires separate approval for the three-attempt metadata failure-class diagnostic"]
async fn approved_live_category_comparison() -> Result<(), LibraryError> {
    let approval = std::env::var("BEYOND_GATE_LIBRARY_PROBE_APPROVAL").ok();
    assert!(
        category_comparison_approved(approval.as_deref(), cfg!(feature = "e2e-isolation")),
        "separate metadata failure-class diagnostic approval required"
    );
    timeout(Duration::from_secs(60), async {
        let public = HttpClient::single_attempt(&["https://www.dndbeyond.com"])
            .map_err(LibraryError::PageRead)?;
        eprintln!("public_config_requested=true; protected_storage_accessed=false");
        let config: Value = public
            .get_json("https://www.dndbeyond.com/api/config/json", None)
            .await
            .map_err(LibraryError::PageRead)?;
        let indexed = config_sources(&config)?;
        let hidden = hidden_source_ids(&config["sourceCategories"], &indexed)?;
        eprintln!("config_sources_count={}", indexed.len());
        eprintln!("config_hidden_sources_count={}", hidden.len());
        let client = HttpClient::single_attempt(&[ORIGIN]).map_err(LibraryError::PageRead)?;
        let session = Session::default();
        eprintln!("protected_session_catalog_requested=true");
        let catalog: Collection<Source> = session
            .get_json(&client, SOURCES)
            .await
            .map_err(LibraryError::SessionRead)?;
        let counts = compare_categories(&indexed, &hidden, &catalog)?;
        for (label, count) in [
            ("catalog_sources_count", counts.catalog),
            ("catalog_hidden_sources_count", counts.hidden),
            ("catalog_missing_attribution_count", counts.missing),
            ("missing_attribution_hidden_count", counts.missing_hidden),
            (
                "remaining_invalid_library_metadata_count",
                counts.remaining_invalid_library,
            ),
            (
                "remaining_invalid_config_metadata_count",
                counts.remaining_invalid_config,
            ),
            (
                "remaining_unresolved_metadata_count",
                counts.remaining_unresolved,
            ),
            (
                "remaining_invalid_proposed_library_metadata_count",
                counts.remaining_invalid_proposed_library,
            ),
            (
                "remaining_invalid_proposed_config_metadata_count",
                counts.remaining_invalid_proposed_config,
            ),
            (
                "remaining_unrepaired_metadata_count",
                counts.remaining_unrepaired,
            ),
        ] {
            eprintln!("{label}={count}");
        }
        for (flag, count) in &counts.library_failures {
            eprintln!("remaining_library_failure_{flag}_count={count}");
        }
        for (flag, count) in &counts.config_failures {
            eprintln!("remaining_config_failure_{flag}_count={count}");
        }
        eprintln!("category_hypothesis_holds={}", counts.hypothesis_holds());
        eprintln!("proposed_metadata_holds={}", counts.proposal_holds());
        eprintln!(
            "ownership_requested=false; raw_results_persisted=false; diagnostic_scope_consumed=true"
        );
        if !counts.proposal_holds() {
            return Err(LibraryError::InvalidMetadata);
        }
        Ok(())
    })
    .await
    .map_err(|_| LibraryError::TimedOut)?
}

/// Gates only the new ownership diagnostic, rejecting every consumed acknowledgment.
fn ownership_diagnostic_approved(value: Option<&str>, isolated: bool) -> bool {
    value == Some("issue-32-ownership-membership-five-attempts") && !isolated
}

/// Classifies unmatched ownership without printing IDs or relaxing production validation.
fn ownership_membership_counts(
    mut catalog: Collection<Source>,
    ownership: Collection<OwnershipRecord>,
    selection: &HashMap<u64, bool>,
) -> Result<BTreeMap<&'static str, usize>, LibraryError> {
    validate_catalog(&mut catalog, selection)?;
    ownership.validate(MAX_ENTRIES_PER_PAGE)?;
    let catalog_ids: HashSet<_> = catalog.items.iter().map(|source| source.id).collect();
    let ownership_ids: HashSet<_> = ownership.items.iter().map(|record| record.id).collect();
    let mut counts = BTreeMap::from([
        ("catalog_raw_records", catalog.items.len()),
        ("catalog_unique_ids", catalog_ids.len()),
        (
            "catalog_visible_ids",
            catalog_ids
                .iter()
                .filter(|id| selection.get(id) == Some(&true))
                .count(),
        ),
        ("ownership_raw_records", ownership.items.len()),
        ("ownership_unique_ids", ownership_ids.len()),
        (
            "ownership_raw_observations",
            ownership
                .items
                .iter()
                .map(|record| record.ownership.len())
                .sum(),
        ),
        ("matched_ownership_ids", 0),
        ("unmatched_hidden_ownership_ids", 0),
        ("unmatched_visible_ownership_ids", 0),
        ("unmatched_unconfigured_ownership_ids", 0),
    ]);
    for id in &ownership_ids {
        let label = if catalog_ids.contains(id) {
            "matched_ownership_ids"
        } else {
            match selection.get(id) {
                Some(false) => "unmatched_hidden_ownership_ids",
                Some(true) => "unmatched_visible_ownership_ids",
                None => "unmatched_unconfigured_ownership_ids",
            }
        };
        *counts.entry(label).or_default() += 1;
    }
    let unmatched = ownership_ids.difference(&catalog_ids).count();
    let valid = match assemble(catalog, ownership, selection) {
        Ok(_) => true,
        Err(LibraryError::UnmatchedOwnership) if unmatched != 0 => false,
        Err(error) => return Err(error),
    };
    counts.insert("production_snapshot_valid", usize::from(valid));
    Ok(counts)
}

/// Exercises each membership class and keeps consistent duplicate ownership counts distinct.
#[test]
fn ownership_membership_diagnostic_classifies_without_granting() {
    let selection = HashMap::from([(1, false), (2, true), (3, true), (4, false)]);
    let catalog =
        serde_json::from_value(collection(vec![source(1), source(2)])).expect("invented catalog");
    let ownership = serde_json::from_value(collection(vec![
        owned(1, &["owned"]),
        owned(2, &["shared"]),
        owned(3, &["owned"]),
        owned(4, &["shared"]),
        owned(5, &[]),
        owned(4, &["shared"]),
    ]))
    .expect("invented ownership");
    let counts =
        ownership_membership_counts(catalog, ownership, &selection).expect("safe classification");
    assert_eq!(counts["catalog_raw_records"], 2);
    assert_eq!(counts["catalog_visible_ids"], 1);
    assert_eq!(counts["ownership_raw_records"], 6);
    assert_eq!(counts["ownership_unique_ids"], 5);
    assert_eq!(counts["ownership_raw_observations"], 5);
    assert_eq!(counts["matched_ownership_ids"], 2);
    assert_eq!(counts["unmatched_hidden_ownership_ids"], 1);
    assert_eq!(counts["unmatched_visible_ownership_ids"], 1);
    assert_eq!(counts["unmatched_unconfigured_ownership_ids"], 1);
    assert_eq!(counts["production_snapshot_valid"], 0);
    assert!(!format!("{counts:?}").contains("Invented"));
    let catalog = serde_json::from_value(collection(vec![source(2)])).expect("invented catalog");
    let ownership =
        serde_json::from_value(collection(vec![owned(2, &["owned"])])).expect("invented ownership");
    let counts =
        ownership_membership_counts(catalog, ownership, &selection).expect("valid complete join");
    assert_eq!(counts["production_snapshot_valid"], 1);
}

/// Rejects invalid evidence, counts, limits, and metadata before emitting any diagnostic counts.
#[test]
fn ownership_membership_diagnostic_preserves_failure_guards() {
    let selection = HashMap::from([(1, true), (2, false)]);
    for ownership in [
        collection(vec![owned(0, &["owned"])]),
        collection(vec![owned(2, &["unexpected"])]),
        collection(vec![owned(2, &["owned"]); MAX_ENTRIES_PER_PAGE + 1]),
        collection(vec![owned(2, &vec!["owned"; MAX_ENTRIES + 1])]),
        json!({"items":[owned(2, &["owned"])],"totalCount":2}),
        json!({"items":[],"totalCount":0,"hasMore":true}),
    ] {
        let catalog =
            serde_json::from_value(collection(vec![source(1)])).expect("invented catalog");
        let ownership = serde_json::from_value(ownership).expect("invented ownership");
        assert!(ownership_membership_counts(catalog, ownership, &selection).is_err());
    }
    let mut invalid = source(1);
    invalid["name"] = json!("Invented\u{1b} title");
    let catalog = serde_json::from_value(collection(vec![invalid])).expect("invented catalog");
    let ownership =
        serde_json::from_value(collection(vec![owned(2, &["owned"])])).expect("invented ownership");
    assert!(matches!(
        ownership_membership_counts(catalog, ownership, &selection),
        Err(LibraryError::InvalidMetadata)
    ));
}

/// Rejects old diagnostic and acceptance acknowledgments, missing permission, and isolated builds.
#[test]
fn ownership_membership_diagnostic_requires_new_approval() {
    for value in [
        None,
        Some("issue-32-category-failure-classes-three-attempts"),
        Some("issue-32-selected-library-cli-mcp-ten-attempts"),
    ] {
        assert!(!ownership_diagnostic_approved(value, false));
    }
    assert!(!ownership_diagnostic_approved(
        Some("issue-32-ownership-membership-five-attempts"),
        true
    ));
    assert!(ownership_diagnostic_approved(
        Some("issue-32-ownership-membership-five-attempts"),
        false
    ));
}

/// Uses public classification and two protected GETs only after separate native approval.
#[cfg(windows)]
#[tokio::test]
#[ignore = "Requires separate approval for the five-attempt ownership membership diagnostic"]
async fn approved_live_ownership_membership_diagnostic() -> Result<(), LibraryError> {
    let approval = std::env::var("BEYOND_GATE_LIBRARY_PROBE_APPROVAL").ok();
    assert!(
        ownership_diagnostic_approved(approval.as_deref(), cfg!(feature = "e2e-isolation")),
        "separate ownership membership diagnostic approval required"
    );
    timeout(Duration::from_secs(60), async {
        let public =
            HttpClient::single_attempt(&[CONFIG_ORIGIN]).map_err(LibraryError::PageRead)?;
        let configuration: Configuration = public
            .get_json(CONFIG, None)
            .await
            .map_err(LibraryError::PageRead)?;
        let selection = configuration.selection()?;
        let client = HttpClient::single_attempt(&[ORIGIN]).map_err(LibraryError::PageRead)?;
        let session = Session::default();
        let mut catalog: Collection<Source> = session
            .get_json(&client, SOURCES)
            .await
            .map_err(LibraryError::SessionRead)?;
        validate_catalog(&mut catalog, &selection)?;
        let ownership = session
            .get_json(&client, OWNERSHIP)
            .await
            .map_err(LibraryError::SessionRead)?;
        let counts = ownership_membership_counts(catalog, ownership, &selection)?;
        for (label, count) in counts {
            eprintln!("{label}={count}");
        }
        eprintln!(
            "production_changed=false; raw_results_persisted=false; diagnostic_scope_consumed=true"
        );
        Ok(())
    })
    .await
    .map_err(|_| LibraryError::TimedOut)?
}

/// Creates a source with original metadata and an explicit first-party-shaped path.
fn source(id: u64) -> Value {
    json!({"sourceId": id, "name": format!("Invented volume {id}"), "slug": format!("invented-{id}")})
}

/// Creates an account-level observation, not a fabricated campaign grant.
fn owned(id: u64, kinds: &[&str]) -> Value {
    json!({"sourceId": id, "ownership": kinds.iter().map(|kind| json!({"type": kind})).collect::<Vec<_>>()})
}

/// Supplies an explicitly complete upstream-shaped envelope.
fn collection(items: Vec<Value>) -> Value {
    json!({"totalCount": items.len(), "items": items})
}

/// Decodes invented inputs through the same wire types and complete assembly as discovery.
fn snapshot(sources: Value, ownership: Value) -> Result<LibrarySnapshot, LibraryError> {
    let mut catalog: Collection<Source> =
        serde_json::from_value(sources).expect("synthetic catalog JSON");
    let selection = catalog
        .items
        .iter()
        .map(|source| (source.id, true))
        .collect();
    validate_catalog(&mut catalog, &selection)?;
    assemble(
        catalog,
        serde_json::from_value(ownership).expect("synthetic ownership JSON"),
        &selection,
    )
}

/// Keeps purchase, account sharing, both, and unknown records distinct.
#[test]
fn account_evidence_and_catalog_visibility_stay_distinct() {
    let result = snapshot(
        collection((1..=4).map(source).collect()),
        collection(vec![
            owned(1, &["owned"]),
            owned(2, &["shared"]),
            owned(3, &["owned", "shared"]),
        ]),
    )
    .expect("complete invented library");
    assert_eq!(
        result.books().iter().map(Book::access).collect::<Vec<_>>(),
        [
            BookAccess::Purchased,
            BookAccess::Shared,
            BookAccess::PurchasedAndShared,
            BookAccess::Unknown
        ]
    );
    assert!(matches!(
        result.books()[1].evidence(),
        [AccessEvidence::SharedWithAccount {
            state: AccessState::Granted
        }]
    ));
}

/// Ignores catalog search flags and preserves explicit account-sharing denial separately.
#[test]
fn catalog_flags_and_account_denial_are_not_grants() {
    let mut record = source(1);
    record["isFree"] = json!(true);
    record["ownership"] = json!([{"type":"owned"}, {"type":"shared"}]);
    let result = snapshot(collection(vec![record]), collection(vec![])).expect("catalog only");
    assert_eq!(result.books()[0].access(), BookAccess::Unknown);
    let book = Book::from_record(BookRecord {
        id: "invented".to_owned(),
        title: "Invented volume".to_owned(),
        source_url: "https://www.dndbeyond.com/sources/dnd/invented".to_owned(),
        evidence: AccessEvidence::SharedWithAccount {
            state: AccessState::Unavailable,
        },
    })
    .expect("explicit observation");
    assert_eq!(book.access(), BookAccess::Unavailable);
}

/// Refuses missing or wrongly typed required upstream fields rather than inventing values.
#[test]
fn required_wire_fields_are_not_defaulted() {
    for value in [
        json!({"items":[]}),
        json!({"totalCount":"0","items":[]}),
        collection(vec![json!({"sourceId":true,"name":"Invented"})]),
        collection(vec![json!({"sourceId":1})]),
        json!({"totalCount":0,"items":[],"hasMore":"false"}),
    ] {
        assert!(serde_json::from_value::<Collection<Source>>(value).is_err());
    }
    for value in [
        collection(vec![json!({"sourceId":1})]),
        collection(vec![json!({"sourceId":1,"ownership":null})]),
        collection(vec![json!({"sourceId":1,"ownership":[{"type":true}]})]),
    ] {
        assert!(serde_json::from_value::<Collection<OwnershipRecord>>(value).is_err());
    }
    assert!(matches!(
        snapshot(collection(vec![source(0)]), collection(vec![])),
        Err(LibraryError::InvalidMetadata)
    ));
    assert!(matches!(
        snapshot(collection(vec![source(1)]), collection(vec![owned(0, &[])])),
        Err(LibraryError::InvalidEvidence)
    ));
}

/// Retains the approved marketplace attribution only when a book path is absent.
#[test]
fn marketplace_is_metadata_not_a_path_repair() {
    let mut record = source(1);
    record["slug"] = Value::Null;
    record["marketplaceUrl"] = json!("https://marketplace.dndbeyond.com/products/invented");
    let result =
        snapshot(collection(vec![record.clone()]), collection(vec![])).expect("supplied link");
    assert_eq!(
        result.books()[0].source_url().host_str(),
        Some("marketplace.dndbeyond.com")
    );
    assert_eq!(result.books()[0].access(), BookAccess::Unknown);
    for invalid in [
        "https://foreign.invalid/item",
        "http://marketplace.dndbeyond.com/item",
        "https://user@marketplace.dndbeyond.com/item",
        "https://marketplace.dndbeyond.com:444/item",
        "https://marketplace.dndbeyond.com/item#private",
        " https://marketplace.dndbeyond.com/item",
    ] {
        record["marketplaceUrl"] = json!(invalid);
        assert!(matches!(
            snapshot(collection(vec![record.clone()]), collection(vec![])),
            Err(LibraryError::InvalidMetadata)
        ));
    }
    record["marketplaceUrl"] = json!("https://marketplace.dndbeyond.com/products/invented");
    record["relativeSlugPath"] = json!("sources/../private");
    assert!(matches!(
        snapshot(collection(vec![record]), collection(vec![])),
        Err(LibraryError::InvalidMetadata)
    ));
}

/// Permits complete metadata above 100 records without creating invented pages.
#[test]
fn catalog_has_its_own_raw_bound() {
    let exact_ownership = snapshot(
        collection((1..=100).map(source).collect()),
        collection((1..=100).map(|id| owned(id, &["owned"])).collect()),
    )
    .expect("exact ownership bound");
    assert_eq!(exact_ownership.books().len(), 100);
    for count in [101, MAX_CATALOG_RECORDS] {
        let result = snapshot(
            collection((1..=count as u64).map(source).collect()),
            collection(vec![]),
        )
        .expect("bounded unpaged catalog");
        assert_eq!(result.books().len(), count);
    }
    let oversized = collection(vec![source(1); MAX_CATALOG_RECORDS + 1]);
    assert!(matches!(
        snapshot(oversized, collection(vec![])),
        Err(LibraryError::LimitExceeded)
    ));
    let evidence = collection((1..=101).map(|id| owned(id, &["owned"])).collect());
    assert!(matches!(
        snapshot(collection((1..=101).map(source).collect()), evidence),
        Err(LibraryError::LimitExceeded)
    ));
}

/// Rejects incomplete collections, explicit continuation, unknown evidence, and unmatched IDs.
#[test]
fn incomplete_and_unknown_contracts_fail() {
    let mut incomplete = collection(vec![source(1)]);
    incomplete["totalCount"] = json!(2);
    assert!(matches!(
        snapshot(incomplete, collection(vec![])),
        Err(LibraryError::IncompleteDiscovery)
    ));
    for (field, value) in [
        ("nextCursor", json!("opaque")),
        ("pagination", json!({})),
        ("hasMore", json!(true)),
    ] {
        let mut continued = collection(vec![source(1)]);
        continued[field] = value;
        assert!(matches!(
            snapshot(continued, collection(vec![])),
            Err(LibraryError::InvalidPagination)
        ));
    }
    assert!(matches!(
        snapshot(
            collection(vec![source(1)]),
            collection(vec![owned(2, &["owned"])])
        ),
        Err(LibraryError::UnmatchedOwnership)
    ));
    assert!(matches!(
        snapshot(
            collection(vec![source(1)]),
            collection(vec![owned(1, &["unexpected"])])
        ),
        Err(LibraryError::InvalidEvidence)
    ));
}

/// Preserves conflict handling and counts evidence duplicates before removal.
#[test]
fn duplicates_cannot_hide_conflicts_or_limits() {
    let exact = snapshot(
        collection(vec![source(1)]),
        collection(vec![owned(1, &vec!["owned"; MAX_ENTRIES])]),
    )
    .expect("exact raw evidence bound");
    assert_eq!(exact.books()[0].evidence().len(), 1);
    assert!(matches!(
        snapshot(
            collection((1..=MAX_CATALOG_RECORDS as u64).map(source).collect()),
            collection(vec![owned(1, &["owned", "shared"])]),
        ),
        Err(LibraryError::LimitExceeded)
    ));
    let mut conflict = source(1);
    conflict["name"] = json!("Different invented volume");
    assert!(matches!(
        snapshot(collection(vec![source(1), conflict]), collection(vec![])),
        Err(LibraryError::ConflictingMetadata)
    ));
    let result = snapshot(
        collection(vec![source(1), source(1)]),
        collection(vec![owned(1, &["owned", "owned"])]),
    )
    .expect("consistent duplicates");
    assert_eq!(result.books().len(), 1);
    assert_eq!(result.books()[0].evidence().len(), 1);
    let excess = owned(1, &vec!["owned"; MAX_ENTRIES + 1]);
    assert!(matches!(
        snapshot(collection(vec![source(1)]), collection(vec![excess])),
        Err(LibraryError::LimitExceeded)
    ));
}

/// Builds bounded public classification without title or URL substitutes.
fn visible_configuration(count: usize) -> Value {
    json!({
        "sources": (1..=count).map(|id| json!({"id":id,"sourceCategoryId":73})).collect::<Vec<_>>(),
        "sourceCategories": [
            {"id":41,"name":"Hidden Sources"},
            {"id":73,"name":"Invented books"}
        ]
    })
}

/// Exercises the production classification, preparation, and full ownership join.
fn selected_snapshot(
    config: Value,
    catalog: Value,
    ownership: Value,
) -> Result<LibrarySnapshot, LibraryError> {
    let configuration: Configuration =
        serde_json::from_value(config).map_err(|_| LibraryError::InvalidMetadata)?;
    let selection = configuration.selection()?;
    let mut catalog = serde_json::from_value(catalog).map_err(|_| LibraryError::InvalidMetadata)?;
    validate_catalog(&mut catalog, &selection)?;
    assemble(
        catalog,
        serde_json::from_value(ownership).map_err(|_| LibraryError::InvalidEvidence)?,
        &selection,
    )
}

/// Keeps classification independent of attribution and validates evidence before Web selection.
#[test]
fn selected_books_validate_all_ownership_before_web_selection() {
    let mut hidden = source(1);
    hidden["name"] = json!("");
    hidden["slug"] = Value::Null;
    let mut visible = source(2);
    visible["name"] = json!(" \tInvented\n volume 2 ");
    let catalog = collection(vec![hidden.clone(), visible.clone()]);
    let result = selected_snapshot(
        category_config(),
        catalog.clone(),
        collection(vec![owned(1, &["owned", "shared"]), owned(2, &["shared"])]),
    )
    .expect("explicit book selection");
    assert_eq!(result.books().len(), 1);
    assert_eq!(result.books()[0].id(), "2");
    assert_eq!(result.books()[0].title(), "Invented volume 2");
    assert_eq!(
        result.books()[0].source_url().as_str(),
        "https://www.dndbeyond.com/sources/dnd/invented-2"
    );
    assert_eq!(result.books()[0].access(), BookAccess::Shared);
    assert!(matches!(
        selected_snapshot(
            category_config(),
            catalog.clone(),
            collection(vec![owned(1, &["unknown"])])
        ),
        Err(LibraryError::InvalidEvidence)
    ));
    let mut config = category_config();
    config["sources"]
        .as_array_mut()
        .expect("invented sources")
        .push(json!({"id":3,"sourceCategoryId":73}));
    let result = selected_snapshot(config, catalog, collection(vec![owned(3, &["owned"])]))
        .expect("configured account evidence outside the Web catalog");
    assert_eq!(result.books().len(), 1);
    assert_eq!(result.books()[0].id(), "2");
    assert_eq!(result.books()[0].access(), BookAccess::Unknown);
    let empty = selected_snapshot(
        category_config(),
        collection(vec![hidden.clone()]),
        collection(vec![owned(1, &["owned"])]),
    )
    .expect("known empty book selection");
    assert!(empty.books().is_empty());
    let duplicate = selected_snapshot(
        category_config(),
        collection(vec![hidden.clone(), hidden.clone()]),
        collection(vec![owned(1, &["owned"])]),
    )
    .expect("consistent hidden duplicates");
    assert!(duplicate.books().is_empty());
    assert!(matches!(
        selected_snapshot(
            category_config(),
            collection(vec![hidden.clone()]),
            collection(vec![owned(1, &vec!["owned"; MAX_ENTRIES + 1])])
        ),
        Err(LibraryError::LimitExceeded)
    ));
    assert!(matches!(
        selected_snapshot(
            category_config(),
            collection(vec![hidden.clone(), hidden.clone()]),
            collection(vec![owned(1, &vec!["owned"; MAX_ENTRIES / 2 + 1])])
        ),
        Err(LibraryError::LimitExceeded)
    ));
    let mut conflict = hidden.clone();
    conflict["name"] = json!("Conflicting hidden metadata");
    assert!(matches!(
        selected_snapshot(
            category_config(),
            collection(vec![hidden.clone(), conflict]),
            collection(vec![])
        ),
        Err(LibraryError::ConflictingMetadata)
    ));
    assert!(matches!(
        selected_snapshot(
            category_config(),
            collection(vec![hidden; MAX_CATALOG_RECORDS + 1]),
            collection(vec![])
        ),
        Err(LibraryError::LimitExceeded)
    ));
    visible["slug"] = Value::Null;
    assert!(matches!(
        selected_snapshot(
            category_config(),
            collection(vec![visible]),
            collection(vec![])
        ),
        Err(LibraryError::InvalidMetadata)
    ));
}

/// Keeps configured ownership-only sources outside Web results without discarding validation.
#[test]
fn web_catalog_scope_validates_ownership_only_sources() {
    let mut config = category_config();
    config["sources"]
        .as_array_mut()
        .expect("invented sources")
        .extend([
            json!({"id":3,"sourceCategoryId":73}),
            json!({"id":4,"sourceCategoryId":41}),
        ]);
    let catalog = collection(vec![source(2)]);
    let result = selected_snapshot(
        config.clone(),
        catalog.clone(),
        collection(vec![
            owned(2, &["owned"]),
            owned(3, &["owned", "shared"]),
            owned(4, &["owned"]),
        ]),
    )
    .expect("validated account evidence outside Web scope");
    assert_eq!(result.books().len(), 1);
    assert_eq!(result.books()[0].id(), "2");
    assert_eq!(result.books()[0].access(), BookAccess::Purchased);
    assert_eq!(result.books()[0].evidence().len(), 1);
    let empty = selected_snapshot(
        config.clone(),
        collection(vec![]),
        collection(vec![owned(3, &["owned"])]),
    )
    .expect("complete empty Web catalog");
    assert!(empty.books().is_empty());
    for (records, expected) in [
        (vec![owned(5, &["owned"])], "unconfigured"),
        (vec![owned(3, &["unexpected"])], "invalid"),
        (vec![owned(0, &["owned"])], "invalid"),
        (
            vec![owned(3, &["owned"]); MAX_ENTRIES_PER_PAGE + 1],
            "limit",
        ),
        (vec![owned(4, &vec!["owned"; MAX_ENTRIES + 1])], "limit"),
    ] {
        let error = selected_snapshot(config.clone(), catalog.clone(), collection(records))
            .expect_err("raw ownership validation precedes Web scope");
        assert!(match expected {
            "unconfigured" => matches!(error, LibraryError::UnmatchedOwnership),
            "invalid" => matches!(error, LibraryError::InvalidEvidence),
            "limit" => matches!(error, LibraryError::LimitExceeded),
            _ => false,
        });
    }
}

/// Normalizes only invalid title whitespace and never widens the URL policy.
#[test]
fn selected_title_cleanup_keeps_strict_metadata_contracts() {
    for (title, expected) in [
        (" Invented\n volume 2\t", Some("Invented volume 2")),
        ("Invented  volume 2", Some("Invented  volume 2")),
        (" \t\n ", None),
        ("Invented\u{1b} volume 2", None),
    ] {
        let mut visible = source(2);
        visible["name"] = json!(title);
        let result = selected_snapshot(
            category_config(),
            collection(vec![visible]),
            collection(vec![]),
        );
        if let Some(expected) = expected {
            assert_eq!(result.expect("valid title").books()[0].title(), expected);
        } else {
            assert!(matches!(result, Err(LibraryError::InvalidMetadata)));
        }
    }
    let mut visible = source(2);
    visible["relativeSlugPath"] = json!("sources/../invalid");
    visible["marketplaceUrl"] = json!("https://marketplace.dndbeyond.com/invented");
    assert!(
        selected_snapshot(
            category_config(),
            collection(vec![visible]),
            collection(vec![])
        )
        .is_err()
    );
    let mut visible = source(2);
    visible["slug"] = Value::Null;
    visible["marketplaceUrl"] = json!("https://www.dndbeyond.com/marketplace/invented");
    assert!(
        selected_snapshot(
            category_config(),
            collection(vec![visible]),
            collection(vec![])
        )
        .is_err()
    );
}

/// Rejects invalid public classification without guessing missing categories or source membership.
#[test]
fn production_configuration_is_complete_and_strict() {
    let config = category_config();
    let typed: Configuration =
        serde_json::from_value(config.clone()).expect("invented configuration");
    assert_eq!(
        typed.selection().expect("selection"),
        HashMap::from([(1, false), (2, true)])
    );
    let mut renamed = config.clone();
    renamed["sourceCategories"][0]["name"] = json!("hIdDeN sOuRcEs");
    renamed["sourceCategories"][0]["id"] = json!(999);
    renamed["sources"][0]["sourceCategoryId"] = json!(999);
    let renamed: Configuration =
        serde_json::from_value(renamed).expect("invented renamed category");
    assert_eq!(
        renamed.selection().expect("name-based selection"),
        HashMap::from([(1, false), (2, true)])
    );
    for categories in [
        Value::Null,
        json!([]),
        json!([{"id":41,"name":"Hidden Sources"},{"id":41,"name":"Other"}]),
        json!([{"id":41,"name":"Hidden Sources"},{"id":73,"name":"hidden sources"}]),
        json!([{"id":0,"name":"Hidden Sources"}]),
        json!([{"id":41,"name":" Hidden Sources "}]),
        json!([{"id":41,"name":"Hidden Sources"}]),
        json!(vec![
            json!({"id":41,"name":"Hidden Sources"});
            MAX_CATALOG_RECORDS + 1
        ]),
    ] {
        let mut bad = config.clone();
        bad["sourceCategories"] = categories;
        assert!(selected_snapshot(bad, collection(vec![]), collection(vec![])).is_err());
    }
    for records in [
        json!([{"id":0,"sourceCategoryId":41}]),
        json!([{"id":1,"sourceCategoryId":999}]),
        json!([{"id":1,"sourceCategoryId":41},{"id":1,"sourceCategoryId":73}]),
        json!([{"id":1,"sourceCategoryId":"41"}]),
        json!(vec![
            json!({"id":1,"sourceCategoryId":41});
            MAX_CATALOG_RECORDS + 1
        ]),
    ] {
        let mut bad = config.clone();
        bad["sources"] = records;
        assert!(selected_snapshot(bad, collection(vec![]), collection(vec![])).is_err());
    }
    assert!(selected_snapshot(config, collection(vec![source(3)]), collection(vec![])).is_err());
}

/// Owns an actual local service and separate public/protected request counters.
struct Mock {
    origin: String,
    requests: Arc<AtomicUsize>,
    public_requests: Arc<AtomicUsize>,
    worker: JoinHandle<()>,
}

impl Mock {
    /// Supplies valid public classification before the scheduled authenticated resource replies.
    async fn start(replies: Vec<(u16, String)>) -> Self {
        Self::with_config(
            (200, visible_configuration(MAX_CATALOG_RECORDS).to_string()),
            replies,
        )
        .await
    }

    /// Separates cookie-free public configuration from bearer-only resource requests.
    async fn with_config(config: (u16, String), replies: Vec<(u16, String)>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("loopback listener");
        let origin = format!("http://{}", listener.local_addr().expect("address"));
        let requests = Arc::new(AtomicUsize::new(0));
        let public_requests = Arc::new(AtomicUsize::new(0));
        let count = Arc::clone(&requests);
        let public_count = Arc::clone(&public_requests);
        let worker = tokio::spawn(async move {
            while let Ok((mut socket, _)) = listener.accept().await {
                let mut request = Vec::new();
                while !request.ends_with(b"\r\n\r\n") && request.len() < 16 * 1024 {
                    let mut byte = [0];
                    socket.read_exact(&mut byte).await.expect("request");
                    request.push(byte[0]);
                }
                let header = String::from_utf8(request)
                    .expect("synthetic header")
                    .to_ascii_lowercase();
                assert!(!header.contains("\r\ncookie:"));
                let public = header.starts_with("get /config ");
                assert_eq!(header.contains("authorization:"), !public);
                let (status, body) = if public {
                    public_count.fetch_add(1, Ordering::SeqCst);
                    &config
                } else {
                    let index = count.fetch_add(1, Ordering::SeqCst);
                    &replies[index.min(replies.len() - 1)]
                };
                if *status == 0 {
                    std::future::pending::<()>().await;
                }
                let response = format!(
                    "HTTP/1.1 {status} Synthetic\r\nContent-Type: application/json\r\nLocation: /must-not-follow\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                if let Err(error) = socket.write_all(response.as_bytes()).await {
                    assert!(matches!(
                        error.kind(),
                        std::io::ErrorKind::ConnectionReset
                            | std::io::ErrorKind::ConnectionAborted
                            | std::io::ErrorKind::BrokenPipe
                    ));
                }
            }
        });
        Self {
            origin,
            requests,
            public_requests,
            worker,
        }
    }

    /// Uses the real JSON and shared session paths, changing only the compiled test origin.
    async fn discover(&self, session: &Session) -> Result<LibrarySnapshot, LibraryError> {
        let client = HttpClient::for_fixture(&self.origin).expect("fixture client");
        timeout(
            Duration::from_secs(3),
            discover(
                session,
                &client,
                &client,
                &format!("{}/config", self.origin),
                &format!("{}/sources", self.origin),
                &format!("{}/ownership", self.origin),
            ),
        )
        .await
        .expect("bounded fixture discovery")
    }
}

impl Drop for Mock {
    /// Cancels only this test's listener and connection workers.
    fn drop(&mut self) {
        self.worker.abort();
    }
}

/// Stops invalid or denied public configuration before session access or protected resource requests.
#[tokio::test]
async fn configuration_failure_prevents_session_access() {
    for config in [
        (307, "{}".to_owned()),
        (403, "{}".to_owned()),
        (429, "{}".to_owned()),
        (503, "{}".to_owned()),
        (200, "invalid-invented-private-json".to_owned()),
        (
            200,
            visible_configuration(MAX_CATALOG_RECORDS + 1).to_string(),
        ),
        (200, json!({"sources":[],"sourceCategories":[]}).to_string()),
        (200, " ".repeat(1024 * 1024 + 1)),
    ] {
        let mock = Mock::with_config(config, vec![(200, collection(vec![]).to_string())]).await;
        let session = fixture::session(Scenario::Valid).expect("invented session");
        assert!(mock.discover(&session).await.is_err());
        assert_eq!(fixture::request_count(&session), 0);
        assert_eq!(mock.requests.load(Ordering::SeqCst), 0);
        assert_eq!(mock.public_requests.load(Ordering::SeqCst), 1);
    }
}

/// Cancels pending metadata without spending an ownership request or retaining work.
#[tokio::test]
async fn cancellation_stops_an_inflight_resource_read() {
    let mock = Mock::start(vec![(0, String::new())]).await;
    let count = Arc::clone(&mock.requests);
    let session = fixture::session(Scenario::Valid).expect("invented session");
    let read = tokio::spawn(async move { mock.discover(&session).await });
    timeout(Duration::from_secs(2), async {
        while count.load(Ordering::SeqCst) == 0 {
            tokio::time::sleep(Duration::from_millis(2)).await;
        }
    })
    .await
    .expect("metadata read started");
    read.abort();
    assert!(read.await.expect_err("cancelled discovery").is_cancelled());
    assert_eq!(count.load(Ordering::SeqCst), 1);
}

/// Reuses the same session but never retains a sharing observation across discoveries.
#[tokio::test]
async fn sharing_loss_is_fresh_with_a_reused_session() {
    let catalog = collection(vec![source(1), source(2)]).to_string();
    let mock = Mock::start(vec![
        (200, catalog.clone()),
        (
            200,
            collection(vec![owned(1, &["owned", "shared"]), owned(2, &["shared"])]).to_string(),
        ),
        (200, catalog),
        (200, collection(vec![owned(1, &["owned"])]).to_string()),
    ])
    .await;
    let session = fixture::session(Scenario::Valid).expect("invented session");
    let first = mock.discover(&session).await.expect("first library");
    let second = mock.discover(&session).await.expect("fresh library");
    assert_eq!(first.books()[0].access(), BookAccess::PurchasedAndShared);
    assert_eq!(second.books()[0].access(), BookAccess::Purchased);
    assert_eq!(second.books()[1].access(), BookAccess::Unknown);
    assert_eq!(mock.requests.load(Ordering::SeqCst), 4);
    assert_eq!(fixture::request_count(&session), 1);
}

/// Stops resource requests on transport, malformed wire, incomplete, and size failures.
#[tokio::test]
async fn failures_never_follow_or_retry_or_return_partial_results() {
    let catalog = collection(vec![source(1)]).to_string();
    for (status, body) in [
        (307, "{}".to_owned()),
        (401, "{}".to_owned()),
        (403, "{}".to_owned()),
        (429, "{}".to_owned()),
        (503, "{}".to_owned()),
        (200, "invalid-private-json".to_owned()),
        (
            200,
            json!({"items": [source(1)], "totalCount": 2}).to_string(),
        ),
        (200, " ".repeat(1024 * 1024 + 1)),
    ] {
        let mock = Mock::start(vec![(status, body)]).await;
        let session = fixture::session(Scenario::Valid).expect("invented session");
        let error = mock
            .discover(&session)
            .await
            .expect_err("whole discovery fails");
        assert!(!format!("{error} {error:?}").contains("private"));
        assert_eq!(mock.requests.load(Ordering::SeqCst), 1);
    }
    let mock = Mock::start(vec![(200, catalog), (403, "{}".to_owned())]).await;
    let session = fixture::session(Scenario::Valid).expect("invented session");
    assert!(mock.discover(&session).await.is_err());
    assert_eq!(mock.requests.load(Ordering::SeqCst), 2);
}
