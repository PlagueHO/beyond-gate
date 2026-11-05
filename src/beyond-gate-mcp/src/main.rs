//! Local stdio MCP server for Beyond Gate read operations.

use std::{error::Error, fmt, io, process::ExitCode};

use beyond_gate_mcp::BeyondGateServer;
use clap::Command;
use rmcp::{ServiceExt, transport::stdio};

/// Starts the local MCP server without writing diagnostics to protocol stdout.
fn main() -> ExitCode {
    #[cfg(feature = "e2e-isolation")]
    if std::env::args_os().skip(1).eq(["--e2e-context-check"]) {
        return match beyond_gate_core::auth::e2e_context() {
            Ok(_) => {
                println!("beyond-gate-e2e-isolation-v1");
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("error: {error}");
                ExitCode::FAILURE
            }
        };
    }
    Command::new("beyond-gate-mcp")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Local read-only MCP server for Beyond Gate")
        .after_help(
            "Uses stdio for MCP messages. Windows book_list uses bounded public category classification and metadata transport through the shared stored session. Other live reads are not implemented and report explicit unsupported errors.",
        )
        .get_matches();

    match run_server(serve()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

/// Runs stdio serving without waiting for blocked native workers after transport closure.
fn run_server(
    operation: impl std::future::Future<Output = Result<(), ServerFailure>>,
) -> Result<(), ServerFailure> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(ServerFailure::Runtime)?;
    let result = runtime.block_on(operation);
    runtime.shutdown_background();
    result
}

/// Retains transport causes while showing only safe failure categories.
#[derive(Debug)]
enum ServerFailure {
    Runtime(io::Error),
    Initialization(Box<dyn Error + Send + Sync>),
    Transport(Box<dyn Error + Send + Sync>),
}

impl fmt::Display for ServerFailure {
    /// Avoids printing raw protocol or transport details to stderr.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Runtime(_) => formatter.write_str("MCP server runtime could not start."),
            Self::Initialization(_) => {
                formatter.write_str("MCP stdio server could not initialize.")
            }
            Self::Transport(_) => formatter
                .write_str("MCP stdio server stopped after a transport or protocol failure."),
        }
    }
}

impl Error for ServerFailure {
    /// Keeps the typed rmcp failure available without exposing its contents.
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Runtime(error) => Some(error),
            Self::Initialization(error) | Self::Transport(error) => Some(error.as_ref()),
        }
    }
}

/// Runs the server until its stdio transport closes or fails.
async fn serve() -> Result<(), ServerFailure> {
    let service = BeyondGateServer::default()
        .serve(stdio())
        .await
        .map_err(|error| ServerFailure::Initialization(Box::new(error)))?;
    service
        .waiting()
        .await
        .map(|_| ())
        .map_err(|error| ServerFailure::Transport(Box::new(error)))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Keeps MCP shutdown independent of a native read still running after cancellation or timeout.
    #[test]
    fn transport_closure_does_not_wait_for_blocking_storage() {
        let (release, wait) = std::sync::mpsc::channel();
        let (returned, result) = std::sync::mpsc::channel();
        let caller = std::thread::spawn(move || {
            let outcome = run_server(async move {
                let (started, ready) = tokio::sync::oneshot::channel();
                tokio::task::spawn_blocking(move || {
                    started.send(()).expect("worker started");
                    wait.recv().expect("release synthetic worker");
                });
                ready.await.expect("worker running");
                Ok(())
            });
            returned.send(outcome.is_ok()).expect("report server exit");
        });
        let outcome = result.recv_timeout(std::time::Duration::from_secs(2));
        release.send(()).expect("release worker before assertions");
        caller.join().expect("MCP caller");
        assert_eq!(outcome, Ok(true));
    }
}
