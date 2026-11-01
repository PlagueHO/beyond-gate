//! Explicit Windows native-host setup without account or credential access.

use clap::ArgMatches;
#[cfg(windows)]
use std::io;
use std::{
    fmt,
    io::{BufRead, Write},
};

#[cfg(windows)]
mod windows;

/// Retains setup causes while keeping arbitrary file and registry data out of diagnostics.
#[derive(Debug)]
pub(crate) enum SetupError {
    Unsupported,
    #[cfg(windows)]
    Directory,
    #[cfg(windows)]
    Conflict,
    #[cfg(windows)]
    Changed,
    #[cfg(windows)]
    Io(io::Error),
    #[cfg(windows)]
    Json(serde_json::Error),
}

impl fmt::Display for SetupError {
    /// Reports stopped setup without making a false unchanged-state claim.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::Unsupported => "Browser native-host setup is supported only on Windows.",
            #[cfg(windows)]
            Self::Directory => {
                "Select a stable local installation directory outside source and build trees. Installation requires the CLI and native-host executables."
            }
            #[cfg(windows)]
            Self::Conflict => {
                "Different or malformed native-host setup exists. It was not overwritten or removed."
            }
            #[cfg(windows)]
            Self::Changed => "Native-host setup changed during this operation. Setup stopped.",
            #[cfg(windows)]
            Self::Io(_) => {
                "An operating-system setup operation failed. Setup is not complete; previewed local files or registrations may remain."
            }
            #[cfg(windows)]
            Self::Json(_) => "Native-host manifest JSON could not be processed. Setup stopped.",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for SetupError {
    /// Preserves typed filesystem, registry, and JSON causes.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            #[cfg(windows)]
            Self::Io(error) => Some(error),
            #[cfg(windows)]
            Self::Json(error) => Some(error),
            _ => None,
        }
    }
}

#[cfg(windows)]
impl From<io::Error> for SetupError {
    /// Retains the operating-system cause without exposing its data in display text.
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

/// Routes only parsed setup commands; unsupported platforms fail before any lookup.
pub(crate) fn run(
    arguments: &ArgMatches,
    stdin: &mut impl BufRead,
    stdout: &mut impl Write,
) -> Result<(), SetupError> {
    #[cfg(windows)]
    {
        windows::run(arguments, stdin, stdout)
    }
    #[cfg(not(windows))]
    {
        let _ = (arguments, stdin, stdout);
        Err(SetupError::Unsupported)
    }
}
