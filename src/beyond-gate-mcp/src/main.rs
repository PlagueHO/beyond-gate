//! Local stdio MCP server for Beyond Gate read operations.

mod server;

use std::{error::Error, fmt, process::ExitCode};

use clap::Command;
use rmcp::{ServiceExt, transport::stdio};
use server::BeyondGateServer;

/// Starts the local MCP server without writing diagnostics to protocol stdout.
#[tokio::main]
async fn main() -> ExitCode {
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
            "Uses stdio for MCP messages. Live reads are not implemented; valid read tools report explicit unsupported errors.",
        )
        .get_matches();

    match serve().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

/// Retains transport causes while showing only safe failure categories.
#[derive(Debug)]
enum ServerFailure {
    Initialization(Box<dyn Error + Send + Sync>),
    Transport(Box<dyn Error + Send + Sync>),
}

impl fmt::Display for ServerFailure {
    /// Avoids printing raw protocol or transport details to stderr.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
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
            Self::Initialization(error) | Self::Transport(error) => Some(error.as_ref()),
        }
    }
}

/// Runs the server until its stdio transport closes or fails.
async fn serve() -> Result<(), ServerFailure> {
    let service = BeyondGateServer
        .serve(stdio())
        .await
        .map_err(|error| ServerFailure::Initialization(Box::new(error)))?;
    service
        .waiting()
        .await
        .map(|_| ())
        .map_err(|error| ServerFailure::Transport(Box::new(error)))
}
