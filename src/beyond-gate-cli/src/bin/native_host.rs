//! Native messaging entry point; stdout contains only bounded protocol frames.
//!
//! The Windows bridge accepts the pinned manual package origin and a one-use
//! pairing preface. Other platforms fail before reading input. No cookie query,
//! account request, native-store access, or host registration is performed here.

use std::process::ExitCode;

/// Checks launch syntax before allocating runtime workers or reading browser input.
#[cfg(windows)]
fn execute() -> ExitCode {
    use beyond_gate_core::auth::handoff::browser;
    use std::time::Duration;

    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    if let Err(error) = browser::check_launch(&arguments) {
        eprintln!("error: {error}");
        return ExitCode::FAILURE;
    }
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            eprintln!("error: native host runtime failed ({:?})", error.kind());
            return ExitCode::FAILURE;
        }
    };
    let result = runtime.block_on(browser::serve(
        &arguments,
        tokio::io::stdin(),
        tokio::io::stdout(),
    ));
    runtime.shutdown_timeout(Duration::from_secs(1));
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

/// Fails without a runtime, input read, or platform-specific substitute.
#[cfg(not(windows))]
fn execute() -> ExitCode {
    eprintln!("error: Windows native browser login is unsupported on this platform");
    ExitCode::FAILURE
}

/// Returns a process status after bounded shutdown of blocking standard-I/O workers.
fn main() -> ExitCode {
    execute()
}
