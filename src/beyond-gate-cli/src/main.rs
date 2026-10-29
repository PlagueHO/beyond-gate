//! Native CLI entry point; all parsing and presentation use the shared runner.

use std::io;

/// Keeps stdout and stderr independent and delegates reads to the core boundary.
fn main() {
    std::process::exit(beyond_gate_cli::run(
        std::env::args_os(),
        &beyond_gate_cli::LiveReads,
        &mut io::stdout().lock(),
        &mut io::stderr().lock(),
    ))
}
