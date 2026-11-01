//! Development-only two-process encrypted handoff with compiled synthetic keys.
//!
//! This is not the product CLI or native host. It uses IPv4 loopback only,
//! has a ten-second lifetime, and never validates an account or opens storage.
//! Arguments select only a fixture role, public port, and invented wrong-key case.

use std::{
    io::{Read, Write},
    net::{Ipv4Addr, SocketAddr},
    path::Path,
    process::ExitCode,
    time::Duration,
};

use beyond_gate_core::auth::handoff::{
    ErrorCode, HandoffError, Message, Transaction,
    flow::PendingHandoff,
    pairing::{PendingPairing, RandomnessError},
    transport::PairedConnection,
};
use serde_json::Value;
use tokio::{
    net::{TcpListener, TcpStream},
    runtime::Builder,
    time::{Instant, timeout},
};
use zeroize::Zeroizing;

/// Restricts fixture failures to fixed diagnostics, never arguments or OS details.
enum FixtureError {
    Setup,
    Io,
    Deadline,
    Handoff(HandoffError),
    Randomness(RandomnessError),
}

impl std::fmt::Display for FixtureError {
    /// Prints only fixed fixture text or the core's redacted error.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Setup => formatter.write_str("paired fixture setup is invalid"),
            Self::Io => formatter.write_str("paired fixture local I/O failed"),
            Self::Deadline => formatter.write_str("paired fixture deadline reached"),
            Self::Handoff(error) => std::fmt::Display::fmt(error, formatter),
            Self::Randomness(error) => std::fmt::Display::fmt(error, formatter),
        }
    }
}

/// Produces only the compiled invented pairing key, with an explicit wrong-key variant.
fn key(wrong: bool) -> Zeroizing<Vec<u8>> {
    Zeroizing::new(vec![if wrong { 43 } else { 42 }; 32])
}

/// Loads bounded, repository-owned synthetic examples without accepting a caller path.
fn fixture() -> Result<Value, FixtureError> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("docs")
        .join("fixtures")
        .join("browser-handoff-v1.json");
    let file = std::fs::File::open(path).map_err(|_| FixtureError::Io)?;
    let mut bytes = Zeroizing::new(Vec::new());
    file.take(65537)
        .read_to_end(&mut bytes)
        .map_err(|_| FixtureError::Io)?;
    if bytes.len() > 65536 {
        return Err(FixtureError::Setup);
    }
    let fixture: Value = serde_json::from_slice(&bytes).map_err(|_| FixtureError::Setup)?;
    if fixture["synthetic_only"] != true || fixture["fixture_version"] != 1 {
        return Err(FixtureError::Setup);
    }
    Ok(fixture)
}

/// Serializes only a named canonical synthetic message in zeroizing memory.
fn payload(fixture: &Value, name: &str) -> Result<Zeroizing<Vec<u8>>, FixtureError> {
    serde_json::to_vec(&fixture["messages"][name])
        .map(Zeroizing::new)
        .map_err(|_| FixtureError::Setup)
}

/// Writes fixed success metadata without serializing any message or credential.
fn receipt() -> Result<(), FixtureError> {
    writeln!(
        std::io::stdout().lock(),
        "{{\"fixture\":\"bg-paired-process-v1\",\"received\":true,\"validated\":false,\"saved\":false}}"
    )
    .map_err(|_| FixtureError::Io)
}

/// Consumes one synthetic ticket and confirms receipt only after actual encrypted submit.
async fn listen(fixture: &Value) -> Result<(), FixtureError> {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
        .await
        .map_err(|_| FixtureError::Io)?;
    let port = listener.local_addr().map_err(|_| FixtureError::Io)?.port();
    writeln!(
        std::io::stdout().lock(),
        "{{\"fixture\":\"bg-paired-process-v1\",\"port\":{port}}}"
    )
    .map_err(|_| FixtureError::Io)?;
    let deadline = Instant::now() + Duration::from_secs(8);
    let ticket = PendingPairing::new(key(false), deadline).map_err(FixtureError::Handoff)?;
    let transaction = Transaction::generate().map_err(FixtureError::Randomness)?;
    let pending = PendingHandoff::new(transaction, deadline).map_err(FixtureError::Handoff)?;
    let (stream, peer) = timeout(Duration::from_secs(5), listener.accept())
        .await
        .map_err(|_| FixtureError::Deadline)?
        .map_err(|_| FixtureError::Io)?;
    if peer.ip() != Ipv4Addr::LOCALHOST {
        return Err(FixtureError::Setup);
    }
    drop(listener);
    receive(fixture, stream, ticket, pending, deadline).await
}

/// Performs the same bounded receipt-only application flow over either local transport.
async fn receive<S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin>(
    fixture: &Value,
    stream: S,
    ticket: PendingPairing,
    pending: PendingHandoff,
    deadline: Instant,
) -> Result<(), FixtureError> {
    let connection = PairedConnection::accept(stream, ticket)
        .await
        .map_err(FixtureError::Handoff)?;
    let (mut connection, received) = pending
        .receive(connection)
        .await
        .map_err(FixtureError::Handoff)?;
    let transaction = received.transaction().as_str().to_owned();
    drop(received.into_value());
    let mut error = fixture["messages"]["cli_error"].clone();
    error["transaction"] = Value::String(transaction.clone());
    error["code"] = Value::String(ErrorCode::UnsupportedContract.as_str().to_owned());
    let error = serde_json::to_vec(&error).map_err(|_| FixtureError::Setup)?;
    connection
        .write_message(Zeroizing::new(error), deadline)
        .await
        .map_err(FixtureError::Handoff)?;
    if !matches!(
        connection.read_message(deadline).await.map_err(FixtureError::Handoff)?,
        Message::Cancel { transaction: Some(bound) } if bound.as_str() == transaction
    ) {
        return Err(FixtureError::Setup);
    }
    drop(connection);
    receipt()
}

/// Connects only to the explicitly selected local fixture and checks the safe receipt result.
async fn host(fixture: &Value, port: &str, mode: &str) -> Result<(), FixtureError> {
    let port = port.parse::<u16>().map_err(|_| FixtureError::Setup)?;
    if port == 0 {
        return Err(FixtureError::Setup);
    }
    let address = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
    let stream = timeout(Duration::from_secs(5), TcpStream::connect(address))
        .await
        .map_err(|_| FixtureError::Deadline)?
        .map_err(|_| FixtureError::Io)?;
    let deadline = Instant::now() + Duration::from_secs(5);
    send(fixture, stream, mode, deadline).await
}

/// Checks real authorization and receipt without treating a terminal error as login success.
async fn send<S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin>(
    fixture: &Value,
    stream: S,
    mode: &str,
    deadline: Instant,
) -> Result<(), FixtureError> {
    let mut connection = PairedConnection::connect(stream, key(mode == "wrong-key"), deadline)
        .await
        .map_err(FixtureError::Handoff)?;
    connection
        .write_message(payload(fixture, "cli_open")?, deadline)
        .await
        .map_err(FixtureError::Handoff)?;
    let transaction = match connection
        .read_message(deadline)
        .await
        .map_err(FixtureError::Handoff)?
    {
        Message::Authorization { transaction, .. } => transaction,
        _ => return Err(FixtureError::Setup),
    };
    let mut submit = fixture["messages"]["cli_submit"].clone();
    let mut correlation = transaction.as_str().to_owned();
    if mode == "wrong-transaction" {
        let replacement = if correlation.starts_with('0') {
            "1"
        } else {
            "0"
        };
        correlation.replace_range(..1, replacement);
    }
    submit["transaction"] = Value::String(correlation);
    let submit = serde_json::to_vec(&submit).map_err(|_| FixtureError::Setup)?;
    connection
        .write_message(Zeroizing::new(submit), deadline)
        .await
        .map_err(FixtureError::Handoff)?;
    if !matches!(
        connection.read_message(deadline).await.map_err(FixtureError::Handoff)?,
        Message::Error { transaction: Some(bound), code: ErrorCode::UnsupportedContract }
            if bound == transaction
    ) {
        return Err(FixtureError::Setup);
    }
    let mut cancel = fixture["messages"]["cli_cancel"].clone();
    cancel["transaction"] = Value::String(transaction.as_str().to_owned());
    let cancel = serde_json::to_vec(&cancel).map_err(|_| FixtureError::Setup)?;
    connection
        .write_message(Zeroizing::new(cancel), deadline)
        .await
        .map_err(FixtureError::Handoff)?;
    match connection.read_message(deadline).await {
        Err(error)
            if matches!(
                error.code(),
                ErrorCode::ProtocolError | ErrorCode::Disconnected
            ) => {}
        Err(error) => return Err(FixtureError::Handoff(error)),
        Ok(_) => return Err(FixtureError::Setup),
    }
    receipt()
}

/// Validates the public fixture namespace through the existing cancellation parser.
#[cfg(windows)]
fn namespace(value: &str) -> Result<Transaction, FixtureError> {
    if value.len() != 32 {
        return Err(FixtureError::Setup);
    }
    let message = serde_json::to_vec(&serde_json::json!({
        "protocol": "bg-host-cli", "version": 1, "type": "cancel", "transaction": value
    }))
    .map_err(|_| FixtureError::Setup)?;
    match beyond_gate_core::auth::handoff::parse_message(
        beyond_gate_core::auth::handoff::Protocol::HostCli,
        message,
    )
    .map_err(FixtureError::Handoff)?
    {
        Message::Cancel {
            transaction: Some(transaction),
        } => Ok(transaction),
        _ => Err(FixtureError::Setup),
    }
}

/// Uses a real, protected single-instance Windows endpoint and the same generated authorization.
#[cfg(windows)]
async fn listen_pipe(fixture: &Value, value: &str) -> Result<(), FixtureError> {
    use beyond_gate_core::auth::handoff::{endpoint::WindowsEndpoint, slot::PendingLoginSlot};
    let namespace = namespace(value)?;
    let slot = PendingLoginSlot::claim_fixture(&namespace).map_err(|_| FixtureError::Io)?;
    let deadline = Instant::now() + Duration::from_secs(8);
    let endpoint = WindowsEndpoint::bind(&slot, deadline).map_err(FixtureError::Handoff)?;
    {
        let mut output = std::io::stdout().lock();
        writeln!(
            output,
            "{{\"fixture\":\"bg-paired-pipe-v1\",\"listening\":true}}"
        )
        .map_err(|_| FixtureError::Io)?;
        output.flush().map_err(|_| FixtureError::Io)?;
    }
    let ticket = PendingPairing::new(key(false), deadline).map_err(FixtureError::Handoff)?;
    let transaction = Transaction::generate().map_err(FixtureError::Randomness)?;
    let pending = PendingHandoff::new(transaction, deadline).map_err(FixtureError::Handoff)?;
    let stream = endpoint.accept().await.map_err(FixtureError::Handoff)?;
    receive(fixture, stream, ticket, pending, deadline).await
}

/// Opens only the synthetic Windows endpoint with narrow rights and anonymous security QoS.
#[cfg(windows)]
async fn host_pipe(fixture: &Value, value: &str, mode: &str) -> Result<(), FixtureError> {
    use beyond_gate_core::auth::handoff::endpoint::WindowsStream;
    let namespace = namespace(value)?;
    let deadline = Instant::now() + Duration::from_secs(5);
    let stream =
        WindowsStream::connect_fixture(&namespace, deadline).map_err(FixtureError::Handoff)?;
    send(fixture, stream, mode, deadline).await
}

/// Rejects all roles and variants except the compiled synthetic process scenarios.
async fn execute() -> Result<(), FixtureError> {
    let arguments: Vec<_> = std::env::args().skip(1).collect();
    let fixture = fixture()?;
    match arguments.as_slice() {
        #[cfg(windows)]
        [role, namespace] if role == "listen-pipe" => listen_pipe(&fixture, namespace).await,
        #[cfg(windows)]
        [role, namespace, mode]
            if role == "host-pipe"
                && matches!(mode.as_str(), "good" | "wrong-key" | "wrong-transaction") =>
        {
            host_pipe(&fixture, namespace, mode).await
        }
        [role] if role == "listen" => listen(&fixture).await,
        [role, port, mode]
            if role == "host"
                && matches!(mode.as_str(), "good" | "wrong-key" | "wrong-transaction") =>
        {
            host(&fixture, port, mode).await
        }
        _ => Err(FixtureError::Setup),
    }
}

/// Keeps the complete fixture bounded and emits no message payload or secret in failures.
fn main() -> ExitCode {
    let result = (|| {
        let runtime = Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|_| FixtureError::Io)?;
        runtime.block_on(async {
            timeout(Duration::from_secs(10), execute())
                .await
                .map_err(|_| FixtureError::Deadline)?
        })
    })();
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
