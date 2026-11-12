//! Owner-scoped loopback spell responses with invented metadata and text only.

use super::{
    SpellQuery,
    query::{BOOL_FILTERS, ID_FILTERS},
};
use crate::{
    auth::session::{Session, fixture as auth},
    http::HttpClient,
};
use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Duration,
};

/// Fixed offline scenarios; product invocations have no scenario selector.
#[derive(Clone, Copy)]
pub enum Scenario {
    /// Two valid invented rows and a selected detail with unknown elements.
    Normal,
    /// An explicitly marked empty page.
    Empty,
    /// One explicit next-page link with retained filters.
    Next,
    /// A next-page anchor lacks its required destination.
    MissingNextHref,
    /// HTTP 403 with no content.
    Denied,
    /// HTTP 401, not an expiry claim.
    Rejected,
    /// Session validation establishes expiry before reading HTML.
    Expired,
    /// HTTP 404 on selected detail.
    Missing,
    /// A page without the required structure.
    Malformed,
    /// Missing listing footer.
    Incomplete,
    /// Repeated source identity.
    Duplicate,
    /// Raw row count beyond local policy.
    TooManyRecords,
    /// Response bytes beyond the HTTP cap.
    Oversized,
    /// An unknown access structure on HTTP 200.
    Unsupported,
    /// Detail container identity differs from the request.
    WrongIdentity,
    /// Canonical URL differs from the request.
    WrongCanonical,
    /// Missing required detail markers.
    MissingMarkers,
    /// A form appears inside the selected subtree.
    UnsafeDetail,
    /// Unknown selected HTML pushes escaped JSON beyond the result cap.
    LargeResult,
    /// Leaves the resource read waiting for deadline checks.
    Waiting,
    /// A redirect must not be followed.
    Redirect,
}

/// Holds only this fixture's transport, worker, and safe route log.
pub(crate) struct Source {
    /// Loopback-only single-attempt client.
    pub(crate) client: HttpClient,
    /// This owner's synthetic origin.
    pub(crate) origin: String,
    /// Ends only the owned worker.
    stop: Arc<AtomicBool>,
    /// Worker joined when the final owner drops.
    worker: Option<thread::JoinHandle<()>>,
    /// Paths only; headers and credentials are never recorded.
    routes: Arc<Mutex<Vec<String>>>,
}

impl Drop for Source {
    /// Stops and joins this worker without touching other processes.
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

/// Builds an invented GET form that echoes the supplied controls exactly.
pub(super) fn form(query: &SpellQuery) -> String {
    let url = query.website_url(super::WEBSITE).expect("synthetic URL");
    let mut body = "<form action='/spells' method='get'>".to_owned();
    body.push_str(&format!(
        "<input type='text' name='filter-search' value='{}'>",
        query.name.as_deref().unwrap_or("")
    ));
    for (field, _, key) in ID_FILTERS {
        let selected: Vec<_> = url
            .query_pairs()
            .filter(|(k, _)| k == key)
            .map(|(_, v)| v.into_owned())
            .collect();
        body.push_str(&format!("<select name='{key}' multiple>"));
        let options: Vec<_> = if field == "levels" {
            (0..=9).collect()
        } else if field == "school_ids" {
            (3..=10).collect()
        } else {
            vec![1, 2, 7, 17, 18, 42]
        };
        for id in options {
            body.push_str(&format!(
                "<option value='{id}' {}>Invented option</option>",
                if selected.contains(&id.to_string()) {
                    "selected"
                } else {
                    ""
                }
            ));
        }
        body.push_str("</select>");
    }
    for (_, _, key) in BOOL_FILTERS.into_iter().chain([(
        "partnered_content",
        "partnered-content",
        "filter-partnered-content",
    )]) {
        let selected = url
            .query_pairs()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.into_owned())
            .unwrap_or_default();
        body.push_str(&format!("<select name='{key}'>"));
        let values = if key == "filter-concentration" {
            ["", "1", "2"]
        } else {
            ["", "t", "f"]
        };
        for value in values {
            body.push_str(&format!(
                "<option value='{value}' {}>Invented choice</option>",
                if selected == value { "selected" } else { "" }
            ));
        }
        body.push_str("</select>");
    }
    body.push_str("</form>");
    body
}

/// Builds direct invented rows, explicit emptiness, and observed paging structure.
pub(super) fn listing(query: &SpellQuery, scenario: Scenario) -> String {
    if matches!(scenario, Scenario::Malformed) {
        return "<html>Invented malformed page</html>".to_owned();
    }
    let mut body = form(query);
    body.push_str("<ul class='listing listing-rpgspell rpgspell-listing'>");
    let count = if matches!(scenario, Scenario::TooManyRecords) {
        101
    } else {
        2
    };
    if matches!(scenario, Scenario::Empty) {
        body.push_str("<li class='alert no-results'>Invented no results</li>");
    } else {
        for n in 0..count {
            let id = if matches!(scenario, Scenario::Duplicate) {
                900001
            } else {
                900001 + n
            };
            let level = query.levels.first().copied().unwrap_or(0);
            let school = match query.school_ids.first().copied().unwrap_or(7) {
                3 => "abjuration",
                4 => "conjuration",
                5 => "divination",
                6 => "enchantment",
                7 => "evocation",
                8 => "illusion",
                9 => "necromancy",
                10 => "transmutation",
                _ => "invalid",
            };
            body.push_str(&format!("<div class='info' data-type='spells' data-slug='{id}-invented-spell'><div class='spell-name'><a class='link' href='/spells/{id}-invented-spell'>Invented Spell {id}</a></div><div class='spell-level'>{level}</div><div class='spell-school'><div class='school {school}'></div></div></div>"));
        }
    }
    body.push_str("</ul>");
    if !matches!(scenario, Scenario::Incomplete) {
        body.push_str("<div class='listing-footer'></div>");
    }
    if matches!(scenario, Scenario::Next | Scenario::MissingNextHref) || query.page > 1 {
        body.push_str(&format!(
            "<ul class='b-pagination-list'><li class='b-pagination-item active'>{}</li>",
            query.page
        ));
        if query.page > 1 {
            let mut first = query.clone();
            first.page = 1;
            let mut target = first.website_url(super::WEBSITE).expect("synthetic first");
            let filters: Vec<_> = target
                .query_pairs()
                .filter(|(key, _)| key != "page")
                .map(|(key, value)| (key.into_owned(), value.into_owned()))
                .collect();
            target.set_query(None);
            target.query_pairs_mut().extend_pairs(filters);
            body.push_str(&format!(
                "<li><a href='{}'>1</a></li>",
                target.as_str().replace('&', "&amp;")
            ));
        }
        if matches!(scenario, Scenario::Next) {
            let mut next = query.clone();
            next.page += 1;
            let target = next.website_url(super::WEBSITE).expect("synthetic next");
            body.push_str(&format!(
                "<li><a rel='next' href='{}'>Next</a></li>",
                target.as_str().replace('&', "&amp;")
            ));
        }
        if matches!(scenario, Scenario::MissingNextHref) {
            body.push_str("<li><a rel='next'>Next</a></li>");
        }
        body.push_str("</ul>");
    }
    body
}

/// Builds selected invented HTML with surrounding account and comment chrome.
pub(super) fn detail(slug: &str, scenario: Scenario) -> String {
    if matches!(scenario, Scenario::Unsupported) {
        return "<html><form>Invented purchase/login page</form></html>".to_owned();
    }
    let canonical = if matches!(scenario, Scenario::WrongCanonical) {
        "900099-other"
    } else {
        slug
    };
    let identity = if matches!(scenario, Scenario::WrongIdentity) {
        "900099-other"
    } else {
        slug
    };
    let mut body = format!(
        "<link rel='canonical' href='{}/spells/{canonical}'><h1 class='page-title'>Invented Spell</h1><nav class='account'>Outside</nav><div class='spell-details {identity}-details'><section class='more-info'></section><section class='detail-content'><div class='ddb-statblock ddb-statblock-spell'>",
        super::WEBSITE
    );
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
        body.push_str(&format!("<div class='ddb-statblock-item-{field}'><div class='ddb-statblock-item-label'>Invented label</div><div class='ddb-statblock-item-value'>Invented value</div></div>"));
    }
    body.push_str("</div><div class='more-info-content'>Synthetic spell detail.</div><div class='available-for'>Invented Class</div>");
    if !matches!(scenario, Scenario::MissingMarkers) {
        body.push_str("<div class='spell-source'>Invented Source</div>");
    }
    body.push_str("<aside data-future='yes'>Unknown invented element</aside>");
    if matches!(scenario, Scenario::UnsafeDetail) {
        body.push_str("<form>Inside</form>");
    }
    if matches!(scenario, Scenario::LargeResult) {
        body.push_str(&"\u{1}".repeat(180_000));
    }
    body.push_str(
        "</section></div><div class='comments'>Outside comments</div><form>Outside form</form>",
    );
    body
}

/// Decodes only this fixture's canonical generated URL into shared query input.
fn query(route: &str) -> SpellQuery {
    let url =
        reqwest::Url::parse(&format!("https://www.dndbeyond.com{route}")).expect("synthetic route");
    let mut value = serde_json::Map::new();
    for (key, data) in url.query_pairs() {
        if key == "page" {
            value.insert(
                "page".to_owned(),
                serde_json::json!(data.parse::<u32>().expect("page")),
            );
        } else if key == "filter-search" {
            value.insert("name".to_owned(), serde_json::json!(data));
        } else if let Some((field, _, _)) = ID_FILTERS.iter().find(|(_, _, k)| *k == key) {
            value
                .entry((*field).to_owned())
                .or_insert_with(|| serde_json::json!([]))
                .as_array_mut()
                .expect("array")
                .push(serde_json::json!(data.parse::<u32>().expect("ID")));
        } else if key == "filter-partnered-content" {
            value.insert(
                "partnered_content".to_owned(),
                serde_json::json!(match data.as_ref() {
                    "t" => "yes",
                    "f" => "no",
                    _ => "any",
                }),
            );
        } else if let Some((field, _, _)) = BOOL_FILTERS.iter().find(|(_, _, k)| *k == key) {
            value.insert(
                (*field).to_owned(),
                serde_json::json!(matches!(data.as_ref(), "t" | "1")),
            );
        }
    }
    serde_json::from_value(value.into()).expect("synthetic query")
}

/// Starts a bounded cookie-only loopback HTML service.
fn start(scenario: Scenario) -> std::io::Result<Source> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    listener.set_nonblocking(true)?;
    let origin = format!("http://{}", listener.local_addr()?);
    let client = HttpClient::for_fixture(&origin).map_err(std::io::Error::other)?;
    let stop = Arc::new(AtomicBool::new(false));
    let routes = Arc::new(Mutex::new(Vec::new()));
    let flag = Arc::clone(&stop);
    let log = Arc::clone(&routes);
    let worker = thread::spawn(move || {
        while !flag.load(Ordering::SeqCst) {
            let (mut stream, _) = match listener.accept() {
                Ok(v) => v,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(2));
                    continue;
                }
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => break,
            };
            if stream.set_nonblocking(false).is_err()
                || stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .is_err()
                || stream
                    .set_write_timeout(Some(Duration::from_secs(2)))
                    .is_err()
            {
                break;
            }
            let mut headers = Vec::new();
            let mut byte = [0];
            while !headers.ends_with(b"\r\n\r\n") && headers.len() < 16 * 1024 {
                match stream.read(&mut byte) {
                    Ok(1) => headers.push(byte[0]),
                    _ => break,
                }
                if !headers.ends_with(b"\r\n\r\n") {
                    continue;
                }
            }
            let request = String::from_utf8_lossy(&headers);
            let route = request.split_whitespace().nth(1).unwrap_or_default();
            log.lock().expect("synthetic log").push(route.to_owned());
            if matches!(scenario, Scenario::Waiting) {
                while !flag.load(Ordering::SeqCst) {
                    thread::sleep(Duration::from_millis(2));
                }
                break;
            }
            let headers = request.to_ascii_lowercase();
            let valid = request.starts_with("GET ")
                && headers.contains("\r\ncookie:")
                && !headers.contains("\r\nauthorization:");
            let (status, body) = if !valid {
                (401, String::new())
            } else {
                match scenario {
                    Scenario::Denied => (403, String::new()),
                    Scenario::Rejected => (401, String::new()),
                    Scenario::Missing => (404, String::new()),
                    Scenario::Redirect => (302, String::new()),
                    Scenario::Oversized => (200, " ".repeat(1024 * 1024 + 1)),
                    _ => (
                        200,
                        if let Some(slug) = route.strip_prefix("/spells/") {
                            detail(slug, scenario)
                        } else {
                            listing(&query(route), scenario)
                        },
                    ),
                }
            };
            let reply = format!(
                "HTTP/1.1 {status} Synthetic\r\nContent-Type: text/html\r\nLocation: /spells/900099-other\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = stream.write_all(reply.as_bytes());
        }
    });
    Ok(Source {
        client,
        origin,
        stop,
        worker: Some(worker),
        routes,
    })
}

/// Creates an isolated synthetic session without opening native credential storage.
///
/// # Errors
///
/// Returns only local loopback or synthetic storage setup failures.
pub fn session(scenario: Scenario) -> std::io::Result<Session> {
    let auth_scenario = if matches!(scenario, Scenario::Expired) {
        auth::Scenario::Expired
    } else {
        auth::Scenario::Valid
    };
    let mut session = auth::session(auth_scenario).map_err(std::io::Error::other)?;
    session.spell_fixture = Some(Arc::new(start(scenario)?));
    Ok(session)
}

/// Returns safe route-only evidence for request budget and no-fan-out checks.
pub fn requested_routes(session: &Session) -> Vec<String> {
    session
        .spell_fixture
        .as_ref()
        .map(|s| s.routes.lock().expect("synthetic log").clone())
        .unwrap_or_default()
}
