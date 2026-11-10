//! Compiled loopback monster responses with invented records and no runtime URL overrides.

use crate::{
    auth::session::{Session, fixture as auth},
    http::HttpClient,
};
use serde_json::{Value, json};
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

/// Selects fixed synthetic source, access, pagination, and session failures.
#[derive(Clone, Copy)]
pub enum Scenario {
    /// Three records, including duplicate names and unknown detail fields.
    Normal,
    /// A complete empty page.
    Empty,
    /// The service denies the HTTP request.
    Denied,
    /// The HTTP request rejects the bearer.
    Rejected,
    /// The exchange provides already expired evidence.
    Expired,
    /// HTTP success supplies access code 4.
    DeniedCode,
    /// HTTP success supplies unverified access code 2.
    UnsupportedCode,
    /// HTTP success supplies unverified access code 3.
    SharedCode,
    /// One otherwise valid page contains a denied row.
    MixedAccess,
    /// Enough uneven name widths to exceed padded table policy, not JSON policy.
    WideTable,
    /// The access field or selected access entry is absent.
    MissingAccess,
    /// The envelope is not a valid source response.
    Malformed,
    /// The website's active page does not match the request.
    Incomplete,
    /// A detail response names a different ID.
    ChangedIdentity,
    /// A body exceeds the transport's 1 MiB cap.
    Oversized,
    /// Valid website HTML and JSON padded to exactly the transport's 1 MiB cap.
    ExactBody,
    /// List permits code 1, but the selected detail now denies it.
    AccessLost,
    /// Leaves the resource waiting until cancellation or deadline.
    Waiting,
}

/// Owns one local service, its transport, and non-secret request-path observations.
pub(crate) struct Source {
    /// Selects the synthetic transport only in compiled fixture builds.
    pub(crate) client: HttpClient,
    /// Supplies a loopback origin, never a caller-controlled production URL.
    pub(crate) origin: String,
    /// Stops only this worker.
    stop: Arc<AtomicBool>,
    /// Joins the bounded worker after its last owner is dropped.
    worker: Option<thread::JoinHandle<()>>,
    /// Keeps request paths but no headers or credentials.
    routes: Arc<Mutex<Vec<String>>>,
}

impl Drop for Source {
    /// Stops and joins the exact owned fixture thread.
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

/// Builds an invented record with unknown nested data and synthetic description text.
pub(super) fn record(id: u64) -> Value {
    json!({
        "id": id, "name": "Invented Beast",
        "url": format!("https://www.dndbeyond.com/monsters/{id}-invented"),
        "isHomebrew": false, "sources": [{"sourceId":7,"future_reference":true}],
        "stats": [{"statId":1,"value":12,"future_stat":null}],
        "actionsDescription": "<p>Synthetic action only.</p>",
        "unknown": {"nested":[1,null,{"original":"invented"}]}
    })
}

/// Wraps exactly selected records and grants in the observed collection envelope.
pub(super) fn collection(entries: Vec<Value>) -> Value {
    let grants: serde_json::Map<String, Value> = entries
        .iter()
        .map(|entry| (entry["id"].to_string(), json!(1)))
        .collect();
    json!({
        "data":entries,"accessType":grants,"stats":{},"metaData":{},
        "pagination":null
    })
}

/// Creates the exact fixed response for a checked synthetic route.
fn reply(scenario: Scenario, route: &str) -> (u16, String) {
    match scenario {
        Scenario::Denied => return (403, "{}".to_owned()),
        Scenario::Rejected => return (401, "{}".to_owned()),
        Scenario::Malformed => return (200, "{}".to_owned()),
        Scenario::Oversized => return (200, " ".repeat(1024 * 1024 + 1)),
        _ => {}
    }
    let url = reqwest::Url::parse(&format!("http://127.0.0.1{route}"))
        .expect("fixed fixture request URL");
    let mut value = if url.path() == "/v1/Monster" {
        let entries = url
            .query_pairs()
            .map(|(key, value)| {
                assert_eq!(key, "ids");
                let id = value.parse::<u64>().expect("selected fixture ID");
                let mut value = record(id);
                if id == 17 && matches!(scenario, Scenario::WideTable) {
                    value["name"] = json!("x".repeat(90_000));
                }
                value
            })
            .collect();
        let mut entries: Vec<Value> = entries;
        entries.reverse();
        collection(entries)
    } else if let Some(id) = url
        .path()
        .strip_prefix("/v1/Monster/")
        .and_then(|id| id.parse::<u64>().ok())
    {
        if !(17..=19).contains(&id) {
            return (404, "{}".to_owned());
        }
        let returned_id = if matches!(scenario, Scenario::ChangedIdentity) {
            id + 1
        } else {
            id
        };
        json!({
            "data":record(returned_id),
            "accessType":if matches!(scenario, Scenario::AccessLost) { 4 } else { 1 },
            "stats":null
        })
    } else {
        return (404, "{}".to_owned());
    };
    match scenario {
        Scenario::DeniedCode | Scenario::UnsupportedCode | Scenario::SharedCode => {
            let code = match scenario {
                Scenario::DeniedCode => 4,
                Scenario::SharedCode => 3,
                _ => 2,
            };
            if let Some(grants) = value["accessType"].as_object_mut() {
                for grant in grants.values_mut() {
                    *grant = json!(code);
                }
            } else {
                value["accessType"] = json!(code);
            }
        }
        Scenario::MissingAccess => {
            value
                .as_object_mut()
                .expect("fixture object")
                .remove("accessType");
        }
        Scenario::MixedAccess => {
            if value["accessType"].is_object() {
                value["accessType"]["18"] = json!(4);
            } else {
                value["accessType"] = json!(4);
            }
        }
        _ => {}
    }
    let mut body = value.to_string();
    if matches!(scenario, Scenario::ExactBody) {
        body.push_str(&" ".repeat(1024 * 1024 - body.len()));
    }
    (200, body)
}

/// Builds public synthetic controls without account data or source snapshots.
fn filter_form() -> String {
    use super::query::{BOOL_FILTERS, CHALLENGE_RATINGS, ID_FILTERS, NUMBER_FILTERS};
    let mut form =
        String::from("<form method=\"get\" action=\"/monsters\"><input name=\"filter-search\">");
    for (_, _, field) in ID_FILTERS {
        if field == "filter-type" {
            for id in 0..=50 {
                form.push_str(&format!(
                    "<input type=\"checkbox\" name=\"{field}\" value=\"{id}\">"
                ));
            }
        } else {
            form.push_str(&format!("<select multiple name=\"{field}\">"));
            for id in 1..=50 {
                form.push_str(&format!("<option value=\"{id}\">Invented option</option>"));
            }
            form.push_str("</select>");
        }
    }
    for field in ["filter-cr-min", "filter-cr-max"] {
        form.push_str(&format!(
            "<select name=\"{field}\"><option value=\"\"></option>"
        ));
        for (_, id) in CHALLENGE_RATINGS {
            form.push_str(&format!("<option value=\"{id}\">Invented rating</option>"));
        }
        form.push_str("</select>");
    }
    for (_, _, field) in BOOL_FILTERS
        .into_iter()
        .chain([("", "", "filter-partnered-content")])
    {
        form.push_str(&format!("<select name=\"{field}\"><option value=\"\"></option><option value=\"t\">Yes</option><option value=\"f\">No</option></select>"));
    }
    for (_, _, field) in NUMBER_FILTERS {
        form.push_str(&format!("<input name=\"{field}\">"));
    }
    form.push_str("</form>");
    form
}

/// Creates one invented website page with explicit empty and next-page markers.
pub(super) fn website(scenario: Scenario, route: &str) -> (u16, String) {
    match scenario {
        Scenario::Denied => return (403, String::new()),
        Scenario::Rejected => return (401, String::new()),
        Scenario::Malformed => return (200, "{}".to_owned()),
        Scenario::Oversized => return (200, " ".repeat(1024 * 1024 + 1)),
        _ => {}
    }
    let url = reqwest::Url::parse(&format!("http://127.0.0.1{route}")).expect("fixture URL");
    let page = url
        .query_pairs()
        .find(|(key, _)| key == "page")
        .and_then(|(_, value)| value.parse::<u32>().ok())
        .expect("page");
    let empty = matches!(scenario, Scenario::Empty)
        || page > 2
        || url
            .query_pairs()
            .any(|(key, value)| key == "filter-search" && value == "missing");
    let ids = if empty {
        Vec::new()
    } else if matches!(scenario, Scenario::WideTable) {
        (17..117).collect()
    } else if page == 1 {
        vec![17, 18]
    } else {
        vec![19]
    };
    let mut body = filter_form();
    body.push_str(
        "<div class=\"listing-header\"></div><div class=\"listing-body\"><ul class=\"listing listing-rpgmonster rpgmonster-listing\">",
    );
    if empty {
        body.push_str("<li class=\"alert no-results\">You failed your Investigation check.</li>");
    } else {
        for id in ids {
            body.push_str(&format!("<li><div class=\"info\"><div class=\"row monster-name\"><a href=\"/monsters/{id}-invented\">Invented Beast</a></div></div></li>"));
        }
    }
    body.push_str("</ul></div><div class=\"listing-footer\"></div>");
    if !empty {
        let active = if matches!(scenario, Scenario::Incomplete) {
            page + 1
        } else {
            page
        };
        body.push_str(&format!("<ul class=\"b-pagination-list\"><li><span class=\"b-pagination-item s-active active\">{active}</span></li>"));
        if page == 1 && !matches!(scenario, Scenario::WideTable) {
            let mut next = url.clone();
            let pairs: Vec<_> = url
                .query_pairs()
                .filter(|(key, _)| key != "page")
                .map(|(k, v)| (k.into_owned(), v.into_owned()))
                .collect();
            next.set_query(None);
            next.query_pairs_mut()
                .extend_pairs(pairs)
                .append_pair("page", "2");
            let href = format!("{}?{}", next.path(), next.query().expect("next query"))
                .replace('&', "&amp;");
            body.push_str(&format!(
                "<li><a rel=\"next\" href=\"{href}\">Next</a></li>"
            ));
        }
        body.push_str("</ul>");
    }
    if matches!(scenario, Scenario::ExactBody) {
        body.push_str(&" ".repeat(1024 * 1024 - body.len()));
    }
    (200, body)
}

/// Starts only an ephemeral loopback source; no account or native store is accessed.
fn source(scenario: Scenario) -> std::io::Result<Source> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    listener.set_nonblocking(true)?;
    let origin = format!("http://{}", listener.local_addr()?);
    let client = HttpClient::for_fixture(&origin).map_err(std::io::Error::other)?;
    let stop = Arc::new(AtomicBool::new(false));
    let routes = Arc::new(Mutex::new(Vec::new()));
    let flag = Arc::clone(&stop);
    let paths = Arc::clone(&routes);
    let worker = thread::spawn(move || {
        while !flag.load(Ordering::SeqCst) {
            let (mut stream, _) = match listener.accept() {
                Ok(connection) => connection,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(2));
                    continue;
                }
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
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
            let mut header = Vec::new();
            let mut byte = [0];
            while !header.ends_with(b"\r\n\r\n") && header.len() < 16 * 1024 {
                if stream.read_exact(&mut byte).is_err() {
                    break;
                }
                header.push(byte[0]);
            }
            let request = String::from_utf8_lossy(&header);
            let route = request.split_whitespace().nth(1).unwrap_or_default();
            paths.lock().expect("fixture routes").push(route.to_owned());
            if matches!(scenario, Scenario::Waiting) {
                while !flag.load(Ordering::SeqCst) {
                    thread::sleep(Duration::from_millis(2));
                }
                break;
            }
            let headers = request.to_ascii_lowercase();
            let html = route.starts_with("/monsters?");
            let (status, body) = if request.starts_with("GET ")
                && html
                && headers.contains("\r\ncookie:")
                && !headers.contains("\r\nauthorization:")
            {
                website(scenario, route)
            } else if request.starts_with("GET ")
                && !html
                && headers.contains("\r\nauthorization:")
                && !headers.contains("\r\ncookie:")
            {
                reply(scenario, route)
            } else {
                (401, "{}".to_owned())
            };
            let media = if html {
                "text/html"
            } else {
                "application/json"
            };
            let response = format!(
                "HTTP/1.1 {status} Synthetic\r\nContent-Type: {media}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = stream.write_all(response.as_bytes());
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

/// Supplies a synthetic authenticated owner with one fixed invented source scenario.
///
/// # Errors
///
/// Returns a local listener or synthetic-session setup error.
pub fn session(scenario: Scenario) -> Result<Session, Box<dyn std::error::Error>> {
    let mut session = auth::session(if matches!(scenario, Scenario::Expired) {
        auth::Scenario::Expired
    } else {
        auth::Scenario::Valid
    })?;
    session.monster_fixture = Some(Arc::new(source(scenario)?));
    Ok(session)
}

/// Returns only observed synthetic paths to assert request bounds and encoding.
pub fn routes(session: &Session) -> Vec<String> {
    session
        .monster_fixture
        .as_ref()
        .expect("monster fixture owner")
        .routes
        .lock()
        .expect("fixture routes")
        .clone()
}
