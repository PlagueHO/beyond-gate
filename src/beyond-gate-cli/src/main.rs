//! Command-line entry point for local session management and blocked login.

use std::process::ExitCode;

use beyond_gate_core::{
    FOUNDATION_NOTICE,
    auth::{self, AuthError, SessionStatus, SessionStore},
};
use clap::{ArgMatches, Command};

/// Enumerates only the supported local auth operations.
#[derive(Clone, Copy)]
enum AuthAction {
    Login,
    Status,
    Logout,
}

/// Builds resource-first help without opening storage or starting account access.
///
/// Clap writes help and version output to stdout and exits successfully.
/// With no arguments, it writes help to stderr and exits with code 2.
fn command() -> Command {
    Command::new("beyond-gate")
        .bin_name("beyond-gate")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Local-first access to authorized D&D Beyond data")
        .after_help(FOUNDATION_NOTICE)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("auth")
                .about("Inspect or remove the protected local session; browser login is blocked")
                .arg_required_else_help(true)
                .subcommand_required(true)
                .subcommand(
                    Command::new("login")
                        .about("Report the browser-login blocker without collecting credentials"),
                )
                .subcommand(
                    Command::new("status")
                        .about("Report local presence only, without checking upstream usability"),
                )
                .subcommand(
                    Command::new("logout")
                        .about("Remove the local credential without upstream revocation"),
                ),
        )
}

/// Converts clap's validated nested command into a local auth action.
fn auth_action(matches: &ArgMatches) -> Option<AuthAction> {
    match matches.subcommand_matches("auth")?.subcommand_name()? {
        "login" => Some(AuthAction::Login),
        "status" => Some(AuthAction::Status),
        "logout" => Some(AuthAction::Logout),
        _ => None,
    }
}

/// Runs one action with injectable operations so tests never open a native store.
///
/// # Errors
///
/// Returns the shared auth failure without converting it into a success message.
fn execute_auth(
    action: AuthAction,
    status: impl FnOnce() -> Result<SessionStatus, AuthError>,
    logout: impl FnOnce() -> Result<(), AuthError>,
) -> Result<&'static str, AuthError> {
    match action {
        AuthAction::Login => Err(auth::login_blocker()),
        AuthAction::Status => Ok(match status()? {
            SessionStatus::Absent => "absent: no local Beyond Gate session is stored.",
            SessionStatus::StoredUnverified => {
                "stored-unverified: local session material exists; upstream usability is not checked; expiry is unknown."
            }
        }),
        AuthAction::Logout => {
            logout()?;
            Ok(
                "Local Beyond Gate session removed or already absent. Upstream revocation is not implemented; the browser session can remain active.",
            )
        }
    }
}

/// Keeps results on stdout and actionable failures on stderr.
fn main() -> ExitCode {
    let matches = command().get_matches();
    let Some(action) = auth_action(&matches) else {
        eprintln!("error: select an auth operation; see beyond-gate --help");
        return ExitCode::from(2);
    };
    match execute_auth(
        action,
        || SessionStore::open()?.status(),
        || SessionStore::open()?.logout(),
    ) {
        Ok(message) => {
            println!("{message}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests;
