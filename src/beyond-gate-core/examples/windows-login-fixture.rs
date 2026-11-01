//! Explicit development-only Windows browser/native/core completion processes.
//!
//! No product endpoint, native credential store, account request, or real cookie is used.

use std::process::ExitCode;

/// Starts only the feature-gated synthetic fixture and prints fixed failures.
fn main() -> ExitCode {
    #[cfg(windows)]
    {
        let arguments = std::env::args_os().skip(1).collect::<Vec<_>>();
        match beyond_gate_core::auth::establishment::fixture::run(&arguments) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                let code = if let Some(error) = error.downcast_ref::<beyond_gate_core::auth::handoff::HandoffError>() {
                    error.code().as_str()
                } else if let Some(error) = error.downcast_ref::<beyond_gate_core::auth::handoff::completion::CompletionError>() {
                    error.code().as_str()
                } else if let Some(error) = error.downcast_ref::<std::io::Error>() {
                    match error.kind() {
                        std::io::ErrorKind::InvalidInput => "fixture_invalid_input",
                        std::io::ErrorKind::NotFound => "fixture_file_missing",
                        _ => "fixture_io_failure",
                    }
                } else {
                    "fixture_setup_failure"
                };
                eprintln!("Windows login fixture failed: {code}.");
                ExitCode::FAILURE
            }
        }
    }
    #[cfg(not(windows))]
    {
        eprintln!("Windows login fixture is supported only on Windows.");
        ExitCode::FAILURE
    }
}
