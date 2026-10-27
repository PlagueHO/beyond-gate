//! Command-line entry point for the Beyond Gate foundation.

use beyond_gate_core::FOUNDATION_NOTICE;
use clap::Command;

/// Parses help and version requests without starting account operations.
///
/// Clap writes help and version output to stdout and exits successfully.
/// With no arguments, it writes help to stderr and exits with code 2.
/// `env!` reads the package version at compile time, not from the user's environment.
fn main() {
    Command::new("beyond-gate")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Local-first access to authorized D&D Beyond data")
        .after_help(FOUNDATION_NOTICE)
        .arg_required_else_help(true)
        .get_matches();
}
