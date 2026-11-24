//! Explicitly selected live entity checks. Ordinary tests use synthetic input only.
//!
//! List and detail operations use the production readers and their unchanged limits.
//! No raw record, selected identifier, credential, or content enters the output.

use beyond_gate_core::{
    auth::session::{Session, SessionError, SessionErrorKind},
    background::{self, BackgroundQuery},
    book, character_campaign,
    class_catalog::{self, ClassKind},
    feat::{self, FeatQuery},
    http::{ErrorKind, HttpError},
    item_catalog::{self, ItemQuery, ItemSelection, MagicItemQuery},
    library,
    monster::{self, MonsterQuery},
    species,
    spell::{self, SpellQuery},
};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::value::RawValue;
use std::{error::Error, time::Duration};

/// Enumerates only implemented entity families, never supplied request destinations.
#[derive(Clone, Copy)]
enum Entity {
    Book,
    Character,
    Campaign,
    Class,
    Subclass,
    Monster,
    Background,
    Feat,
    Species,
    Spell,
    Equipment,
    MagicItem,
}

/// Defines the complete fixed order and per-family worst-case HTTP attempts.
const ENTITIES: [(Entity, &str, usize); 12] = [
    (Entity::Book, "book", 11),
    (Entity::Character, "character", 5),
    (Entity::Campaign, "campaign", 6),
    (Entity::Class, "class", 6),
    (Entity::Subclass, "subclass", 10),
    (Entity::Monster, "monster", 6),
    (Entity::Background, "background", 5),
    (Entity::Feat, "feat", 5),
    (Entity::Species, "species", 4),
    (Entity::Spell, "spell", 4),
    (Entity::Equipment, "equipment", 6),
    (Entity::MagicItem, "magic_item", 6),
];

/// Contains only a selected identity and the production parent/page/query context.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Selection {
    id: String,
    #[serde(default, deserialize_with = "present_string")]
    parent_id: Option<String>,
    #[serde(default, deserialize_with = "present_page")]
    page: Option<u32>,
    #[serde(default, deserialize_with = "present_query")]
    query: Option<Box<RawValue>>,
}

/// Rejects explicit null rather than silently discarding a supplied parent.
fn present_string<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
    String::deserialize(d).map(Some)
}

/// Rejects explicit null or nonnumeric source-page context.
fn present_page<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<u32>, D::Error> {
    u32::deserialize(d).map(Some)
}

/// Requires an actual query object when its field is present.
fn present_query<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> Result<Option<Box<RawValue>>, D::Error> {
    let value = Box::<RawValue>::deserialize(d)?;
    if !value.get().trim_start().starts_with('{') {
        return Err(serde::de::Error::custom("query must be an object"));
    }
    Ok(Some(value))
}

/// Rejects unknown and duplicate family fields; omitted families cannot satisfy a full run.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Selections {
    book: Option<Selection>,
    character: Option<Selection>,
    campaign: Option<Selection>,
    class: Option<Selection>,
    subclass: Option<Selection>,
    monster: Option<Selection>,
    background: Option<Selection>,
    feat: Option<Selection>,
    species: Option<Selection>,
    spell: Option<Selection>,
    equipment: Option<Selection>,
    magic_item: Option<Selection>,
}

impl Selections {
    /// Selects a typed family without accepting a URL or arbitrary operation name.
    fn get(&self, entity: Entity) -> Result<&Selection, &'static str> {
        let value = match entity {
            Entity::Book => &self.book,
            Entity::Character => &self.character,
            Entity::Campaign => &self.campaign,
            Entity::Class => &self.class,
            Entity::Subclass => &self.subclass,
            Entity::Monster => &self.monster,
            Entity::Background => &self.background,
            Entity::Feat => &self.feat,
            Entity::Species => &self.species,
            Entity::Spell => &self.spell,
            Entity::Equipment => &self.equipment,
            Entity::MagicItem => &self.magic_item,
        };
        value.as_ref().ok_or("selection_invalid")
    }
}

/// Parses only the existing bounded query model; absence uses its documented defaults.
fn query<Q: DeserializeOwned + Default>(selection: &Selection) -> Result<Q, &'static str> {
    match &selection.query {
        Some(value) => serde_json::from_str(value.get()).map_err(|_| "selection_invalid"),
        None => Ok(Q::default()),
    }
}

/// Requires a canonical positive numeric ID without exposing the supplied value.
fn numeric_id(id: &str) -> Result<(), &'static str> {
    if id
        .parse::<u64>()
        .ok()
        .filter(|n| *n > 0 && n.to_string() == id)
        .is_some()
    {
        Ok(())
    } else {
        Err("selection_invalid")
    }
}

/// Uses production validators before credential input or network work.
fn validate_selection(entity: Entity, s: &Selection) -> Result<(), &'static str> {
    if !matches!(entity, Entity::Subclass) && s.parent_id.is_some()
        || !matches!(entity, Entity::Equipment | Entity::MagicItem) && s.page.is_some()
        || !matches!(
            entity,
            Entity::Monster | Entity::Background | Entity::Feat | Entity::Spell
        ) && s.query.is_some()
    {
        return Err("selection_invalid");
    }
    match entity {
        Entity::Book => book::validate_book_id(&s.id).map_err(|_| "selection_invalid"),
        Entity::Character | Entity::Campaign => numeric_id(&s.id),
        Entity::Class => class_catalog::validate_input(ClassKind::Class, None, None, Some(&s.id))
            .map_err(|_| "selection_invalid"),
        Entity::Subclass => {
            let parent = s.parent_id.as_deref().ok_or("selection_invalid")?;
            class_catalog::validate_input(ClassKind::Subclass, None, Some(parent), Some(&s.id))
                .map_err(|_| "selection_invalid")
        }
        Entity::Monster => {
            monster::validate_id(&s.id).map_err(|_| "selection_invalid")?;
            query::<MonsterQuery>(s)?
                .validate()
                .map_err(|_| "selection_invalid")
        }
        Entity::Background => {
            background::validate_id(&s.id).map_err(|_| "selection_invalid")?;
            query::<BackgroundQuery>(s)?
                .validate()
                .map_err(|_| "selection_invalid")
        }
        Entity::Feat => {
            feat::validate_id(&s.id).map_err(|_| "selection_invalid")?;
            query::<FeatQuery>(s)?
                .validate()
                .map_err(|_| "selection_invalid")
        }
        Entity::Species => species::validate_id(&s.id).map_err(|_| "selection_invalid"),
        Entity::Spell => {
            spell::validate_slug(&s.id).map_err(|_| "selection_invalid")?;
            query::<SpellQuery>(s)?
                .validate()
                .map_err(|_| "selection_invalid")
        }
        Entity::Equipment | Entity::MagicItem => ItemSelection {
            id: s.id.clone(),
            page: s.page.ok_or("selection_invalid")?,
        }
        .validate()
        .map_err(|_| "selection_invalid"),
    }
}

/// Selects a fixed family subset without adding new entity names.
fn selected_scope(value: &str) -> Result<Scope, &'static str> {
    let selected: Vec<_> = ENTITIES
        .iter()
        .copied()
        .filter(|(_, name, _)| value == "all" || value == *name)
        .collect();
    if selected.is_empty() {
        Err("selection_invalid")
    } else {
        Ok(selected)
    }
}

/// Holds a fixed ordered subset and its reviewed attempt limits.
type Scope = Vec<(Entity, &'static str, usize)>;

/// Validates bounded JSON and required context without credentials or HTTP.
fn parse_selections(text: &str, family: &str) -> Result<(Selections, Scope), &'static str> {
    if text.len() > 16 * 1024 {
        return Err("selection_invalid");
    }
    let selections: Selections = serde_json::from_str(text).map_err(|_| "selection_invalid")?;
    let scope = selected_scope(family)?;
    for (entity, _, _) in &scope {
        validate_selection(*entity, selections.get(*entity)?)?;
    }
    Ok((selections, scope))
}

/// Loads only explicitly supplied non-secret selection data.
fn load() -> Result<(Selections, Scope), &'static str> {
    parse_selections(
        &std::env::var("BEYOND_GATE_LIVE_ENTITY_SELECTIONS").map_err(|_| "selection_invalid")?,
        &std::env::var("BEYOND_GATE_LIVE_ENTITY").map_err(|_| "selection_invalid")?,
    )
}

/// Follows only bounded typed cause chains and publishes fixed classifications.
fn failure(error: &(dyn Error + 'static)) -> &'static str {
    let mut cause = Some(error);
    let mut fallback = "read_failed";
    for _ in 0..4 {
        let Some(error) = cause else { break };
        if matches!(
            error.downcast_ref::<book::BookError>(),
            Some(book::BookError::AccessDenied)
        ) || matches!(
            error.downcast_ref::<monster::MonsterError>(),
            Some(monster::MonsterError::AccessDenied)
        ) {
            return "access_denied";
        }
        if matches!(
            error.downcast_ref::<monster::MonsterError>(),
            Some(monster::MonsterError::UnsupportedAccess)
        ) {
            return "unsupported_contract";
        }
        if matches!(
            error.downcast_ref::<book::BookError>(),
            Some(book::BookError::TimedOut)
        ) || matches!(
            error.downcast_ref::<library::LibraryError>(),
            Some(library::LibraryError::TimedOut)
        ) || matches!(
            error.downcast_ref::<character_campaign::CharacterCampaignError>(),
            Some(character_campaign::CharacterCampaignError::TimedOut)
        ) || matches!(
            error.downcast_ref::<class_catalog::ClassCatalogError>(),
            Some(class_catalog::ClassCatalogError::TimedOut)
        ) || matches!(
            error.downcast_ref::<monster::MonsterError>(),
            Some(monster::MonsterError::TimedOut)
        ) || matches!(
            error.downcast_ref::<background::BackgroundError>(),
            Some(background::BackgroundError::TimedOut)
        ) || matches!(
            error.downcast_ref::<feat::FeatError>(),
            Some(feat::FeatError::TimedOut)
        ) || matches!(
            error.downcast_ref::<species::SpeciesError>(),
            Some(species::SpeciesError::TimedOut)
        ) || matches!(
            error.downcast_ref::<spell::SpellError>(),
            Some(spell::SpellError::TimedOut)
        ) || matches!(
            error.downcast_ref::<item_catalog::ItemError>(),
            Some(item_catalog::ItemError::TimedOut)
        ) {
            return "timed_out";
        }
        if let Some(session) = error.downcast_ref::<SessionError>() {
            if session.kind() != SessionErrorKind::ReadFailed {
                return session.kind().as_str();
            }
            fallback = session.kind().as_str();
        }
        if let Some(http) = error.downcast_ref::<HttpError>() {
            return match http.kind() {
                ErrorKind::Timeout => "timed_out",
                ErrorKind::RateLimited => "rate_limited",
                ErrorKind::AccessDenied => "access_denied",
                ErrorKind::Redirect => "redirect_rejected",
                ErrorKind::BodyTooLarge => "body_limit",
                _ => fallback,
            };
        }
        cause = error.source();
    }
    fallback
}

/// Calls one production reader and discards its raw result without serialization.
async fn operation(
    session: &Session,
    entity: Entity,
    detail: bool,
    s: &Selection,
) -> Result<(), &'static str> {
    match (entity, detail) {
        (Entity::Book, false) => library::discover_library(session)
            .await
            .map(|_| ())
            .map_err(|e| failure(&e)),
        (Entity::Book, true) => book::get_book_outline(session, &s.id)
            .await
            .map(|_| ())
            .map_err(|e| failure(&e)),
        (Entity::Character, false) => character_campaign::list_characters(session)
            .await
            .map(|_| ())
            .map_err(|e| failure(&e)),
        (Entity::Character, true) => character_campaign::get_character(session, &s.id)
            .await
            .map(|_| ())
            .map_err(|e| failure(&e)),
        (Entity::Campaign, false) => character_campaign::list_campaigns(session, false)
            .await
            .map(|_| ())
            .map_err(|e| failure(&e)),
        (Entity::Campaign, true) => character_campaign::get_campaign(session, &s.id)
            .await
            .map(|_| ())
            .map_err(|e| failure(&e)),
        (Entity::Class, false) => class_catalog::list(session, ClassKind::Class, None, None)
            .await
            .map(|_| ())
            .map_err(|e| failure(&e)),
        (Entity::Class, true) => class_catalog::show(session, ClassKind::Class, &s.id, None)
            .await
            .map(|_| ())
            .map_err(|e| failure(&e)),
        (Entity::Subclass, false) => {
            class_catalog::list(session, ClassKind::Subclass, None, s.parent_id.as_deref())
                .await
                .map(|_| ())
                .map_err(|e| failure(&e))
        }
        (Entity::Subclass, true) => {
            class_catalog::show(session, ClassKind::Subclass, &s.id, s.parent_id.as_deref())
                .await
                .map(|_| ())
                .map_err(|e| failure(&e))
        }
        (Entity::Monster, false) => monster::list(session, &query(s)?)
            .await
            .map(|_| ())
            .map_err(|e| failure(&e)),
        (Entity::Monster, true) => monster::show(session, &s.id)
            .await
            .map(|_| ())
            .map_err(|e| failure(&e)),
        (Entity::Background, false) => background::list(session, &query(s)?)
            .await
            .map(|_| ())
            .map_err(|e| failure(&e)),
        (Entity::Background, true) => background::show(session, &s.id)
            .await
            .map(|_| ())
            .map_err(|e| failure(&e)),
        (Entity::Feat, false) => feat::list(session, &query(s)?)
            .await
            .map(|_| ())
            .map_err(|e| failure(&e)),
        (Entity::Feat, true) => feat::show(session, &s.id)
            .await
            .map(|_| ())
            .map_err(|e| failure(&e)),
        (Entity::Species, false) => species::list(session)
            .await
            .map(|_| ())
            .map_err(|e| failure(&e)),
        (Entity::Species, true) => species::show(session, &s.id)
            .await
            .map(|_| ())
            .map_err(|e| failure(&e)),
        (Entity::Spell, false) => spell::list(session, &query(s)?)
            .await
            .map(|_| ())
            .map_err(|e| failure(&e)),
        (Entity::Spell, true) => spell::show(session, &s.id)
            .await
            .map(|_| ())
            .map_err(|e| failure(&e)),
        (Entity::Equipment, false) => item_catalog::list(
            session,
            &ItemQuery {
                page: s.page.ok_or("selection_invalid")?,
            },
        )
        .await
        .map(|_| ())
        .map_err(|e| failure(&e)),
        (Entity::Equipment, true) => item_catalog::show(
            session,
            &ItemSelection {
                id: s.id.clone(),
                page: s.page.ok_or("selection_invalid")?,
            },
        )
        .await
        .map(|_| ())
        .map_err(|e| failure(&e)),
        (Entity::MagicItem, false) => item_catalog::list_magic_items(
            session,
            &MagicItemQuery {
                page: s.page.ok_or("selection_invalid")?,
                ..MagicItemQuery::default()
            },
        )
        .await
        .map(|_| ())
        .map_err(|e| failure(&e)),
        (Entity::MagicItem, true) => item_catalog::show_magic_item(
            session,
            &ItemSelection {
                id: s.id.clone(),
                page: s.page.ok_or("selection_invalid")?,
            },
        )
        .await
        .map(|_| ())
        .map_err(|e| failure(&e)),
    }
}

/// Carries only a fixed case identity, outcome, and redacted reason.
struct Row {
    name: &'static str,
    detail: bool,
    status: &'static str,
    reason: &'static str,
}

/// Runs each selected operation once and stops immediately on its first error.
async fn run_scope<F, Fut>(
    scope: &Scope,
    mut read: F,
    rows: &mut Vec<Row>,
) -> Result<(), &'static str>
where
    F: FnMut(Entity, bool) -> Fut,
    Fut: std::future::Future<Output = Result<(), &'static str>>,
{
    for (entity, name, _) in scope {
        for detail in [false, true] {
            let result = read(*entity, detail).await;
            rows.push(Row {
                name,
                detail,
                status: if result.is_ok() { "passed" } else { "failed" },
                reason: result.as_ref().map_or_else(|e| *e, |_| "verified"),
            });
            result?;
        }
    }
    Ok(())
}

/// Marks interrupted work failed and all later unattempted operations blocked.
fn complete_rows(scope: &Scope, rows: &mut Vec<Row>, outcome: Result<(), &'static str>) {
    let completed = rows.len();
    let interrupted_read =
        outcome == Err("timed_out") && rows.last().is_none_or(|row| row.status != "failed");
    for (entity_index, (_, name, _)) in scope.iter().enumerate() {
        for (operation_index, detail) in [false, true].into_iter().enumerate() {
            let index = entity_index * 2 + operation_index;
            if index < completed {
                continue;
            }
            let interrupted = index == completed && interrupted_read;
            rows.push(Row {
                name,
                detail,
                status: if interrupted { "failed" } else { "blocked" },
                reason: if interrupted {
                    "timed_out"
                } else {
                    "stopped_after_failure"
                },
            });
        }
    }
}

/// Formats only allowlisted operation names and classifications.
fn emit(row: &Row) {
    println!(
        "beyond_gate_live_entity={}.{}:{}:{}",
        row.name,
        if row.detail { "detail" } else { "list" },
        row.status,
        row.reason
    );
}

/// Validates selections without credentials or HTTP, even when explicitly selected.
#[test]
#[ignore = "launcher selection preflight only; no network or credentials"]
fn validate_entity_selections() -> Result<(), &'static str> {
    load()?;
    println!("beyond_gate_live_entities_selection=valid");
    Ok(())
}

/// Checks the explicitly approved scope once, stopping after the first failure.
#[tokio::test]
#[ignore = "requires explicit entity live approval, selections, and process-only input"]
async fn public_and_authenticated_entities() -> Result<(), &'static str> {
    if std::env::var("BEYOND_GATE_LIVE_API_APPROVED")
        .ok()
        .as_deref()
        != Some("selected-entity-readers-74-attempts")
    {
        return Err("approval_required");
    }
    let (selections, scope) = load()?;
    let input = std::env::var_os("BEYOND_GATE_COBALT_SESSION").ok_or("input_missing")?;
    let session = Session::default().with_environment_value(Some(input));
    session.status().await.map_err(|e| failure(&e))?;
    println!(
        "beyond_gate_live_entities_budget={}",
        scope.iter().map(|(_, _, budget)| budget).sum::<usize>()
    );
    let mut rows = Vec::new();
    let session_ref = &session;
    let selections_ref = &selections;
    let result = tokio::time::timeout(
        Duration::from_secs(600),
        run_scope(
            &scope,
            |entity, detail| async move {
                operation(session_ref, entity, detail, selections_ref.get(entity)?).await
            },
            &mut rows,
        ),
    )
    .await;
    let outcome = result.unwrap_or(Err("timed_out"));
    complete_rows(&scope, &mut rows, outcome);
    for row in &rows {
        emit(row);
    }
    outcome
}

/// Checks required context and rejects endpoint overrides with invented identifiers.
#[test]
fn selection_contexts_are_strict() {
    let missing_parent: Selection = serde_json::from_str(r#"{"id":"1"}"#).expect("synthetic JSON");
    assert!(validate_selection(Entity::Subclass, &missing_parent).is_err());
    assert!(validate_selection(Entity::Equipment, &missing_parent).is_err());
    assert!(
        serde_json::from_str::<Selection>(r#"{"id":"1","url":"https://example.invalid"}"#).is_err()
    );
    assert!(serde_json::from_str::<Selection>(r#"{"id":"1","parent_id":null}"#).is_err());
    assert!(serde_json::from_str::<Selection>(r#"{"id":"1","query":null}"#).is_err());
    assert!(serde_json::from_str::<Selection>(r#"{"id":"1","id":"2"}"#).is_err());
}

/// Rejects noncanonical IDs and unbounded query/page selectors before requests.
#[test]
fn invalid_selections_fail_locally() {
    let item: Selection = serde_json::from_str(r#"{"id":"1:2","page":0}"#).expect("synthetic JSON");
    assert!(validate_selection(Entity::Equipment, &item).is_err());
    let monster: Selection =
        serde_json::from_str(r#"{"id":"1","query":{"page":0}}"#).expect("synthetic JSON");
    assert!(validate_selection(Entity::Monster, &monster).is_err());
    let spell: Selection = serde_json::from_str(r#"{"id":"https://example.invalid/private"}"#)
        .expect("synthetic JSON");
    assert!(validate_selection(Entity::Spell, &spell).is_err());
    assert!(selected_scope("unknown").is_err());
    assert!(numeric_id("01").is_err());
}

/// Keeps explicit denial and operation deadlines distinct from generic read failures.
#[test]
fn entity_failure_reasons_keep_denials_and_deadlines() {
    assert_eq!(
        failure(&monster::MonsterError::AccessDenied),
        "access_denied"
    );
    assert_eq!(failure(&book::BookError::AccessDenied), "access_denied");
    assert_eq!(
        failure(&monster::MonsterError::UnsupportedAccess),
        "unsupported_contract"
    );
    assert_eq!(failure(&monster::MonsterError::TimedOut), "timed_out");
    assert_eq!(failure(&library::LibraryError::TimedOut), "timed_out");
    assert_eq!(failure(&item_catalog::ItemError::TimedOut), "timed_out");
}

/// Checks the exact UTF-8 boundary, missing full scope, and duplicate query fields.
#[test]
fn bounded_selections_preserve_query_validation() {
    let text = r#"{"character":{"id":"1"}}"#;
    let padded = format!("{text}{}", " ".repeat(16 * 1024 - text.len()));
    assert!(parse_selections(&padded, "character").is_ok());
    assert!(parse_selections(&(padded + " "), "character").is_err());
    assert!(parse_selections(text, "all").is_err());
    assert!(
        parse_selections(
            r#"{"monster":{"id":"1","query":{"page":1,"page":2}}}"#,
            "monster"
        )
        .is_err()
    );
    assert!(
        parse_selections(
            r#"{"monster":{"id":"1","query":{"url":"https://example.invalid"}}}"#,
            "monster"
        )
        .is_err()
    );
}

/// Checks that failure prevents further reads and never marks unattempted work passed.
#[tokio::test]
async fn failed_operation_stops_the_sequence() {
    let scope = selected_scope("all").expect("fixed scope");
    for reason in ["access_denied", "timed_out"] {
        let calls = std::cell::Cell::new(0);
        let mut rows = Vec::new();
        let outcome = run_scope(
            &scope,
            |_, _| {
                calls.set(calls.get() + 1);
                std::future::ready(if calls.get() == 3 {
                    Err(reason)
                } else {
                    Ok(())
                })
            },
            &mut rows,
        )
        .await;
        assert_eq!(calls.get(), 3);
        complete_rows(&scope, &mut rows, outcome);
        assert_eq!(rows.len(), 24);
        assert!(rows[..2].iter().all(|r| r.status == "passed"));
        assert_eq!(rows[2].reason, reason);
        assert!(rows[3..].iter().all(|r| r.status == "blocked"));
    }
}

/// Checks that a successful full sequence executes exactly one list and detail per family.
#[tokio::test]
async fn full_success_executes_each_operation_once() {
    let scope = selected_scope("all").expect("fixed scope");
    let mut calls = Vec::new();
    let mut rows = Vec::new();
    let outcome = run_scope(
        &scope,
        |entity, detail| {
            calls.push((entity, detail));
            std::future::ready(Ok(()))
        },
        &mut rows,
    )
    .await;
    assert_eq!(outcome, Ok(()));
    complete_rows(&scope, &mut rows, outcome);
    assert_eq!(calls.len(), 24);
    assert_eq!(rows.len(), 24);
    for pair in calls.as_chunks::<2>().0 {
        assert!(!pair[0].1);
        assert!(pair[1].1);
    }
    assert!(
        rows.iter()
            .all(|row| row.status == "passed" && row.reason == "verified")
    );
}

/// Checks cancellation marks only the interrupted operation failed.
#[tokio::test]
async fn deadline_blocks_remaining_operations() {
    let scope = selected_scope("character").expect("fixed scope");
    let mut rows = Vec::new();
    let outcome = tokio::time::timeout(
        Duration::from_millis(1),
        run_scope(&scope, |_, _| std::future::pending(), &mut rows),
    )
    .await
    .unwrap_or(Err("timed_out"));
    complete_rows(&scope, &mut rows, outcome);
    assert_eq!(rows[0].status, "failed");
    assert_eq!(rows[0].reason, "timed_out");
    assert_eq!(rows[1].status, "blocked");
}
