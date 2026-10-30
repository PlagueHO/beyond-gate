//! Native CLI entry point; all parsing and presentation use the shared runner.

use std::io;

/// Keeps stdout and stderr independent and delegates reads to the core boundary.
fn main() {
    #[cfg(feature = "e2e-isolation")]
    if std::env::args_os().skip(1).eq(["--e2e-context-check"]) {
        match beyond_gate_core::auth::e2e_context() {
            Ok(_) => println!("beyond-gate-e2e-isolation-v1"),
            Err(error) => {
                eprintln!("error: {error}");
                std::process::exit(1);
            }
        }
        return;
    }
    std::process::exit(beyond_gate_cli::run(
        std::env::args_os(),
        &beyond_gate_cli::LiveReads,
        &mut io::stdout().lock(),
        &mut io::stderr().lock(),
    ))
}
