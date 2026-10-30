//! Development-only setup for a separately approved isolated native-store test.

use beyond_gate_core::auth::{SessionSecret, SessionStore, e2e_context};

/// Seeds only synthetic bytes or removes the current isolated test entry.
fn main() -> std::process::ExitCode {
    let result = execute();
    match result {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("error: {message}");
            std::process::ExitCode::FAILURE
        }
    }
}

/// Rejects arbitrary input and performs no store access for a context check.
fn execute() -> Result<(), String> {
    let arguments: Vec<_> = std::env::args().skip(1).collect();
    let context = e2e_context().map_err(|error| error.to_string())?;
    if arguments == ["check"] {
        println!("beyond-gate-e2e-isolation-v1");
        return Ok(());
    }
    if arguments != ["seed"] && arguments != ["cleanup"] {
        return Err("select check, seed, or cleanup".into());
    }
    let store = SessionStore::open().map_err(|error| error.to_string())?;
    if arguments == ["seed"] {
        let secret = SessionSecret::new(format!("synthetic-only-{context}").into_bytes())
            .map_err(|error| error.to_string())?;
        store.save(&secret).map_err(|error| error.to_string())?;
    } else {
        store.logout().map_err(|error| error.to_string())?;
    }
    Ok(())
}
