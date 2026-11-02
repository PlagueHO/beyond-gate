//! Real product codecs and completion with invented material in isolated processes.
//!
//! This feature is not a product ingress. The only accepted cookie is compiled
//! invented data. Storage is an explicit mock backend. The native relay connects
//! only to a random fixture pipe, never to the fixed product endpoint.

use std::{
    error::Error,
    ffi::OsString,
    fs::{self, File},
    io::{Read, Write},
    net::{Ipv4Addr, TcpListener},
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde_json::{Value, json};
use tokio::{
    runtime::Builder,
    time::{Instant, timeout},
};
use zeroize::Zeroizing;

use super::*;
use crate::auth::handoff::{
    Profile, Transaction, browser,
    completion::complete_with,
    endpoint::{WindowsEndpoint, WindowsStream},
    flow::PendingHandoff,
    pairing::PendingPairing,
    slot::PendingLoginSlot,
    transport::PairedConnection,
};

const CONTEXT: &str = "bg-windows-product-fixture-v1";
const COOKIE: &[u8] = b"synthetic-session";
type FixtureResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

/// Returns a typed invalid-input cause without reflecting caller data.
fn invalid() -> Box<dyn Error + Send + Sync> {
    Box::new(std::io::Error::new(
        std::io::ErrorKind::InvalidInput,
        "invalid synthetic fixture context",
    ))
}

/// Resolves only a copied host beneath the known, unique fixture directory.
fn directory() -> FixtureResult<PathBuf> {
    let executable = fs::canonicalize(std::env::current_exe()?)?;
    let directory = executable.parent().ok_or_else(invalid)?;
    let root = fs::canonicalize(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("target")
            .join("browser-feasibility"),
    )?;
    let name = directory
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(invalid)?;
    if directory.parent() != Some(root.as_path())
        || !(name.starts_with("chrome-") || name.starts_with("edge-"))
        || !name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        || executable.file_name().and_then(|name| name.to_str()) != Some("host.exe")
    {
        return Err(invalid());
    }
    Ok(directory.to_owned())
}

/// Reads only bounded public fixture control, never a caller-selected credential file.
fn control(directory: &std::path::Path) -> FixtureResult<Transaction> {
    let mut bytes = Vec::new();
    File::open(directory.join("control.json"))?
        .take(4097)
        .read_to_end(&mut bytes)?;
    if bytes.len() > 4096 {
        return Err(invalid());
    }
    let control: Value = serde_json::from_slice(&bytes)?;
    if control["context"] != CONTEXT {
        return Err(invalid());
    }
    let namespace = control["namespace"].as_str().ok_or_else(invalid)?;
    let bytes = serde_json::to_vec(&json!({
        "protocol": "bg-host-cli", "version": 1, "type": "cancel", "transaction": namespace,
    }))?;
    match crate::auth::handoff::parse_message(crate::auth::handoff::Protocol::HostCli, bytes)? {
        crate::auth::handoff::Message::Cancel {
            transaction: Some(namespace),
        } => Ok(namespace),
        _ => Err(invalid()),
    }
}

/// Records public process ownership without writing any protocol input or credentials.
fn host_report(directory: &std::path::Path) -> FixtureResult<()> {
    fs::write(
        directory.join("host-report.json"),
        serde_json::to_vec(&json!({
            "fixture": CONTEXT, "pid": std::process::id(),
            "live_validated": false, "native_saved": false,
        }))?,
    )?;
    Ok(())
}

/// Runs the actual relay through a fixture-only named pipe with the compiled key.
async fn native(arguments: &[OsString]) -> FixtureResult<()> {
    browser::check_launch(arguments)?;
    let directory = directory()?;
    let namespace = control(&directory)?;
    host_report(&directory)?;
    browser::run_with(
        tokio::io::stdin(),
        tokio::io::stdout(),
        |key, deadline| async move {
            let stream = WindowsStream::connect_fixture(&namespace, deadline)?;
            PairedConnection::connect(stream, key, deadline).await
        },
    )
    .await?;
    Ok(())
}

/// Builds one invented unsigned token whose claims satisfy the accepted test rule.
fn response() -> FixtureResult<Vec<u8>> {
    let claims = json!({
        "http://schemas.xmlsoap.org/ws/2005/05/identity/claims/nameidentifier": "42",
        "exp": 4102444800u64,
    });
    let token = format!(
        "{}.{}.{}",
        URL_SAFE_NO_PAD.encode(b"{}"),
        URL_SAFE_NO_PAD.encode(serde_json::to_vec(&claims)?),
        URL_SAFE_NO_PAD.encode(b"synthetic-signature")
    );
    Ok(serde_json::to_vec(&json!({ "token": token }))?)
}

/// Accepts at most one bounded loopback request and responds without forwarding traffic.
fn service(
    mode: &str,
    count: Arc<AtomicUsize>,
) -> FixtureResult<(String, std::thread::JoinHandle<FixtureResult<()>>)> {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?;
    listener.set_nonblocking(true)?;
    let address = listener.local_addr()?;
    let status = if mode == "rejected" { 401 } else { 200 };
    let delayed = mode == "cancelled";
    let body = response()?;
    let worker = std::thread::spawn(move || {
        let deadline = std::time::Instant::now() + Duration::from_secs(3);
        let mut stream = loop {
            match listener.accept() {
                Ok((stream, peer)) if peer.ip() == Ipv4Addr::LOCALHOST => break stream,
                Ok(_) => return Err(invalid()),
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    if std::time::Instant::now() >= deadline {
                        return Ok(());
                    }
                    std::thread::sleep(Duration::from_millis(5));
                }
                Err(error) => return Err(error.into()),
            }
        };
        stream.set_nonblocking(false)?;
        stream.set_read_timeout(Some(Duration::from_secs(2)))?;
        stream.set_write_timeout(Some(Duration::from_secs(2)))?;
        let mut request = Zeroizing::new(Vec::new());
        while !request.windows(4).any(|bytes| bytes == b"\r\n\r\n") {
            let mut buffer = [0; 1024];
            let length = stream.read(&mut buffer)?;
            if length == 0 && delayed {
                return Ok(());
            }
            if length == 0 || request.len() + length > 16 * 1024 {
                return Err(invalid());
            }
            request.extend_from_slice(&buffer[..length]);
        }
        if !request.starts_with(b"POST /v1/cobalt-token HTTP/1.1\r\n")
            || !std::str::from_utf8(&request)?
                .to_ascii_lowercase()
                .contains("cookie: cobaltsession=synthetic-session\r\n")
        {
            return Err(invalid());
        }
        count.fetch_add(1, Ordering::SeqCst);
        if delayed {
            std::thread::sleep(Duration::from_millis(250));
        }
        let output = format!(
            "HTTP/1.1 {status} Fixture\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        let result = stream
            .write_all(output.as_bytes())
            .and_then(|()| stream.write_all(&body));
        match result {
            Ok(()) => Ok(()),
            Err(error)
                if delayed
                    && matches!(
                        error.kind(),
                        std::io::ErrorKind::BrokenPipe | std::io::ErrorKind::ConnectionReset
                    ) =>
            {
                Ok(())
            }
            Err(error) => Err(error.into()),
        }
    });
    Ok((format!("http://{address}/v1/cobalt-token"), worker))
}

/// Rejects non-fixture bytes before any service or store operation.
async fn establish(
    input: SessionEstablishmentInput,
    control: LoginControl,
    client: Client,
    url: String,
    store: Arc<SessionStore>,
    unavailable: bool,
) -> Result<SavedSession, EstablishmentError> {
    if input.cookie.0.as_slice() != COOKIE {
        return Err(EstablishmentError::new(
            crate::auth::handoff::ErrorCode::InvalidMaterial,
        ));
    }
    establish_with(input, control, client, &url, move || {
        if unavailable {
            Err(AuthError::new(AuthErrorKind::StorageUnavailable))
        } else {
            Ok(store)
        }
    })
    .await
}

/// Uses actual pending, pairing, validation, save, and acknowledgement controllers.
async fn cli(mode: &str) -> FixtureResult<()> {
    let namespace = Transaction::generate()?;
    let slot = Arc::new(PendingLoginSlot::claim_fixture(&namespace)?);
    let deadline = Instant::now() + Duration::from_secs(30);
    let endpoint = WindowsEndpoint::bind(&slot, deadline)?;
    let ticket = PendingPairing::new(Zeroizing::new(vec![42; 32]), deadline)?;
    let pending =
        PendingHandoff::for_profile(Profile::CobaltSessionV1, Transaction::generate()?, deadline)?;
    let control = LoginControl::new(Arc::clone(&slot), deadline)?;
    let backend = keyring_core::mock::Store::new()?;
    let store = Arc::new(SessionStore::from_store(backend.as_ref(), None)?);
    let target = Arc::clone(&store);
    let count = Arc::new(AtomicUsize::new(0));
    let (url, service) = service(mode, Arc::clone(&count))?;
    let client = direct_client_builder()
        .connect_timeout(Duration::from_secs(2))
        .timeout(Duration::from_secs(3))
        .build()?;
    writeln!(
        std::io::stdout().lock(),
        "{}",
        json!({
            "fixture": CONTEXT, "ready": true, "namespace": namespace.as_str(),
        })
    )?;
    let stream = endpoint.accept_for_pairing().await?;
    let paired = PairedConnection::accept(stream, ticket).await?;
    let (paired, receipt) = pending.receive(paired).await?;
    let result = complete_with(paired, receipt, control, |input, control| {
        establish(
            input,
            control,
            client,
            url,
            target,
            mode == "storage-unavailable",
        )
    })
    .await;
    service.join().map_err(|_| invalid())??;
    let outcome = match result {
        Ok(_) => "saved",
        Err(error) => error.code().as_str(),
    };
    let in_memory_saved = match store.entry.get_secret() {
        Ok(bytes) => {
            let cookie = BrowserSessionCookie::from_persisted(SessionSecret::new(bytes)?)?;
            cookie.0.as_slice() == COOKIE
        }
        Err(keyring_core::Error::NoEntry) => false,
        Err(error) => return Err(error.into()),
    };
    writeln!(
        std::io::stdout().lock(),
        "{}",
        json!({
            "fixture": CONTEXT, "cli_outcome": outcome,
            "service_requests": count.load(Ordering::SeqCst), "in_memory_saved": in_memory_saved,
            "live_validated": false, "native_saved": false,
        })
    )?;
    Ok(())
}

/// Runs only a known fixture CLI role or a checked browser launch.
///
/// # Errors
///
/// Returns typed setup, framing, pairing, local mock-service, or completion causes.
/// Callers must print fixed diagnostics, never arbitrary source text.
pub fn run(arguments: &[OsString]) -> FixtureResult<()> {
    let mode = if arguments.len() == 2 && arguments[0] == "fixture-cli" {
        let mode = arguments[1].to_str().ok_or_else(invalid)?;
        if !matches!(
            mode,
            "saved" | "rejected" | "cancelled" | "lost-ack" | "storage-unavailable"
        ) {
            return Err(invalid());
        }
        Some(mode)
    } else {
        browser::check_launch(arguments)?;
        None
    };
    let runtime = Builder::new_current_thread().enable_all().build()?;
    let result = runtime.block_on(async {
        timeout(Duration::from_secs(35), async {
            match mode {
                Some(mode) => cli(mode).await,
                None => native(arguments).await,
            }
        })
        .await
        .map_err(|_| invalid())?
    });
    runtime.shutdown_timeout(Duration::from_secs(1));
    result
}

/// Handles delayed, fragmented HTTP headers on Windows without inheriting nonblocking reads.
#[cfg(test)]
#[test]
fn mock_service_waits_for_fragmented_request() {
    let count = Arc::new(AtomicUsize::new(0));
    let (url, worker) = service("saved", Arc::clone(&count)).expect("loopback mock service");
    let address = url
        .strip_prefix("http://")
        .and_then(|url| url.strip_suffix("/v1/cobalt-token"))
        .expect("fixture address");
    let mut client = std::net::TcpStream::connect(address).expect("loopback connection");
    client
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("bounded response read");
    std::thread::sleep(Duration::from_millis(100));
    client
        .write_all(b"POST /v1/cobalt-token HTTP/1.1\r\n")
        .expect("partial request");
    std::thread::sleep(Duration::from_millis(100));
    client
        .write_all(b"Cookie: cobaltSession=synthetic-session\r\n\r\n")
        .expect("remaining headers");
    let mut received = Vec::new();
    client
        .read_to_end(&mut received)
        .expect("complete response");
    worker.join().expect("mock worker").expect("served request");
    let body = response().expect("invented response");
    let mut expected = format!(
        "HTTP/1.1 200 Fixture\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    )
    .into_bytes();
    expected.extend_from_slice(&body);
    assert_eq!(received, expected);
    assert_eq!(count.load(Ordering::SeqCst), 1);
}

/// Invented fixture material cannot become a general cookie or native-store ingress.
#[cfg(test)]
#[tokio::test]
async fn non_fixture_cookie_is_rejected_before_exchange_or_save() {
    let namespace = Transaction::generate().expect("public namespace");
    let slot = Arc::new(PendingLoginSlot::claim_fixture(&namespace).expect("private fixture slot"));
    let control =
        LoginControl::new(slot, Instant::now() + Duration::from_secs(5)).expect("control");
    let backend = keyring_core::mock::Store::new().expect("in-memory backend");
    let store =
        Arc::new(SessionStore::from_store(backend.as_ref(), None).expect("in-memory entry"));
    let input = SessionEstablishmentInput::new(
        BrowserSessionCookie::new(b"other-invented-value".to_vec()).expect("valid invented syntax"),
    );
    let result = establish(
        input,
        control,
        direct_client_builder().build().expect("client"),
        "http://127.0.0.1:9/v1/cobalt-token".to_owned(),
        Arc::clone(&store),
        false,
    )
    .await;
    assert_eq!(
        result.expect_err("fixture admission").code(),
        crate::auth::handoff::ErrorCode::InvalidMaterial
    );
    assert!(matches!(
        store.entry.get_secret(),
        Err(keyring_core::Error::NoEntry)
    ));
}
