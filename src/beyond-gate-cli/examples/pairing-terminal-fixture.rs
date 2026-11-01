//! Feature-only terminal preview with an invented key, no pending login, and no store.

use std::{
    process::ExitCode,
    time::{Duration, Instant},
};

use beyond_gate_cli::pairing::PairingDisplay;
use zeroize::Zeroizing;

/// Shows invented material briefly or returns a safe error before nonterminal output.
///
/// # Errors
///
/// Returns fixed setup or redacted display errors without any account operation.
fn preview() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::args().skip(1).collect::<Vec<_>>() != ["preview"] {
        return Err("select the synthetic preview role".into());
    }
    let display = PairingDisplay::open(
        Zeroizing::new(vec![42; 32]),
        Instant::now() + Duration::from_secs(8),
    )?;
    std::thread::sleep(Duration::from_secs(5));
    display.close()?;
    println!(
        "{{\"fixture\":\"bg-terminal-pairing-v1\",\"closed\":true,\"validated\":false,\"saved\":false}}"
    );
    Ok(())
}

/// Reports only safe diagnostics; normal product commands expose no fixture role.
fn main() -> ExitCode {
    match preview() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
