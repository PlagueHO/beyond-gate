//! Parses only bounded website candidate IDs, form options, and explicit pagination.

use super::query::NUMBER_FILTERS;
use super::{MAX_PAGE_RECORDS, MonsterError, MonsterQuery, WEBSITE, source_url, validate_id};
use scraper::{ElementRef, Html, Selector};
use std::collections::HashSet;

/// Candidate identities in website order; they are not content-access evidence.
pub(super) struct Page {
    /// Exact selected identities for one collection read.
    pub(super) ids: Vec<String>,
    /// Explicit next page, never fetched automatically.
    pub(super) next: Option<u32>,
}

/// Compiles fixed internal selectors without reflecting page content in errors.
fn selector(value: &str) -> Result<Selector, MonsterError> {
    Selector::parse(value).map_err(|_| MonsterError::InvalidResponse)
}

/// Requires exactly one known element rather than accepting a missing or ambiguous page.
fn one<'a>(
    mut elements: impl Iterator<Item = ElementRef<'a>>,
) -> Result<ElementRef<'a>, MonsterError> {
    let element = elements.next().ok_or(MonsterError::InvalidResponse)?;
    if elements.next().is_some() {
        return Err(MonsterError::InvalidResponse);
    }
    Ok(element)
}

/// Checks requested option IDs against the same response's public filter form.
fn filters(document: &Html, query: &MonsterQuery) -> Result<(), MonsterError> {
    let forms = selector("form")?;
    let form = one(document.select(&forms).filter(|form| {
        matches!(
            form.value().attr("action"),
            Some("/monsters" | "https://www.dndbeyond.com/monsters")
        )
    }))?;
    if !form
        .value()
        .attr("method")
        .is_some_and(|s| s.eq_ignore_ascii_case("get"))
    {
        return Err(MonsterError::InvalidResponse);
    }
    let controls = selector("[name]")?;
    let options = selector("option")?;
    for (key, value) in query.website_url(WEBSITE)?.query_pairs() {
        if key == "page" {
            continue;
        }
        let matching: Vec<_> = form
            .select(&controls)
            .filter(|control| control.value().attr("name") == Some(key.as_ref()))
            .collect();
        if matching.is_empty() {
            return Err(MonsterError::InvalidResponse);
        }
        if matching[0].value().name() == "select" {
            if matching.len() != 1
                || !matching[0]
                    .select(&options)
                    .any(|option| option.value().attr("value") == Some(value.as_ref()))
            {
                return Err(MonsterError::InvalidInput);
            }
        } else if matching.iter().all(|control| {
            control.value().name() == "input" && control.value().attr("type") == Some("checkbox")
        }) {
            if !matching
                .iter()
                .any(|control| control.value().attr("value") == Some(value.as_ref()))
            {
                return Err(MonsterError::InvalidInput);
            }
        } else if matching.len() != 1
            || matching[0].value().name() != "input"
            || (key != "filter-search" && !NUMBER_FILTERS.iter().any(|(_, _, field)| *field == key))
        {
            return Err(MonsterError::InvalidResponse);
        }
    }
    Ok(())
}

/// Extracts one canonical numeric monster link from a listing row.
fn identity(row: ElementRef<'_>) -> Result<String, MonsterError> {
    let links = selector(
        ".monster-name a[href^='/monsters/'], .monster-name a[href^='https://www.dndbeyond.com/monsters/']",
    )?;
    let mut ids = Vec::new();
    for link in row.select(&links) {
        let href = link
            .value()
            .attr("href")
            .ok_or(MonsterError::InvalidResponse)?;
        let absolute = if href.starts_with("/monsters/") {
            format!("{WEBSITE}{href}")
        } else {
            href.to_owned()
        };
        let url = reqwest::Url::parse(&absolute).map_err(|_| MonsterError::InvalidResponse)?;
        let Some(path) = url.path().strip_prefix("/monsters/") else {
            continue;
        };
        let id = path
            .split('-')
            .next()
            .ok_or(MonsterError::InvalidResponse)?;
        validate_id(id).map_err(|_| MonsterError::InvalidResponse)?;
        source_url(id, &absolute)?;
        ids.push(id.to_owned());
    }
    if ids.len() != 1 {
        return Err(MonsterError::InvalidResponse);
    }
    ids.pop().ok_or(MonsterError::InvalidResponse)
}

/// Compares filter semantics while allowing source query ordering and empty-field omission.
fn pairs(url: &reqwest::Url) -> Vec<(String, String)> {
    let mut pairs: Vec<_> = url
        .query_pairs()
        .filter(|(key, value)| key != "page" && !value.is_empty())
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect();
    pairs.sort();
    pairs
}

/// Checks only scoped pagination, including repeated top/bottom controls.
fn pagination(
    document: &Html,
    query: &MonsterQuery,
    empty: bool,
) -> Result<Option<u32>, MonsterError> {
    let active = selector(".b-pagination-list .b-pagination-item.active")?;
    let mut current = None;
    for node in document.select(&active) {
        let text = node.text().collect::<String>();
        let page = text
            .trim()
            .parse::<u32>()
            .map_err(|_| MonsterError::IncompletePage)?;
        if page != query.page || current.is_some_and(|n| n != page) {
            return Err(MonsterError::IncompletePage);
        }
        current = Some(page);
    }
    let next = selector(".b-pagination-list a[rel='next']")?;
    let expected = query.website_url(WEBSITE)?;
    let mut found = None;
    for link in document.select(&next) {
        let href = link
            .value()
            .attr("href")
            .ok_or(MonsterError::IncompletePage)?;
        if href.contains('\\') || href.chars().any(char::is_control) {
            return Err(MonsterError::IncompletePage);
        }
        let url = expected
            .join(href)
            .map_err(|_| MonsterError::IncompletePage)?;
        let pages: Vec<_> = url
            .query_pairs()
            .filter(|(key, _)| key == "page")
            .map(|(_, value)| value.into_owned())
            .collect();
        let value = pages
            .first()
            .and_then(|s| s.parse::<u32>().ok())
            .ok_or(MonsterError::IncompletePage)?;
        if empty
            || current.is_none()
            || url.origin() != expected.origin()
            || url.path() != "/monsters"
            || url.fragment().is_some()
            || !url.username().is_empty()
            || url.password().is_some()
            || pages.len() != 1
            || pages[0] != value.to_string()
            || query.page.checked_add(1) != Some(value)
            || pairs(&url) != pairs(&expected)
            || found.is_some_and(|n| n != value)
        {
            return Err(MonsterError::IncompletePage);
        }
        found = Some(value);
    }
    if !empty && query.page != 1 && current.is_none() {
        return Err(MonsterError::IncompletePage);
    }
    let links = selector(".b-pagination-list a[href]")?;
    let has_later_page = document.select(&links).any(|link| {
        link.value()
            .attr("href")
            .and_then(|href| expected.join(href).ok())
            .is_some_and(|url| {
                url.query_pairs().any(|(key, value)| {
                    key == "page" && value.parse::<u32>().is_ok_and(|page| page > query.page)
                })
            })
    });
    if found.is_none() && has_later_page {
        return Err(MonsterError::IncompletePage);
    }
    Ok(found)
}

/// Parses one complete candidate page; missing markup never becomes an empty success.
pub(super) fn parse(bytes: &[u8], query: &MonsterQuery) -> Result<Page, MonsterError> {
    let text = std::str::from_utf8(bytes).map_err(|_| MonsterError::InvalidResponse)?;
    let document = Html::parse_document(text);
    filters(&document, query)?;
    let listing = selector(".listing-body > ul.listing-rpgmonster.rpgmonster-listing")?;
    let listing = one(document.select(&listing))?;
    let body = listing
        .parent()
        .and_then(ElementRef::wrap)
        .ok_or(MonsterError::InvalidResponse)?;
    for (element, class) in [
        (
            body.prev_siblings().find_map(ElementRef::wrap),
            "listing-header",
        ),
        (
            body.next_siblings().find_map(ElementRef::wrap),
            "listing-footer",
        ),
    ] {
        if !element.is_some_and(|element| {
            element
                .value()
                .attr("class")
                .is_some_and(|classes| classes.split_ascii_whitespace().any(|value| value == class))
        }) {
            return Err(MonsterError::IncompletePage);
        }
    }
    let items: Vec<_> = listing.children().filter_map(ElementRef::wrap).collect();
    if items.len() > MAX_PAGE_RECORDS {
        return Err(MonsterError::LimitExceeded);
    }
    if items.iter().any(|item| item.value().name() != "li") {
        return Err(MonsterError::InvalidResponse);
    }
    let rows = selector(".info")?;
    let mut ids = Vec::new();
    let mut seen = HashSet::new();
    for row in listing.select(&rows) {
        if ids.len() >= MAX_PAGE_RECORDS {
            return Err(MonsterError::LimitExceeded);
        }
        let id = identity(row)?;
        if !seen.insert(id.clone()) {
            return Err(MonsterError::ConflictingIdentity);
        }
        ids.push(id);
    }
    let marker = selector("li.alert.no-results")?;
    let markers: Vec<_> = listing.select(&marker).collect();
    if ids.is_empty() {
        if items.len() != 1
            || markers.len() != 1
            || markers[0].text().collect::<String>().trim()
                != "You failed your Investigation check."
        {
            return Err(MonsterError::InvalidResponse);
        }
    } else if !markers.is_empty() {
        return Err(MonsterError::InvalidResponse);
    }
    if !ids.is_empty()
        && (items.len() != ids.len() || items.iter().any(|item| item.select(&rows).count() != 1))
    {
        return Err(MonsterError::IncompletePage);
    }
    let next = pagination(&document, query, ids.is_empty())?;
    Ok(Page { ids, next })
}
