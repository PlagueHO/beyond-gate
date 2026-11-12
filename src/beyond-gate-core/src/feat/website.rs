//! Strict feat GET form, result structure, explicit empty marker, and metadata parsing.

use super::{FeatError as Failure, FeatQuery, MAX_RECORDS, WEBSITE, source_url, validate_id};
use crate::public_config::valid_text;
use scraper::{ElementRef, Html, Selector};
use serde_json::{Value, json};
use std::collections::HashSet;

/// Compiles internal selectors without printing arbitrary HTML in errors.
fn selector(value: &str) -> Result<Selector, Failure> {
    Selector::parse(value).map_err(|_| Failure::InvalidResponse)
}

/// Requires exactly one known element instead of accepting ambiguous structure.
fn one<'a>(mut nodes: impl Iterator<Item = ElementRef<'a>>) -> Result<ElementRef<'a>, Failure> {
    let node = nodes.next().ok_or(Failure::InvalidResponse)?;
    if nodes.next().is_some() {
        return Err(Failure::InvalidResponse);
    }
    Ok(node)
}

/// Checks requested values against the same GET form's options and echoed selectors.
fn filters(document: &Html, query: &FeatQuery) -> Result<(), Failure> {
    let forms = selector("form")?;
    let form = one(document.select(&forms).filter(|f| {
        matches!(
            f.value().attr("action"),
            Some("/feats" | "https://www.dndbeyond.com/feats")
        )
    }))?;
    if !form
        .value()
        .attr("method")
        .is_some_and(|m| m.eq_ignore_ascii_case("get"))
    {
        return Err(Failure::InvalidResponse);
    }
    let controls = selector("[name]")?;
    let options = selector("option")?;
    for (key, value) in query.website_url(WEBSITE)?.query_pairs() {
        let control = one(form
            .select(&controls)
            .filter(|c| c.value().attr("name") == Some(key.as_ref())))?;
        if key == "filter-name" {
            if control.value().name() != "input"
                || control.value().attr("type") != Some("text")
                || control.value().attr("value") != Some(value.as_ref())
            {
                return Err(Failure::InvalidResponse);
            }
        } else {
            if control.value().name() != "select" {
                return Err(Failure::InvalidResponse);
            }
            if !control
                .select(&options)
                .any(|o| o.value().attr("value") == Some(value.as_ref()))
            {
                return Err(Failure::InvalidInput);
            }
            if !control.select(&options).any(|o| {
                o.value().attr("value") == Some(value.as_ref())
                    && o.value().attr("selected").is_some()
            }) {
                return Err(Failure::IncompleteResult);
            }
        }
    }
    Ok(())
}

/// Normalizes harmless metadata whitespace without exposing HTML or rules text.
fn text(node: ElementRef<'_>) -> Result<String, Failure> {
    let value = node
        .text()
        .collect::<Vec<_>>()
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if !valid_text(&value) {
        return Err(Failure::InvalidResponse);
    }
    Ok(value)
}

/// Parses one terminal listing; unknown paging and empty structure never imply success.
pub(super) fn parse(bytes: &[u8], query: &FeatQuery) -> Result<Value, Failure> {
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
    filters(&document, query)?;
    if document
        .select(&selector(".b-pagination-list a, a[rel='next']")?)
        .next()
        .is_some()
    {
        return Err(Failure::UnsupportedOperation);
    }
    let listing = one(document.select(&selector("ul.listing.listing-rpgfeat.rpgfeat-listing")?))?;
    one(document.select(&selector(".listing-footer")?))?;
    let rows: Vec<_> = listing.children().filter_map(ElementRef::wrap).collect();
    if rows.len() > MAX_RECORDS {
        return Err(Failure::LimitExceeded);
    }
    let links = selector(".list-row-name-primary-text a[href]")?;
    if rows.len() == 1
        && rows[0]
            .value()
            .has_class("no-results", scraper::CaseSensitivity::CaseSensitive)
    {
        if rows[0].value().name() != "li"
            || !rows[0]
                .value()
                .has_class("alert", scraper::CaseSensitivity::CaseSensitive)
            || rows[0].select(&links).next().is_some()
        {
            return Err(Failure::InvalidResponse);
        }
        return Ok(json!([]));
    }
    if rows.is_empty() {
        return Err(Failure::IncompleteResult);
    }
    let source = selector(".list-row-name-secondary-text")?;
    let mut ids = HashSet::new();
    let mut result = Vec::new();
    for row in rows {
        if row.value().name() != "li" {
            return Err(Failure::InvalidResponse);
        }
        let link = one(row.select(&links))?;
        let href = link.value().attr("href").ok_or(Failure::InvalidResponse)?;
        let absolute = if href.starts_with("/feats/") {
            format!("{WEBSITE}{href}")
        } else {
            href.to_owned()
        };
        let url = reqwest::Url::parse(&absolute).map_err(|_| Failure::InvalidResponse)?;
        let id = url
            .path()
            .strip_prefix("/feats/")
            .and_then(|s| s.split_once('-'))
            .map(|(id, _)| id)
            .ok_or(Failure::InvalidResponse)?;
        validate_id(id).map_err(|_| Failure::InvalidResponse)?;
        let source_url = source_url(id, &absolute)?;
        if !ids.insert(id.to_owned()) {
            return Err(Failure::ConflictingIdentity);
        }
        result.push(json!({"id":id,"name":text(link)?,
            "source":text(one(row.select(&source))?)?,"source_url":source_url}));
    }
    Ok(Value::Array(result))
}
