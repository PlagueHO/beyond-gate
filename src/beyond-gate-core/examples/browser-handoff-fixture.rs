//! Development-only native host for isolated, synthetic browser feasibility.
//!
//! This binary is not a product host. Its pending reply is injected test
//! authorization, not authenticated CLI proof. It accepts only the invented
//! profile, never validates an account, and never opens credential storage.
//! Reports contain process and event metadata, not cookies or message payloads.

use std::{
    error::Error,
    fmt,
    fs::File,
    io::{self, Read},
    path::{Path, PathBuf},
    time::Duration,
};

use beyond_gate_core::auth::handoff::{
    Browser, ErrorCode, FrameReader, FrameWriter, HandoffError, Message, Profile, Protocol,
    parse_message,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::{runtime::Builder, time::Instant};
use zeroize::Zeroizing;

/// Distinguishes missing test setup and safe protocol errors without reflecting inputs.
enum FixtureError {
    Io(io::Error),
    Json(serde_json::Error),
    Handoff(HandoffError),
    InvalidSetup,
    UnexpectedMessage,
}

impl fmt::Display for FixtureError {
    /// Reports only fixed test diagnostics or the shared redacted error.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) if error.kind() == io::ErrorKind::NotFound => {
                formatter.write_str("synthetic host setup file is missing")
            }
            Self::Io(_) => formatter.write_str("synthetic host I/O failed"),
            Self::Json(error) if error.is_eof() => {
                formatter.write_str("synthetic host setup is incomplete")
            }
            Self::Json(_) => formatter.write_str("synthetic host setup JSON is invalid"),
            Self::Handoff(error) => fmt::Display::fmt(error, formatter),
            Self::InvalidSetup => formatter.write_str("synthetic host setup is invalid"),
            Self::UnexpectedMessage => formatter.write_str("synthetic host message is unexpected"),
        }
    }
}

impl fmt::Debug for FixtureError {
    /// Prevents debug output from exposing setup or protocol data.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, formatter)
    }
}

impl Error for FixtureError {
    /// Exposes only the already-redacted handoff cause.
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Handoff(error) => Some(error),
            _ => None,
        }
    }
}

/// Selects one explicitly synthetic host behavior.
#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
enum Mode {
    Receive,
    NoPending,
}

/// Reads non-secret test setup from this copied fixture executable's own directory.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Control {
    context: String,
    browser: String,
    origin: String,
    mode: Mode,
}

/// Records receipt only; none of these fields is application-peer authentication.
#[derive(Serialize)]
struct Report {
    pid: u32,
    origin_matches: bool,
    hello_received: bool,
    submit_received: bool,
    validation_performed: bool,
    storage_used: bool,
}

/// Bounds test setup reads without using a product configuration or discovery file.
fn read_file(path: &Path, limit: usize) -> Result<Vec<u8>, FixtureError> {
    let file = File::open(path).map_err(FixtureError::Io)?;
    let mut bytes = Vec::new();
    file.take((limit + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(FixtureError::Io)?;
    if bytes.len() > limit {
        return Err(FixtureError::InvalidSetup);
    }
    Ok(bytes)
}

/// Rejects all setup except the explicit fake context, browser, and extension ID.
fn setup(directory: &Path) -> Result<(Control, Browser, Value), FixtureError> {
    let control: Control =
        serde_json::from_slice(&read_file(&directory.join("control.json"), 4096)?)
            .map_err(FixtureError::Json)?;
    let browser = match control.browser.as_str() {
        "chrome" => Browser::Chrome,
        "edge" => Browser::Edge,
        _ => return Err(FixtureError::InvalidSetup),
    };
    let id = control
        .origin
        .strip_prefix("chrome-extension://")
        .and_then(|value| value.strip_suffix('/'))
        .ok_or(FixtureError::InvalidSetup)?;
    if control.context != "beyond-gate-synthetic-v1"
        || id.len() != 32
        || !id.bytes().all(|byte| (b'a'..=b'p').contains(&byte))
    {
        return Err(FixtureError::InvalidSetup);
    }
    let fixture: Value =
        serde_json::from_slice(&read_file(&directory.join("fixture.json"), 64 * 1024)?)
            .map_err(FixtureError::Json)?;
    if fixture["synthetic_only"] != true
        || fixture["fixture_version"] != 1
        || fixture["profile"]["id"] != "synthetic-v1"
    {
        return Err(FixtureError::InvalidSetup);
    }
    Ok((control, browser, fixture))
}

/// Writes only known metadata to the unique test directory.
fn report(directory: &Path, value: &Report) -> Result<(), FixtureError> {
    let bytes = serde_json::to_vec(value).map_err(FixtureError::Json)?;
    std::fs::write(directory.join("host-report.json"), bytes).map_err(FixtureError::Io)
}

/// Encodes a known synthetic message into a zeroizing frame payload.
fn payload(message: &Value) -> Result<Zeroizing<Vec<u8>>, FixtureError> {
    serde_json::to_vec(message)
        .map(Zeroizing::new)
        .map_err(FixtureError::Json)
}

/// Exchanges two real browser frames without any service-validation or save operation.
async fn run(directory: PathBuf) -> Result<(), FixtureError> {
    let (control, browser, fixture) = setup(&directory)?;
    let mut observed = Report {
        pid: std::process::id(),
        origin_matches: std::env::args().nth(1).as_deref() == Some(&control.origin),
        hello_received: false,
        submit_received: false,
        validation_performed: false,
        storage_used: false,
    };
    report(&directory, &observed)?;
    if !observed.origin_matches {
        return Err(FixtureError::InvalidSetup);
    }
    let mut reader = FrameReader::new(tokio::io::stdin(), Protocol::Browser);
    let mut writer = FrameWriter::new(tokio::io::stdout(), Protocol::Browser);
    let hello = reader
        .read_message(Instant::now() + Duration::from_secs(10))
        .await
        .map_err(FixtureError::Handoff)?;
    if !matches!(hello, Message::Initiate {
        browser: declared,
        profile: Profile::SyntheticV1,
    } if declared == browser)
    {
        return Err(FixtureError::UnexpectedMessage);
    }
    observed.hello_received = true;
    report(&directory, &observed)?;
    if matches!(control.mode, Mode::NoPending) {
        return writer
            .write_message(
                payload(&fixture["messages"]["browser_error"])?,
                Instant::now() + Duration::from_secs(5),
            )
            .await
            .map_err(FixtureError::Handoff);
    }
    let pending = &fixture["messages"]["browser_pending"];
    let expected = match parse_message(Protocol::Browser, payload(pending)?.to_vec())
        .map_err(FixtureError::Handoff)?
    {
        Message::Authorization {
            transaction,
            profile: Profile::SyntheticV1,
            ..
        } => transaction,
        _ => return Err(FixtureError::InvalidSetup),
    };
    writer
        .write_message(payload(pending)?, Instant::now() + Duration::from_secs(5))
        .await
        .map_err(FixtureError::Handoff)?;
    let submit = reader
        .read_message(Instant::now() + Duration::from_secs(15))
        .await
        .map_err(FixtureError::Handoff)?;
    if !matches!(submit, Message::Submit {
        transaction,
        profile: Profile::SyntheticV1,
        ..
    } if transaction == expected)
    {
        return Err(FixtureError::UnexpectedMessage);
    }
    observed.submit_received = true;
    report(&directory, &observed)?;
    let mut reply = fixture["messages"]["browser_error"].clone();
    reply["transaction"] = Value::String(expected.as_str().to_owned());
    reply["code"] = Value::String(ErrorCode::UnsupportedContract.as_str().to_owned());
    writer
        .write_message(payload(&reply)?, Instant::now() + Duration::from_secs(5))
        .await
        .map_err(FixtureError::Handoff)
}

/// Limits runtime shutdown because blocking standard-input tasks are not cancellable.
fn execute() -> Result<(), FixtureError> {
    let directory = std::env::current_exe()
        .map_err(FixtureError::Io)?
        .parent()
        .ok_or(FixtureError::InvalidSetup)?
        .to_path_buf();
    let runtime = Builder::new_current_thread()
        .enable_time()
        .build()
        .map_err(FixtureError::Io)?;
    let result = runtime.block_on(run(directory));
    runtime.shutdown_timeout(Duration::from_secs(1));
    result
}

/// Keeps stdout exclusively framed; diagnostics contain no cookie or setup values.
fn main() {
    if let Err(error) = execute() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
