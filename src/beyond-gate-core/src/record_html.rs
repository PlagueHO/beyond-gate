//! Explicit selected-record HTML with fixed source fields and no arbitrary URL reads.
//!
//! Record resolution and the one selected page share a session identity and
//! deadline. Inline descriptions require no additional request. Ordinary JSON
//! reads and MCP records do not use this module.

pub(crate) mod page;

#[cfg(test)]
mod tests;

use crate::{
    auth::session::{Session, SessionError},
    background::{self, BackgroundError},
    character_campaign::{self, CharacterCampaignError},
    class_catalog::{self, ClassCatalogError, ClassKind},
    feat::{self, FeatError},
    http::{HttpClient, HttpError},
    item_catalog::{self, ItemError, ItemSelection},
    monster::{self, MonsterError},
    species::{self, SpeciesError},
    spell::{self, SpellError},
};
use page::PageKind;
use serde_json::{Value, json};
use std::{error::Error, fmt, time::Duration};

/// Supported record operations; page availability still requires checked source data.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Resource {
    /// One selected character, including supported links in its preserved definitions.
    Character,
    /// One campaign record; no website content contract is currently established.
    Campaign,
    /// One class metadata record.
    Class,
    /// One subclass metadata record with parent context.
    Subclass,
    /// One selected background definition.
    Background,
    /// One selected feat definition.
    Feat,
    /// One selected species definition.
    Species,
    /// One selected monster with fresh access code 1.
    Monster,
    /// One equipment definition on an explicit source page.
    Equipment,
    /// One magic-item definition on an explicit source page.
    MagicItem,
    /// One spell with its existing checked HTML.
    Spell,
}

/// Explicit source selection; no field contains credentials, URLs, or DOM selectors.
pub struct Request {
    /// Selects an existing shared operation.
    pub resource: Resource,
    /// Existing record ID, composite item ID, or spell key.
    pub id: String,
    /// Required one-based source page for equipment and magic items.
    pub page: Option<u32>,
    /// Optional existing subclass parent scope.
    pub class_id: Option<String>,
    /// Fixed normalized source path, required when supported page links are ambiguous.
    pub field: Option<String>,
    /// Allows ordinary record Markdown when no supported HTML source exists.
    pub markdown: bool,
}

/// Separates original record bounds from the independently bounded selected subtree.
pub struct ReadResult {
    /// Original checked record; its existing keys and raw values are unchanged.
    pub record: Value,
    /// Checked content, or no content for ordinary record Markdown.
    pub selected: Option<SelectedHtml>,
}

/// Attribution for exactly one selected HTML source.
pub struct SelectedHtml {
    /// Normalized fixed field path, not an arbitrary JSON pointer.
    pub field: String,
    /// Checked website URL for a fetched page, or none for inline descriptions.
    pub source_url: Option<String>,
    /// Selected untrusted content, never a complete website document.
    pub raw_html: String,
}

impl ReadResult {
    /// Keeps ordinary record Markdown distinct from successful HTML extraction.
    pub fn metadata(record: Value) -> Self {
        Self {
            record,
            selected: None,
        }
    }
}

/// Redacted source-selection failures with original typed operation causes.
pub enum HtmlError {
    /// A source selector or existing resource argument is invalid.
    InvalidInput,
    /// A selected record or page has malformed source metadata.
    InvalidResponse,
    /// No supported source is present; never a successful empty HTML result.
    MissingSource,
    /// Known source fields do not have supported URL or page contracts.
    UnsupportedSource(Vec<String>),
    /// More than one supported page link requires explicit selection.
    AmbiguousSource(Vec<String>),
    /// HTML or source counts exceed unchanged page or bounded DOM limits.
    LimitExceeded,
    /// The complete record and page operation reached its deadline.
    TimedOut,
    /// Original character/campaign read failure.
    Character(CharacterCampaignError),
    /// Original class/subclass read failure.
    Class(ClassCatalogError),
    /// Original background read failure.
    Background(BackgroundError),
    /// Original feat read failure.
    Feat(FeatError),
    /// Original species read failure.
    Species(SpeciesError),
    /// Original monster access or source failure.
    Monster(MonsterError),
    /// Original equipment or magic-item read failure.
    Item(ItemError),
    /// Original spell read failure.
    Spell(SpellError),
    /// Selected page session failure, including denial and rejected redirects.
    Session(SessionError),
    /// Redacted public client setup failure.
    Http(HttpError),
}

impl fmt::Display for HtmlError {
    /// Prints only fixed diagnostics and internally generated source field names.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInput => f.write_str("invalid HTML source selection"),
            Self::InvalidResponse => f.write_str("invalid selected HTML response"),
            Self::MissingSource => f.write_str("no supported HTML source exists in this record"),
            Self::UnsupportedSource(fields) => write!(
                f,
                "HTML source contract is unsupported: {}",
                fields.join(", ")
            ),
            Self::AmbiguousSource(fields) => write!(
                f,
                "multiple HTML sources exist; select --html-field: {}",
                fields.join(", ")
            ),
            Self::LimitExceeded => f.write_str("selected HTML source or DOM limit exceeded"),
            Self::TimedOut => f.write_str("selected HTML operation timed out"),
            Self::Character(error) => error.fmt(f),
            Self::Class(error) => error.fmt(f),
            Self::Background(error) => error.fmt(f),
            Self::Feat(error) => error.fmt(f),
            Self::Species(error) => error.fmt(f),
            Self::Monster(error) => error.fmt(f),
            Self::Item(error) => error.fmt(f),
            Self::Spell(error) => error.fmt(f),
            Self::Http(error) => error.fmt(f),
            Self::Session(error) => {
                write!(f, "selected HTML page read failed: {error}")?;
                if let Some(http) = error
                    .source()
                    .and_then(|cause| cause.downcast_ref::<HttpError>())
                {
                    write!(f, ": {http}")?;
                }
                Ok(())
            }
        }
    }
}

impl fmt::Debug for HtmlError {
    /// Excludes original record values and HTML from test-runner diagnostics.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl Error for HtmlError {
    /// Preserves typed causes without exposing raw source data.
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Character(error) => Some(error),
            Self::Class(error) => Some(error),
            Self::Background(error) => Some(error),
            Self::Feat(error) => Some(error),
            Self::Species(error) => Some(error),
            Self::Monster(error) => Some(error),
            Self::Item(error) => Some(error),
            Self::Spell(error) => Some(error),
            Self::Session(error) => Some(error),
            Self::Http(error) => Some(error),
            _ => None,
        }
    }
}

impl Request {
    /// Rejects invalid selectors and existing resource inputs before session access.
    ///
    /// # Errors
    ///
    /// Returns invalid input or the original typed resource validation failure.
    pub fn validate(&self) -> Result<(), HtmlError> {
        if (self.page.is_some()
            && !matches!(self.resource, Resource::Equipment | Resource::MagicItem))
            || (self.class_id.is_some() && self.resource != Resource::Subclass)
        {
            return Err(HtmlError::InvalidInput);
        }
        if let Some(field) = &self.field {
            validate_field(field)?;
            let valid = match self.resource {
                Resource::Character => character_field(field),
                Resource::Class | Resource::Subclass => field == "more_details_url",
                Resource::Equipment | Resource::MagicItem => field == "raw.description",
                Resource::Spell => field == "source_url",
                Resource::Background | Resource::Feat => matches!(
                    field.as_str(),
                    "source_url" | "raw.more_details_url" | "raw.description"
                ),
                Resource::Monster => {
                    matches!(field.as_str(), "source_url" | "raw.url" | "raw.description")
                }
                Resource::Species => {
                    matches!(field.as_str(), "raw.more_details_url" | "raw.description")
                }
                Resource::Campaign => field == "source_url",
            };
            if !valid {
                return Err(HtmlError::InvalidInput);
            }
        }
        match self.resource {
            Resource::Background => {
                background::validate_id(&self.id).map_err(HtmlError::Background)
            }
            Resource::Feat => feat::validate_id(&self.id).map_err(HtmlError::Feat),
            Resource::Species => species::validate_id(&self.id).map_err(HtmlError::Species),
            Resource::Monster => monster::validate_id(&self.id).map_err(HtmlError::Monster),
            Resource::Spell => spell::validate_slug(&self.id).map_err(HtmlError::Spell),
            Resource::Class | Resource::Subclass => class_catalog::validate_input(
                if self.resource == Resource::Class {
                    ClassKind::Class
                } else {
                    ClassKind::Subclass
                },
                None,
                self.class_id.as_deref(),
                Some(&self.id),
            )
            .map_err(HtmlError::Class),
            Resource::Equipment | Resource::MagicItem => {
                self.item_selection()?.validate().map_err(HtmlError::Item)
            }
            Resource::Character | Resource::Campaign => {
                if self.id.is_empty()
                    || self.id.trim() != self.id
                    || self.id.len() > 256
                    || self.id.chars().any(char::is_control)
                {
                    Err(HtmlError::InvalidInput)
                } else {
                    Ok(())
                }
            }
        }
    }

    /// Retains explicit composite identity and one source page without inventing defaults.
    fn item_selection(&self) -> Result<ItemSelection, HtmlError> {
        Ok(ItemSelection {
            id: self.id.clone(),
            page: self.page.ok_or(HtmlError::InvalidInput)?,
        })
    }
}

/// Checks the finite source-path grammar without evaluating arbitrary JSON pointers.
///
/// # Errors
///
/// Rejects unknown fields, raw upstream casing, and noncanonical or oversized indices.
pub fn validate_field(field: &str) -> Result<(), HtmlError> {
    if matches!(
        field,
        "source_url" | "more_details_url" | "raw.more_details_url" | "raw.url" | "raw.description"
    ) || character_field(field)
    {
        Ok(())
    } else {
        Err(HtmlError::InvalidInput)
    }
}

/// Parses one canonical array index and a fixed remaining field suffix.
fn indexed<'a>(field: &'a str, prefix: &str) -> Option<&'a str> {
    let (index, suffix) = field.strip_prefix(prefix)?.split_once("].")?;
    let number = index.parse::<usize>().ok()?;
    (number < 1_000 && number.to_string() == index).then_some(suffix)
}

/// Allows only observed character definition fields and bounded array positions.
fn character_field(field: &str) -> bool {
    if matches!(
        field,
        "source_url"
            | "raw.readonly_url"
            | "raw.background.definition.more_details_url"
            | "raw.race.more_details_url"
    ) {
        return true;
    }
    if let Some(suffix) = indexed(field, "raw.feats[") {
        return suffix == "definition.more_details_url";
    }
    let Some(suffix) = indexed(field, "raw.classes[") else {
        return false;
    };
    if matches!(
        suffix,
        "definition.more_details_url" | "subclass_definition.more_details_url"
    ) {
        return true;
    }
    [
        "definition.class_features[",
        "subclass_definition.class_features[",
    ]
    .iter()
    .any(|prefix| indexed(suffix, prefix) == Some("more_details_url"))
}

/// Resolves one original record and optionally one checked HTML source.
///
/// # Errors
///
/// Rejects ambiguous selection before the page request and never tries another
/// candidate after a failure. Record and page reads preserve typed source failures,
/// one session identity, existing source limits, and a whole-operation deadline.
pub async fn read(session: &Session, request: &Request) -> Result<ReadResult, HtmlError> {
    request.validate()?;
    let session = session.read_scope();
    let seconds = if matches!(request.resource, Resource::Class | Resource::Subclass) {
        120
    } else {
        60
    };
    tokio::time::timeout(Duration::from_secs(seconds), async {
        let record = resolve(&session, request).await?;
        let sources = sources(&record, request.resource)?;
        let Some(source) = choose(&sources, request.field.as_deref(), request.markdown)? else {
            return Ok(ReadResult::metadata(record));
        };
        let raw_html = if let Some(content) = &source.inline {
            page::validate_fragment(content)?;
            content.clone()
        } else {
            let kind = source
                .kind
                .ok_or_else(|| HtmlError::UnsupportedSource(vec![source.field.clone()]))?;
            let url = source.target.as_ref().ok_or(HtmlError::InvalidResponse)?;
            let (client, origin) = page_client(&session, kind)?;
            let target = format!("{origin}{}", url.path());
            let body = match kind {
                PageKind::Background => session.get_background_detail_html(&client, &target).await,
                PageKind::Feat => session.get_feat_detail_html(&client, &target).await,
                PageKind::Class | PageKind::Subclass => {
                    session.get_class_detail_html(&client, &target).await
                }
            }
            .map_err(HtmlError::Session)?;
            page::selected(&body, kind, url, &source.name)?
        };
        Ok(ReadResult {
            record,
            selected: Some(SelectedHtml {
                field: request
                    .field
                    .clone()
                    .unwrap_or_else(|| source.field.clone()),
                source_url: source.target.as_ref().map(ToString::to_string),
                raw_html,
            }),
        })
    })
    .await
    .map_err(|_| HtmlError::TimedOut)?
}

/// Calls existing shared operations without adding definition or ownership inference.
async fn resolve(session: &Session, request: &Request) -> Result<Value, HtmlError> {
    let id = &request.id;
    match request.resource {
        Resource::Background => background::show(session, id)
            .await
            .map_err(HtmlError::Background),
        Resource::Feat => feat::show(session, id).await.map_err(HtmlError::Feat),
        Resource::Species => species::show(session, id).await.map_err(HtmlError::Species),
        Resource::Monster => monster::show(session, id).await.map_err(HtmlError::Monster),
        Resource::Spell => spell::show(session, id).await.map_err(HtmlError::Spell),
        Resource::Class | Resource::Subclass => class_catalog::show(
            session,
            if request.resource == Resource::Class {
                ClassKind::Class
            } else {
                ClassKind::Subclass
            },
            id,
            request.class_id.as_deref(),
        )
        .await
        .map_err(HtmlError::Class),
        Resource::Equipment => item_catalog::show(session, &request.item_selection()?)
            .await
            .map_err(HtmlError::Item),
        Resource::MagicItem => item_catalog::show_magic_item(session, &request.item_selection()?)
            .await
            .map_err(HtmlError::Item),
        Resource::Character => {
            let value = character_campaign::get_character(session, id)
                .await
                .map_err(HtmlError::Character)?;
            Ok(
                json!({"id":value.id(),"name":value.name(),"source_url":value.source_url().map(|url| url.as_str()),"raw":value.raw()}),
            )
        }
        Resource::Campaign => {
            let value = character_campaign::get_campaign(session, id)
                .await
                .map_err(HtmlError::Character)?;
            Ok(
                json!({"id":value.id(),"name":value.name(),"source_url":value.source_url().map(|url| url.as_str()),"character_ids":value.character_ids(),"raw":value.raw()}),
            )
        }
    }
}

/// One semantic source; normalized aliases do not create duplicate page candidates.
struct Source {
    field: String,
    aliases: Vec<String>,
    kind: Option<PageKind>,
    target: Option<reqwest::Url>,
    inline: Option<String>,
    name: String,
}

/// Registers one existing inline string without selecting an API URL for an HTML request.
fn inline_source(field: &str, value: &Value) -> Result<Option<Source>, HtmlError> {
    if value.is_null() {
        return Ok(None);
    }
    let content = value.as_str().ok_or(HtmlError::InvalidResponse)?;
    if content.trim().is_empty() {
        return Ok(None);
    }
    Ok(Some(Source {
        field: field.to_owned(),
        aliases: vec![],
        kind: None,
        target: None,
        inline: Some(content.to_owned()),
        name: String::new(),
    }))
}

/// Checks a source URL against the selected definition's numeric identity.
fn page_source(
    field: &str,
    kind: Option<PageKind>,
    definition: &Value,
    link: &Value,
    parent: Option<&str>,
) -> Result<Option<Source>, HtmlError> {
    if link.is_null() {
        return Ok(None);
    }
    let text = link.as_str().ok_or(HtmlError::InvalidResponse)?;
    if text.is_empty() {
        return Ok(None);
    }
    if kind.is_some()
        && definition["name"]
            .as_str()
            .is_some_and(|name| name.len() > 1024)
    {
        return Err(HtmlError::LimitExceeded);
    }
    if kind.is_some()
        && (source_id(&definition["id"]).is_none()
            || definition["name"]
                .as_str()
                .is_none_or(|name| name.trim().is_empty() || name.chars().any(char::is_control))
            || (kind == Some(PageKind::Subclass) && parent.is_none()))
    {
        return Err(HtmlError::InvalidResponse);
    }
    let absolute = if text.starts_with('/') {
        format!("https://www.dndbeyond.com{text}")
    } else {
        text.to_owned()
    };
    let target = reqwest::Url::parse(&absolute).ok().filter(|url| {
        url.as_str() == absolute
            && url.origin().ascii_serialization() == "https://www.dndbeyond.com"
            && url.username().is_empty()
            && url.password().is_none()
            && url.query().is_none()
            && !text.contains(['%', '\\'])
            && text.trim() == text
            && !text.chars().any(char::is_control)
    });
    let target = target.filter(|url| {
        let expected = parent
            .map(str::to_owned)
            .or_else(|| source_id(&definition["id"]));
        let prefix = match kind {
            Some(PageKind::Background) => "/backgrounds/",
            Some(PageKind::Feat) => "/feats/",
            Some(PageKind::Class | PageKind::Subclass) => "/classes/",
            None => return false,
        };
        let Some((id, slug)) = url
            .path()
            .strip_prefix(prefix)
            .and_then(|path| path.split_once('-'))
        else {
            return false;
        };
        expected.as_deref() == Some(id)
            && !slug.is_empty()
            && slug
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
            && (matches!(kind, Some(PageKind::Class | PageKind::Subclass))
                || url.fragment().is_none())
            && (kind != Some(PageKind::Subclass) || url.fragment().is_some())
            && (!field.contains(".class_features[") || url.fragment().is_some())
    });
    Ok(Some(Source {
        field: field.to_owned(),
        aliases: vec![],
        kind: kind.filter(|_| target.is_some()),
        target,
        inline: None,
        name: definition["name"].as_str().unwrap_or("").to_owned(),
    }))
}

/// Keeps canonical positive IDs consistent for projected and preserved definitions.
fn source_id(value: &Value) -> Option<String> {
    let text = value
        .as_str()
        .map(str::to_owned)
        .or_else(|| value.as_u64().map(|id| id.to_string()))?;
    text.parse::<u64>()
        .ok()
        .filter(|id| *id > 0 && id.to_string() == text)
        .map(|_| text)
}

/// Enumerates only established semantic fields; unknown raw fields remain untouched.
fn sources(record: &Value, resource: Resource) -> Result<Vec<Source>, HtmlError> {
    let mut result = Vec::new();
    let (field, kind) = match resource {
        Resource::Background => ("source_url", Some(PageKind::Background)),
        Resource::Feat => ("source_url", Some(PageKind::Feat)),
        Resource::Class => ("more_details_url", Some(PageKind::Class)),
        Resource::Subclass => ("more_details_url", Some(PageKind::Subclass)),
        Resource::Monster | Resource::Character | Resource::Campaign => ("source_url", None),
        Resource::Species => ("raw.more_details_url", None),
        Resource::Equipment | Resource::MagicItem | Resource::Spell => ("", None),
    };
    if resource == Resource::Spell {
        if let Some(mut source) = inline_source("source_url", &record["raw_html"])? {
            source.target = record["source_url"]
                .as_str()
                .and_then(|url| reqwest::Url::parse(url).ok());
            result.push(source);
        }
    } else if resource == Resource::Character {
        character_sources(&mut result, record)?;
    } else {
        let link = if resource == Resource::Species {
            &record["raw"]["moreDetailsUrl"]
        } else {
            &record[field]
        };
        let parent = record["parent_class_id"].as_str();
        if !field.is_empty()
            && let Some(mut source) = page_source(field, kind, record, link, parent)?
        {
            source.aliases = match resource {
                Resource::Background | Resource::Feat => vec!["raw.more_details_url".to_owned()],
                Resource::Monster => vec!["raw.url".to_owned()],
                _ => vec![],
            };
            result.push(source);
        }
        if matches!(
            resource,
            Resource::Background
                | Resource::Feat
                | Resource::Species
                | Resource::Monster
                | Resource::Equipment
                | Resource::MagicItem
        ) && let Some(source) = inline_source("raw.description", &record["raw"]["description"])?
        {
            result.push(source);
        }
    }
    if result.len() > 1_000 {
        return Err(HtmlError::LimitExceeded);
    }
    Ok(result)
}

/// Keeps optional source arrays distinct from malformed recognized metadata.
fn source_array(value: &Value) -> Result<Option<&[Value]>, HtmlError> {
    if value.is_null() {
        return Ok(None);
    }
    value
        .as_array()
        .map(|values| Some(values.as_slice()))
        .ok_or(HtmlError::InvalidResponse)
}

/// Registers observed character definition links without scanning arbitrary JSON keys.
fn character_sources(result: &mut Vec<Source>, record: &Value) -> Result<(), HtmlError> {
    let raw = &record["raw"];
    for (field, kind, definition, link) in [
        ("source_url", None, record, &record["source_url"]),
        (
            "raw.background.definition.more_details_url",
            Some(PageKind::Background),
            &raw["background"]["definition"],
            &raw["background"]["definition"]["moreDetailsUrl"],
        ),
        (
            "raw.race.more_details_url",
            None,
            &raw["race"],
            &raw["race"]["moreDetailsUrl"],
        ),
        ("raw.readonly_url", None, raw, &raw["readonlyUrl"]),
    ] {
        if let Some(source) = page_source(field, kind, definition, link, None)? {
            result.push(source);
        }
    }

    if let Some(classes) = source_array(&raw["classes"])? {
        if classes.len() > 1_000 {
            return Err(HtmlError::LimitExceeded);
        }
        for (index, class) in classes.iter().enumerate() {
            let parent = source_id(&class["definition"]["id"]);
            for (key, normalized, kind) in [
                ("definition", "definition", PageKind::Class),
                (
                    "subclassDefinition",
                    "subclass_definition",
                    PageKind::Subclass,
                ),
            ] {
                let definition = &class[key];
                let field = format!("raw.classes[{index}].{normalized}.more_details_url");
                if let Some(source) = page_source(
                    &field,
                    Some(kind),
                    definition,
                    &definition["moreDetailsUrl"],
                    if kind == PageKind::Subclass {
                        parent.as_deref()
                    } else {
                        None
                    },
                )? {
                    result.push(source);
                }
                if let Some(features) = source_array(&definition["classFeatures"])? {
                    if features.len() > 1_000 {
                        return Err(HtmlError::LimitExceeded);
                    }
                    for (feature_index, feature) in features.iter().enumerate() {
                        let field = format!(
                            "raw.classes[{index}].{normalized}.class_features[{feature_index}].more_details_url"
                        );
                        if let Some(source) = page_source(
                            &field,
                            Some(kind),
                            definition,
                            &feature["moreDetailsUrl"],
                            if kind == PageKind::Subclass {
                                parent.as_deref()
                            } else {
                                None
                            },
                        )? {
                            result.push(source);
                        }
                        if result.len() > 1_000 {
                            return Err(HtmlError::LimitExceeded);
                        }
                    }
                    if result.len() > 1_000 {
                        return Err(HtmlError::LimitExceeded);
                    }
                }
            }
        }
    }
    if let Some(feats) = source_array(&raw["feats"])? {
        if feats.len() > 1_000 {
            return Err(HtmlError::LimitExceeded);
        }
        for (index, feat) in feats.iter().enumerate() {
            let field = format!("raw.feats[{index}].definition.more_details_url");
            let definition = &feat["definition"];
            if let Some(source) = page_source(
                &field,
                Some(PageKind::Feat),
                definition,
                &definition["moreDetailsUrl"],
                None,
            )? {
                result.push(source);
            }
            if result.len() > 1_000 {
                return Err(HtmlError::LimitExceeded);
            }
        }
    }
    Ok(())
}

/// Selects a single source before page access; unsupported links are never tried.
fn choose<'a>(
    sources: &'a [Source],
    field: Option<&str>,
    markdown: bool,
) -> Result<Option<&'a Source>, HtmlError> {
    if let Some(field) = field {
        let source = sources
            .iter()
            .find(|source| {
                source.field == field || source.aliases.iter().any(|alias| alias == field)
            })
            .ok_or(HtmlError::MissingSource)?;
        if source.inline.is_none() && source.kind.is_none() {
            return Err(HtmlError::UnsupportedSource(vec![source.field.clone()]));
        }
        return Ok(Some(source));
    }
    let pages: Vec<_> = sources
        .iter()
        .filter(|source| source.kind.is_some())
        .collect();
    if pages.len() > 1 {
        return Err(HtmlError::AmbiguousSource(
            pages.iter().map(|source| source.field.clone()).collect(),
        ));
    }
    if let Some(source) = pages.first() {
        return Ok(Some(source));
    }
    if let Some(source) = sources.iter().find(|source| source.inline.is_some()) {
        return Ok(Some(source));
    }
    if markdown {
        return Ok(None);
    }
    if sources.is_empty() {
        Err(HtmlError::MissingSource)
    } else {
        Err(HtmlError::UnsupportedSource(
            sources.iter().map(|source| source.field.clone()).collect(),
        ))
    }
}

/// Uses fixed origins or the selected resource's compiled owner-scoped fixture.
fn page_client(session: &Session, kind: PageKind) -> Result<(HttpClient, String), HtmlError> {
    session.supported().map_err(HtmlError::Session)?;
    #[cfg(any(test, feature = "session-fixture"))]
    match kind {
        PageKind::Background => {
            if let Some(source) = &session.background_fixture {
                return Ok((source.client.clone(), source.origin.clone()));
            }
        }
        PageKind::Feat => {
            if let Some(source) = &session.feat_fixture {
                return Ok((source.client.clone(), source.origin.clone()));
            }
        }
        PageKind::Class | PageKind::Subclass => {
            if let Some(source) = &session.class_catalog_fixture {
                return Ok((source.client.clone(), source.origin.clone()));
            }
        }
    }
    #[cfg(not(any(test, feature = "session-fixture")))]
    let _ = kind;
    Ok((
        HttpClient::single_attempt(&["https://www.dndbeyond.com"]).map_err(HtmlError::Http)?,
        "https://www.dndbeyond.com".to_owned(),
    ))
}
