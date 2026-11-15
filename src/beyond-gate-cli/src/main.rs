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
    let arguments: Vec<_> = std::env::args_os().collect();
    let code = if arguments.len() == 1 {
        beyond_gate_cli::run(
            arguments,
            &beyond_gate_cli::LiveReads,
            &mut io::stdout().lock(),
            &mut anstream::AutoStream::auto(io::stderr().lock()),
        )
    } else {
        beyond_gate_cli::run(
            arguments,
            &beyond_gate_cli::LiveReads,
            &mut io::stdout().lock(),
            &mut io::stderr().lock(),
        )
    };
    std::process::exit(code)
}
