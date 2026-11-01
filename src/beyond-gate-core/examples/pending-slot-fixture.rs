//! Feature-only reservation checks with a public, disjoint synthetic namespace.
//!
//! No pairing key, account input, browser query, network request, or credential
//! store is used. A holder exits after three seconds; a probe drops immediately.

use std::{io::Write, process::ExitCode, time::Duration};

use beyond_gate_core::auth::handoff::{
    Message, Protocol, Transaction, parse_message, slot::PendingLoginSlot,
};

/// Validates a public correlation suffix using the existing strict message parser.
fn namespace(text: &str) -> Result<Transaction, ()> {
    if text.len() != 32 {
        return Err(());
    }
    let bytes = serde_json::to_vec(&serde_json::json!({
        "protocol": "bg-host-cli",
        "version": 1,
        "type": "cancel",
        "transaction": text,
    }))
    .map_err(|_| ())?;
    match parse_message(Protocol::HostCli, bytes).map_err(|_| ())? {
        Message::Cancel {
            transaction: Some(transaction),
        } => Ok(transaction),
        _ => Err(()),
    }
}

/// Claims only the synthetic namespace and emits one fixed readiness record.
fn run() -> Result<(), &'static str> {
    let mut arguments = std::env::args().skip(1);
    let role = arguments
        .next()
        .ok_or("pending slot fixture setup is invalid")?;
    let suffix = arguments
        .next()
        .ok_or("pending slot fixture setup is invalid")?;
    if !matches!(role.as_str(), "hold" | "probe") || arguments.next().is_some() {
        return Err("pending slot fixture setup is invalid");
    }
    let transaction = namespace(&suffix).map_err(|_| "pending slot fixture setup is invalid")?;
    let slot = PendingLoginSlot::claim_fixture(&transaction)
        .map_err(|_| "pending slot fixture reservation is unavailable")?;
    {
        let mut output = std::io::stdout().lock();
        writeln!(output, "{{\"fixture\":\"bg-pending-slot-v1\",\"claimed\":true,\"validated\":false,\"saved\":false}}")
            .map_err(|_| "pending slot fixture metadata failed")?;
        output
            .flush()
            .map_err(|_| "pending slot fixture metadata failed")?;
    }
    if role == "hold" {
        std::thread::sleep(Duration::from_secs(3));
    }
    drop(slot);
    Ok(())
}

/// Reports only fixed failures and releases owned resources before normal exit.
fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}
