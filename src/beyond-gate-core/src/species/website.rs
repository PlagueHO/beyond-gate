//! Parses bounded species metadata from the public website listing.

use super::{MAX_RECORDS, SpeciesError as Failure, WEBSITE, validate_id};
use crate::public_config::valid_text;
use scraper::{ElementRef, Html, Selector};
use serde_json::{Value, json};
use std::collections::HashSet;

/// Compiles only internal selectors without reflecting arbitrary page content.
fn selector(value: &str) -> Result<Selector, Failure> {
    Selector::parse(value).map_err(|_| Failure::InvalidResponse)
}

/// Requires exactly one known node rather than accepting an ambiguous structure.
fn one<'a>(mut nodes: impl Iterator<Item = ElementRef<'a>>) -> Result<ElementRef<'a>, Failure> {
    let node = nodes.next().ok_or(Failure::InvalidResponse)?;
    if nodes.next().is_some() {
        return Err(Failure::InvalidResponse);
    }
    Ok(node)
}

/// Normalizes metadata whitespace without returning arbitrary HTML or rules text.
fn text(node: ElementRef<'_>) -> Result<String, Failure> {
    let text = node
        .text()
        .collect::<Vec<_>>()
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if !valid_text(&text) {
        return Err(Failure::InvalidResponse);
    }
    Ok(text)
}

/// Parses the observed terminal species page; no filter or page is inferred.
pub(super) fn parse(bytes: &[u8]) -> Result<Value, Failure> {
    let body = std::str::from_utf8(bytes).map_err(|_| Failure::InvalidResponse)?;
    let document = Html::parse_document(body);
    if document
        .select(&selector(
            "#challenge-form, script[src*='/cdn-cgi/challenge-platform/']",
        )?)
        .next()
        .is_some()
    {
        return Err(Failure::UnsupportedOperation);
    }
    if document
        .select(&selector(".b-pagination-list a, a[rel='next']")?)
        .next()
        .is_some()
    {
        return Err(Failure::UnsupportedOperation);
    }

    let containers_selector =
        selector("div.listing-container.listing-container-ul.RPGRace-listing.card-listing")?;
    let containers: Vec<_> = document.select(&containers_selector).collect();
    let listings = selector("ul.listing.listing-rpgrace.rpgrace-listing")?;
    if containers.is_empty() || document.select(&listings).count() != containers.len() {
        return Err(Failure::IncompleteResult);
    }

    let mut ids = HashSet::new();
    let mut result = Vec::new();
    let links = selector("a.listing-card__link[href]")?;
    let titles = selector(".listing-card__title")?;
    let sources = selector(".listing-card__source")?;
    let legacy_labels = selector(".badge-label")?;
    let footer = selector(".listing-footer")?;
    for container in containers {
        let listing = one(container.select(&listings))?;
        one(container.select(&footer))?;
        let rows: Vec<_> = listing.children().filter_map(ElementRef::wrap).collect();
        if rows.is_empty() {
            return Err(Failure::IncompleteResult);
        }
        if result.len().saturating_add(rows.len()) > MAX_RECORDS {
            return Err(Failure::LimitExceeded);
        }
        for row in rows {
            if row.value().name() != "li"
                || !row
                    .value()
                    .has_class("listing-card", scraper::CaseSensitivity::CaseSensitive)
            {
                return Err(Failure::InvalidResponse);
            }
            let link = one(row.select(&links))?;
            let href = link.value().attr("href").ok_or(Failure::InvalidResponse)?;
            let source_url = source_url(href)?;
            let url = reqwest::Url::parse(&source_url).map_err(|_| Failure::InvalidResponse)?;
            let id = source_id(url.path())?;
            if !ids.insert(id.clone()) {
                return Err(Failure::ConflictingIdentity);
            }
            let name = text(one(row.select(&titles))?)?;
            let source = text(one(row.select(&sources))?)?;
            let is_legacy = row
                .select(&legacy_labels)
                .any(|label| label.text().collect::<String>().trim() == "Legacy");
            result.push(json!({
                "id": id,
                "name": name,
                "source": source,
                "source_url": source_url,
                "is_legacy": is_legacy
            }));
        }
    }
    if result.is_empty() {
        return Err(Failure::IncompleteResult);
    }
    Ok(Value::Array(result))
}

/// Validates a canonical species detail link without using it for a request.
fn source_url(text: &str) -> Result<String, Failure> {
    if !valid_text(text) || text.contains('\\') || text.contains('%') {
        return Err(Failure::InvalidResponse);
    }
    let absolute = if text.starts_with("/species/") {
        format!("{WEBSITE}{text}")
    } else {
        text.to_owned()
    };
    let url = reqwest::Url::parse(&absolute).map_err(|_| Failure::InvalidResponse)?;
    if url.as_str() != absolute
        || url.origin().ascii_serialization() != WEBSITE
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(Failure::InvalidResponse);
    }
    source_id(url.path())?;
    Ok(url.to_string())
}

/// Requires the displayed URL to contain one canonical species ID and slug.
fn source_id(url: &str) -> Result<String, Failure> {
    let (id, slug) = url
        .strip_prefix("/species/")
        .and_then(|path| path.split_once('-'))
        .ok_or(Failure::InvalidResponse)?;
    validate_id(id).map_err(|_| Failure::InvalidResponse)?;
    if slug.is_empty()
        || !slug
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    {
        return Err(Failure::InvalidResponse);
    }
    Ok(id.to_owned())
}

/// Restricts cookie transport to the fixed, unfiltered species listing page.
pub(crate) fn allowed_list_url(url: &reqwest::Url) -> bool {
    url.path() == "/species" && url.query().is_none() && url.fragment().is_none()
}
