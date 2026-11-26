//! Compiled loopback JSON responses with invented records and no runtime URL overrides.

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
    /// The source offset does not match the request.
    Incomplete,
    /// A detail response names a different ID.
    ChangedIdentity,
    /// Public lookup metadata is unavailable after a valid detail response.
    ConfigurationDenied,
    /// Public configuration contains a malformed lookup row.
    InvalidConfiguration,
    /// A body exceeds the transport's 1 MiB cap.
    Oversized,
    /// JSON padded to exactly the transport's 1 MiB cap.
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
        "alignmentId":8, "sizeId":4, "typeId":16, "challengeRatingId":3,
        "conditionImmunities":[4], "damageAdjustments":[22], "environments":[9,999],
        "languages":[{"languageId":17,"notes":"Invented language note"}],
        "movements":[{"movementId":1,"speed":20,"notes":null}],
        "savingThrows":[{"statId":5,"bonusModifier":null}],
        "senses":[{"senseId":2,"notes":"Invented sense note"}],
        "skills":[{"skillId":3,"value":4}], "subTypes":[17],
        "stats": [{"statId":1,"value":12,"future_stat":null}],
        "actionsDescription": "<p>Synthetic action only.</p>",
        "unknown": {"nested":[1,null,{"original":"invented"}]}
    })
}

/// Supplies invented public labels with the same table shapes as the official client.
pub(super) fn configuration() -> Value {
    json!({
        "alignments":[{"id":8,"name":"Invented alignment"}],
        "challengeRatings":[{"id":3,"value":0.25}],
        "creatureSizes":[{"id":4,"name":"Invented size"}],
        "monsterTypes":[{"id":16,"name":"Invented type"}],
        "monsterSubTypes":[{"id":17,"name":"Invented subtype"}],
        "conditions":[{"definition":{"id":4,"name":"Invented condition","description":"Not a label"}}],
        "damageAdjustments":[{"id":22,"name":"Invented adjustment","type":2}],
        "environments":[{"id":9,"name":"Invented environment"}],
        "languages":[{"id":17,"name":"Invented language"}],
        "movements":[{"id":1,"name":"Invented movement"}],
        "stats":[{"id":1,"name":"Invented ability"},{"id":5,"name":"Invented save"}],
        "senses":[{"id":2,"name":"Invented sense"}],
        "abilitySkills":[{"id":3,"name":"Invented skill"}],
        "sources":[{"id":7,"name":"Must not be resolved"}]
    })
}

/// Wraps invented records and exact grants in a single default-size final batch.
pub(super) fn collection(entries: Vec<Value>) -> Value {
    let grants: serde_json::Map<String, Value> = entries
        .iter()
        .map(|entry| (entry["id"].to_string(), json!(1)))
        .collect();
    json!({
        "pagination":{"skip":0,"take":10,"total":entries.len()},
        "data":entries,"accessType":grants,"stats":{},"metaData":{}
    })
}

/// Creates the exact fixed response for a checked synthetic JSON route.
fn reply(scenario: Scenario, route: &str) -> (u16, String) {
    if route == "/api/config/json" {
        if matches!(scenario, Scenario::ConfigurationDenied) {
            return (403, "{}".to_owned());
        }
        let mut value = configuration();
        if matches!(scenario, Scenario::InvalidConfiguration) {
            value["stats"][0]["name"] = json!("\ninvalid");
        }
        return (200, value.to_string());
    }
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
        let pairs: Vec<_> = url.query_pairs().collect();
        let number = |key| {
            pairs
                .iter()
                .find(|(k, _)| k == key)
                .expect("explicit pagination")
                .1
                .parse::<u32>()
                .expect("numeric pagination")
        };
        assert!(pairs.iter().any(|(k, v)| k == "showHomebrew" && v == "f"));
        assert!(!pairs.iter().any(|(k, _)| k == "ids" || k == "page"));
        let (skip, take) = (number("skip"), number("take"));
        let total: u32 = if matches!(scenario, Scenario::Empty)
            || pairs.iter().any(|(k, v)| k == "search" && v == "missing")
        {
            0
        } else if matches!(scenario, Scenario::WideTable) {
            100
        } else {
            3
        };
        let entries = (0..take.min(total.saturating_sub(skip)))
            .map(|offset| {
                let id = 17 + u64::from(skip) + u64::from(offset);
                let mut value = record(id);
                if id == 17 && matches!(scenario, Scenario::WideTable) {
                    value["name"] = json!("x".repeat(900_000));
                }
                value
            })
            .collect();
        let mut value = collection(entries);
        value["pagination"] = json!({
            "skip":if matches!(scenario, Scenario::Incomplete) { skip + 1 } else { skip },
            "take":take,"total":total
        });
        value
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

/// Checks bearer-only monster routes and a credential-free public configuration route.
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
            let public = route == "/api/config/json";
            let (status, body) = if request.starts_with("GET ")
                && (public || route.starts_with("/v1/Monster"))
                && headers.contains("\r\nauthorization:") != public
                && !headers.contains("\r\ncookie:")
            {
                reply(scenario, route)
            } else {
                (401, "{}".to_owned())
            };
            let response = format!(
                "HTTP/1.1 {status} Synthetic\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
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

/// Keeps invented request paths observable after an adapter takes ownership of its session.
pub fn route_log(session: &Session) -> Arc<Mutex<Vec<String>>> {
    Arc::clone(
        &session
            .monster_fixture
            .as_ref()
            .expect("monster fixture owner")
            .routes,
    )
}

/// Returns only observed synthetic paths to assert request bounds and encoding.
pub fn routes(session: &Session) -> Vec<String> {
    route_log(session).lock().expect("fixture routes").clone()
}
