//! Strict extraction of one website page or one selected spell detail subtree.

use super::{MAX_RECORDS, SpellError as Failure, SpellQuery, WEBSITE, validate_slug};
use scraper::{ElementRef, Html, Selector};
use serde_json::{Value, json};
use std::collections::{BTreeMap, HashSet};

/// Compiles fixed selectors without exposing page content in diagnostics.
fn selector(value: &str) -> Result<Selector, Failure> {
    Selector::parse(value).map_err(|_| Failure::InvalidResponse)
}

/// Requires exactly one known element rather than accepting ambiguous content.
fn one<'a>(mut nodes: impl Iterator<Item = ElementRef<'a>>) -> Result<ElementRef<'a>, Failure> {
    let node = nodes.next().ok_or(Failure::InvalidResponse)?;
    if nodes.next().is_some() {
        return Err(Failure::InvalidResponse);
    }
    Ok(node)
}

/// Normalizes metadata whitespace without returning arbitrary markup.
fn text(node: ElementRef<'_>) -> Result<String, Failure> {
    let value = node
        .text()
        .collect::<Vec<_>>()
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if !crate::public_config::valid_text(&value) {
        return Err(Failure::InvalidResponse);
    }
    Ok(value)
}

/// Rejects known challenge markup without attempting another route.
fn document(bytes: &[u8]) -> Result<Html, Failure> {
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
    Ok(document)
}

/// Groups requested nonpage controls while retaining repeated option identities.
fn filters_in(url: &reqwest::Url) -> BTreeMap<String, Vec<String>> {
    let mut result = BTreeMap::<String, Vec<String>>::new();
    for (key, value) in url.query_pairs().filter(|(k, _)| k != "page") {
        result
            .entry(key.into_owned())
            .or_default()
            .push(value.into_owned());
    }
    for values in result.values_mut() {
        values.sort();
    }
    result
}

/// Includes the implicit enabled first option of a single-row, single-select control.
fn selected_values(control: ElementRef<'_>) -> Result<Vec<String>, Failure> {
    let options = selector("option")?;
    let mut selected: Vec<_> = control
        .select(&options)
        .filter(|option| option.value().attr("selected").is_some())
        .collect();
    if selected.is_empty()
        && control.value().attr("multiple").is_none()
        && matches!(control.value().attr("size"), None | Some("1"))
        && let Some(first) = control.select(&options).find(|option| {
            option.value().attr("disabled").is_none()
                && !option
                    .ancestors()
                    .filter_map(ElementRef::wrap)
                    .take_while(|node| node.value().name() != "select")
                    .any(|node| {
                        node.value().name() == "optgroup" && node.value().attr("disabled").is_some()
                    })
        })
    {
        selected.push(first);
    }
    selected
        .into_iter()
        .map(|option| {
            option
                .value()
                .attr("value")
                .map(str::to_owned)
                .ok_or(Failure::InvalidResponse)
        })
        .collect()
}

/// Checks option membership and echoed selections in the actual returned GET form.
fn filters(document: &Html, query: &SpellQuery) -> Result<(), Failure> {
    let forms = selector("form")?;
    let form = one(document.select(&forms).filter(|f| {
        matches!(
            f.value().attr("action"),
            Some("/spells" | "https://www.dndbeyond.com/spells")
        )
    }))?;
    if !form
        .value()
        .attr("method")
        .is_some_and(|s| s.eq_ignore_ascii_case("get"))
    {
        return Err(Failure::InvalidResponse);
    }
    let requested = filters_in(&query.website_url(WEBSITE)?);
    let controls = selector("[name]")?;
    let options = selector("option")?;
    for (key, values) in &requested {
        let nodes: Vec<_> = form
            .select(&controls)
            .filter(|n| n.value().attr("name") == Some(key))
            .collect();
        if nodes.is_empty() {
            return Err(Failure::InvalidResponse);
        }
        let mut selected = Vec::new();
        if nodes.len() == 1 && nodes[0].value().name() == "select" {
            for value in values {
                if !nodes[0]
                    .select(&options)
                    .any(|o| o.value().attr("value") == Some(value))
                {
                    return Err(Failure::InvalidInput);
                }
            }
            selected = selected_values(nodes[0])?;
        } else if key == "filter-search"
            && nodes.len() == 1
            && nodes[0].value().name() == "input"
            && nodes[0].value().attr("type") == Some("text")
        {
            selected.push(nodes[0].value().attr("value").unwrap_or("").to_owned());
        } else if nodes
            .iter()
            .all(|n| n.value().name() == "input" && n.value().attr("type") == Some("checkbox"))
        {
            for value in values {
                if !nodes.iter().any(|n| n.value().attr("value") == Some(value)) {
                    return Err(Failure::InvalidInput);
                }
            }
            selected = nodes
                .iter()
                .filter(|n| n.value().attr("checked").is_some())
                .filter_map(|n| n.value().attr("value").map(str::to_owned))
                .collect();
        } else {
            return Err(Failure::InvalidResponse);
        }
        selected.sort();
        if &selected != values {
            return Err(Failure::IncompleteResult);
        }
    }
    // Supplied filters must not hide an unexpected selected restriction.
    for node in form.select(&controls) {
        let key = node.value().attr("name").ok_or(Failure::InvalidResponse)?;
        if !key.starts_with("filter-") || requested.contains_key(key) {
            continue;
        }
        let selected = if node.value().name() == "select" {
            selected_values(node)?.iter().any(|value| !value.is_empty())
        } else {
            node.value().attr("checked").is_some()
                || (node.value().attr("type") == Some("text")
                    && node.value().attr("value").is_some_and(|s| !s.is_empty()))
        };
        if selected {
            return Err(Failure::IncompleteResult);
        }
    }
    Ok(())
}

/// Compares URL filters while allowing empty unused form controls.
fn pairs(url: &reqwest::Url) -> Vec<(String, String)> {
    let mut values: Vec<_> = url
        .query_pairs()
        .filter(|(k, v)| k != "page" && !v.is_empty())
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();
    values.sort();
    values
}

/// Validates each pagination group and requires top/bottom controls to agree.
fn pagination(document: &Html, query: &SpellQuery, empty: bool) -> Result<Option<u32>, Failure> {
    let expected = query.website_url(WEBSITE)?;
    let groups = selector(".b-pagination-list")?;
    let active = selector(".b-pagination-item.active")?;
    let links = selector("a[href]")?;
    let mut outcomes = Vec::new();
    for group in document.select(&groups) {
        if empty {
            return Err(Failure::IncompleteResult);
        }
        if text(one(group.select(&active))?)? != query.page.to_string() {
            return Err(Failure::IncompleteResult);
        }
        let mut next = None;
        for link in group.select(&links) {
            let href = link.value().attr("href").ok_or(Failure::IncompleteResult)?;
            if href.contains('\\') || href.chars().any(char::is_control) {
                return Err(Failure::IncompleteResult);
            }
            let url = expected.join(href).map_err(|_| Failure::IncompleteResult)?;
            let pages: Vec<_> = url
                .query_pairs()
                .filter(|(k, _)| k == "page")
                .map(|(_, v)| v.into_owned())
                .collect();
            let page = match pages.as_slice() {
                [] => 1,
                [value] => value
                    .parse::<u32>()
                    .ok()
                    .filter(|n| *n > 0 && value == &n.to_string())
                    .ok_or(Failure::IncompleteResult)?,
                _ => return Err(Failure::IncompleteResult),
            };
            if url.origin() != expected.origin()
                || url.path() != "/spells"
                || !url.username().is_empty()
                || url.password().is_some()
                || url.fragment().is_some()
                || pairs(&url) != pairs(&expected)
            {
                return Err(Failure::IncompleteResult);
            }
            let is_next = link
                .value()
                .attr("rel")
                .is_some_and(|s| s.split_ascii_whitespace().any(|s| s == "next"));
            if is_next && (query.page.checked_add(1) != Some(page) || next.replace(page).is_some())
            {
                return Err(Failure::IncompleteResult);
            }
        }
        if next.is_none()
            && group.select(&links).any(|link| {
                link.value()
                    .attr("href")
                    .and_then(|href| expected.join(href).ok())
                    .is_some_and(|url| {
                        url.query_pairs().any(|(k, v)| {
                            k == "page" && v.parse::<u32>().is_ok_and(|n| n > query.page)
                        })
                    })
            })
        {
            return Err(Failure::IncompleteResult);
        }
        outcomes.push(next);
    }
    if outcomes.windows(2).any(|w| w[0] != w[1])
        || (outcomes.is_empty() && query.page != 1 && !empty)
    {
        return Err(Failure::IncompleteResult);
    }
    Ok(outcomes.first().copied().flatten())
}

/// Checks a displayed URL against the complete selected website identity.
fn matching_url(value: &str, slug: &str) -> bool {
    value == format!("/spells/{slug}") || value == format!("{WEBSITE}/spells/{slug}")
}

/// Reads the observed school icon class, whose element has no display text.
fn school(row: ElementRef<'_>, query: &SpellQuery) -> Result<&'static str, Failure> {
    let node = one(row.select(&selector(".spell-school .school")?))?;
    let schools = [
        (3, "abjuration", "Abjuration"),
        (4, "conjuration", "Conjuration"),
        (5, "divination", "Divination"),
        (6, "enchantment", "Enchantment"),
        (7, "evocation", "Evocation"),
        (8, "illusion", "Illusion"),
        (9, "necromancy", "Necromancy"),
        (10, "transmutation", "Transmutation"),
    ];
    let matches: Vec<_> = schools
        .iter()
        .filter(|(_, class, _)| {
            node.value()
                .has_class(class, scraper::CaseSensitivity::CaseSensitive)
        })
        .collect();
    let [(id, _, name)] = matches.as_slice() else {
        return Err(Failure::InvalidResponse);
    };
    if !query.school_ids.is_empty() && !query.school_ids.contains(id) {
        return Err(Failure::IncompleteResult);
    }
    Ok(name)
}

/// Extracts every direct row before returning one complete explicit page.
pub(super) fn list(bytes: &[u8], query: &SpellQuery) -> Result<Value, Failure> {
    let document = document(bytes)?;
    if document
        .select(&selector("ul.listing.listing-rpgspell.rpgspell-listing")?)
        .next()
        .is_none()
    {
        return Err(Failure::UnsupportedOperation);
    }
    filters(&document, query)?;
    let listing = one(document.select(&selector("ul.listing.listing-rpgspell.rpgspell-listing")?))?;
    one(document.select(&selector(".listing-footer")?))?;
    let rows: Vec<_> = listing.children().filter_map(ElementRef::wrap).collect();
    if rows.len() > MAX_RECORDS {
        return Err(Failure::LimitExceeded);
    }
    if rows.is_empty() {
        return Err(Failure::IncompleteResult);
    }
    let empty = rows.len() == 1
        && rows[0].value().name() == "li"
        && rows[0]
            .value()
            .has_class("alert", scraper::CaseSensitivity::CaseSensitive)
        && rows[0]
            .value()
            .has_class("no-results", scraper::CaseSensitivity::CaseSensitive);
    let mut entries = Vec::new();
    let mut ids = HashSet::new();
    if empty && rows[0].select(&selector("a, .info")?).next().is_some() {
        return Err(Failure::InvalidResponse);
    }
    if !empty {
        for row in rows {
            if row.value().name() != "div"
                || !row
                    .value()
                    .has_class("info", scraper::CaseSensitivity::CaseSensitive)
                || row.value().attr("data-type") != Some("spells")
            {
                return Err(Failure::InvalidResponse);
            }
            let slug = row
                .value()
                .attr("data-slug")
                .ok_or(Failure::InvalidResponse)?;
            validate_slug(slug).map_err(|_| Failure::InvalidResponse)?;
            let (id, _) = slug.split_once('-').ok_or(Failure::InvalidResponse)?;
            if !ids.insert(id.to_owned()) {
                return Err(Failure::ConflictingIdentity);
            }
            let link = one(row.select(&selector(".spell-name a.link[href]")?))?;
            if !link
                .value()
                .attr("href")
                .is_some_and(|s| matching_url(s, slug))
            {
                return Err(Failure::InvalidResponse);
            }
            let level_text = text(one(row.select(&selector(".spell-level")?))?)?;
            let level = if level_text.eq_ignore_ascii_case("cantrip") {
                0
            } else {
                let numeral = match level_text.as_str() {
                    "1st" => "1",
                    "2nd" => "2",
                    "3rd" => "3",
                    "4th" => "4",
                    "5th" => "5",
                    "6th" => "6",
                    "7th" => "7",
                    "8th" => "8",
                    "9th" => "9",
                    value => value,
                };
                numeral
                    .parse::<u32>()
                    .ok()
                    .filter(|n| *n <= 9)
                    .ok_or(Failure::InvalidResponse)?
            };
            if !query.levels.is_empty() && !query.levels.contains(&level) {
                return Err(Failure::IncompleteResult);
            }
            entries.push(json!({"id":id,"slug":slug,"name":text(link)?,"level":level,
                "school":school(row, query)?,
                "source_url":format!("{WEBSITE}/spells/{slug}")}));
        }
    }
    Ok(
        json!({"entries":entries,"pagination":{"page":query.page,"next_page":pagination(&document,query,empty)?}}),
    )
}

/// Preserves unknown selected detail elements without trusting or executing HTML.
pub(super) fn show(bytes: &[u8], slug: &str) -> Result<Value, Failure> {
    let document = document(bytes)?;
    let containers = selector(".spell-details")?;
    let Some(container) = document.select(&containers).next() else {
        return Err(Failure::UnsupportedOperation);
    };
    let canonical = one(document.select(&selector("link[rel='canonical']")?))?;
    if canonical.value().attr("href") != Some(format!("{WEBSITE}/spells/{slug}").as_str()) {
        return Err(Failure::InvalidResponse);
    }
    let identities: Vec<_> = container
        .value()
        .classes()
        .filter(|c| c.ends_with("-details") && *c != "spell-details")
        .collect();
    if document.select(&containers).count() != 1 || identities != [format!("{slug}-details")] {
        return Err(Failure::InvalidResponse);
    }
    one(container.select(&selector("section.more-info")?))?;
    let content = one(container.select(&selector("section.detail-content")?))?;
    for marker in [
        ".ddb-statblock.ddb-statblock-spell",
        ".more-info-content",
        ".available-for",
        ".spell-source",
    ] {
        one(content.select(&selector(marker)?))?;
    }
    for marker in [".more-info-content", ".available-for", ".spell-source"] {
        text(one(content.select(&selector(marker)?))?)?;
    }
    for field in [
        "level",
        "casting-time",
        "range-area",
        "components",
        "duration",
        "school",
        "attack-save",
        "damage-effect",
    ] {
        let item = one(content.select(&selector(&format!(".ddb-statblock-item-{field}"))?))?;
        text(one(item.select(&selector(".ddb-statblock-item-label")?))?)?;
        text(one(item.select(&selector(".ddb-statblock-item-value")?))?)?;
    }
    if content.select(&selector("script, form, iframe, object, embed, nav, [id*='comment'], [class*='comment'], [id*='account'], [class*='account'], [class*='purchase'], [class*='unlock'], [class*='login']")?).next().is_some() {
        return Err(Failure::UnsupportedOperation);
    }
    let name = text(one(document.select(&selector("h1.page-title")?))?)?;
    let (id, _) = slug.split_once('-').ok_or(Failure::InvalidInput)?;
    Ok(
        json!({"id":id,"slug":slug,"name":name,"source_url":format!("{WEBSITE}/spells/{slug}"),"raw_html":content.html()}),
    )
}
