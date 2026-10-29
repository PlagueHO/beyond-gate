//! Native CLI entry point; all parsing and presentation use the shared runner.

use std::{io, process::ExitCode};

/// Keeps stdout and stderr independent and delegates reads to the core boundary.
fn main() -> ExitCode {
    ExitCode::from(beyond_gate_cli::run(
        std::env::args_os(),
        &beyond_gate_cli::LiveReads,
        &mut io::stdout().lock(),
        &mut io::stderr().lock(),
    ))
}
