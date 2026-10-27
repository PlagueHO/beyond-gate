//! Entry point reserved for the future local stdio MCP server.

use std::process::ExitCode;

use beyond_gate_core::FOUNDATION_NOTICE;
use clap::Command;

/// Handles help and version requests, or reports that serving is unavailable.
///
/// `ExitCode` lets `main` report failure without terminating the process from a
/// helper. Diagnostics use stderr so stdout remains reserved for MCP messages.
fn main() -> ExitCode {
    Command::new("beyond-gate-mcp")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Local stdio MCP server for Beyond Gate (not implemented)")
        .after_help(FOUNDATION_NOTICE)
        .get_matches();

    eprintln!("error: {FOUNDATION_NOTICE}");
    ExitCode::FAILURE
}
