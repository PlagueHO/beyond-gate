//! Temporary terminal pairing display, with no credential ingress or login result.
//!
//! This extends the CLI's plain terminal presentation. The key has its own line;
//! instructions explain its one use and never imply account validation or save.
//! All three standard streams must be terminals. Secret output uses an owned
//! terminal handle and an alternate screen, not ordinary stdout or stderr.
//!
//! The caller must close the display on claim, cancellation, expiry, or disconnect.
//! Synchronous terminal I/O cannot be interrupted by an async timer. Terminal
//! recording, OS-owned copies, crashes, and forced termination can prevent cleanup;
//! this module does not claim protection from screen capture or terminal recording.

use std::{
    error::Error,
    fmt,
    fs::{File, OpenOptions},
    io::{self, IsTerminal, Write},
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use crossterm::{
    cursor::MoveTo,
    execute,
    terminal::{Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, size},
};
use zeroize::Zeroizing;

/// Retains an OS error behind a fixed terminal-specific source boundary.
pub struct TerminalCause(io::Error);

impl fmt::Display for TerminalCause {
    /// Reports a safe category without file names, OS text, or secret-bearing messages.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self.0.kind() {
            io::ErrorKind::PermissionDenied => "pairing terminal access was denied",
            _ => "pairing terminal I/O failed",
        })
    }
}

impl fmt::Debug for TerminalCause {
    /// Uses the same safe category as Display.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, formatter)
    }
}

impl Error for TerminalCause {}

/// Reports fixed display failures without any pairing material or raw OS text.
#[derive(Debug)]
pub enum PairingDisplayError {
    /// At least one standard stream or the owned handle is not a terminal.
    TerminalRequired,
    /// The terminal cannot show the whole key and instructions without wrapping.
    TerminalTooSmall,
    /// Another display owns this process's terminal screen.
    Busy,
    /// The input is not exactly 32 bytes.
    InvalidKey,
    /// The original pending deadline has expired.
    TimedOut,
    /// The supplied pending deadline exceeds five minutes.
    InvalidDeadline,
    /// Terminal setup, rendering, or restoration failed.
    Io(TerminalCause),
}

impl PairingDisplayError {
    /// Preserves a concrete I/O cause behind the safe formatter.
    fn io(error: io::Error) -> Self {
        Self::Io(TerminalCause(error))
    }
}

impl fmt::Display for PairingDisplayError {
    /// Returns only fixed local diagnostics, never input or dependency text.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::TerminalRequired => "pairing requires interactive stdin, stdout, and stderr",
            Self::TerminalTooSmall => {
                "pairing requires a terminal of at least 60 columns and 10 rows"
            }
            Self::InvalidKey => "pairing key size is invalid",
            Self::Busy => "another pairing display owns this terminal",
            Self::TimedOut => "pairing display deadline expired",
            Self::InvalidDeadline => "pairing display deadline exceeds five minutes",
            Self::Io(_) => "pairing terminal operation failed",
        })
    }
}

impl Error for PairingDisplayError {
    /// Exposes only the redacted typed terminal cause.
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(cause) => Some(cause),
            _ => None,
        }
    }
}

/// Prevents this process from clearing or replacing another active pairing display.
static DISPLAY_ACTIVE: AtomicBool = AtomicBool::new(false);

/// Releases process-local display ownership on every setup or rendering result.
struct DisplayLease;

impl DisplayLease {
    /// Claims the process's standard terminal without blocking or stealing its screen.
    fn claim() -> Result<Self, PairingDisplayError> {
        DISPLAY_ACTIVE
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| PairingDisplayError::Busy)?;
        Ok(Self)
    }
}

impl Drop for DisplayLease {
    /// Releases only the ownership established by this lease.
    fn drop(&mut self) {
        DISPLAY_ACTIVE.store(false, Ordering::Release);
    }
}

/// Checks every standard stream before allocating encoded secret output.
fn require_terminal(
    input: bool,
    output: bool,
    diagnostic: bool,
) -> Result<(), PairingDisplayError> {
    if input && output && diagnostic {
        Ok(())
    } else {
        Err(PairingDisplayError::TerminalRequired)
    }
}

/// Rejects expired or overlong lifetimes without changing the caller's deadline.
fn check_deadline(deadline: Instant) -> Result<(), PairingDisplayError> {
    let remaining = deadline
        .checked_duration_since(Instant::now())
        .filter(|duration| !duration.is_zero())
        .ok_or(PairingDisplayError::TimedOut)?;
    if remaining > Duration::from_secs(300) {
        return Err(PairingDisplayError::InvalidDeadline);
    }
    Ok(())
}

/// Opens only the OS terminal device; read access permits Windows console-mode checks.
fn terminal() -> Result<File, PairingDisplayError> {
    #[cfg(windows)]
    let path = "CONOUT$";
    #[cfg(not(windows))]
    let path = "/dev/tty";
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .map_err(PairingDisplayError::io)?;
    if !file.is_terminal() {
        return Err(PairingDisplayError::TerminalRequired);
    }
    Ok(file)
}

/// Converts one owned key to canonical unpadded base64url and clears raw bytes on return.
fn encode(key: Zeroizing<Vec<u8>>) -> Result<Zeroizing<String>, PairingDisplayError> {
    if key.len() != 32 {
        return Err(PairingDisplayError::InvalidKey);
    }
    Ok(Zeroizing::new(URL_SAFE_NO_PAD.encode(&*key)))
}

/// Writes the inherited plain CLI layout; only the owned terminal calls this in production.
fn render(writer: &mut impl Write, key: &str) -> io::Result<()> {
    writeln!(writer, "Beyond Gate pairing\r")?;
    writeln!(writer, "\r")?;
    writeln!(writer, "Pairing key (one use)\r")?;
    writeln!(writer, "{key}\r")?;
    writeln!(writer, "\r")?;
    writeln!(writer, "Use this key only in the browser pairing prompt.\r")?;
    writeln!(
        writer,
        "Do not put it in commands, URLs, chat, files, or logs.\r"
    )?;
    writeln!(
        writer,
        "This display does not validate or save a session.\r"
    )?;
    writer.flush()
}

/// Owns one temporary alternate screen without retaining encoded or raw key buffers.
pub struct PairingDisplay {
    screen: File,
    active: bool,
    _lease: DisplayLease,
}

impl PairingDisplay {
    /// Shows a caller-generated key without permitting redirected output.
    ///
    /// The caller must use an OS-generated key, retain its separate pending ticket,
    /// and close this display by the supplied deadline. This operation performs no
    /// browser connection, account validation, or save. Owned encoding is zeroizing;
    /// terminal and formatter-internal copies have no erasure guarantee.
    ///
    /// # Errors
    ///
    /// Returns a safe terminal, size, key, deadline, or I/O failure. Partial rendering
    /// attempts to clear and leave only the owned alternate screen.
    pub fn open(key: Zeroizing<Vec<u8>>, deadline: Instant) -> Result<Self, PairingDisplayError> {
        require_terminal(
            io::stdin().is_terminal(),
            io::stdout().is_terminal(),
            io::stderr().is_terminal(),
        )?;
        let lease = DisplayLease::claim()?;
        check_deadline(deadline)?;
        let encoded = encode(key)?;
        let mut control = terminal()?;
        let (columns, rows) = size().map_err(PairingDisplayError::io)?;
        if columns < 60 || rows < 10 {
            return Err(PairingDisplayError::TerminalTooSmall);
        }
        execute!(control, EnterAlternateScreen).map_err(PairingDisplayError::io)?;
        // Reopen after switching so legacy Windows consoles write to the new buffer.
        let screen = match terminal() {
            Ok(screen) => screen,
            Err(error) => {
                if execute!(control, LeaveAlternateScreen).is_err() {
                    eprintln!("pairing terminal setup restoration failed");
                }
                return Err(error);
            }
        };
        let mut display = Self {
            screen,
            active: true,
            _lease: lease,
        };
        execute!(display.screen, Clear(ClearType::All), MoveTo(0, 0))
            .map_err(PairingDisplayError::io)?;
        render(&mut display.screen, &encoded).map_err(PairingDisplayError::io)?;
        check_deadline(deadline)?;
        Ok(display)
    }

    /// Clears the owned alternate screen and restores the previous screen once.
    fn restore(&mut self) -> Result<(), PairingDisplayError> {
        if !self.active {
            return Ok(());
        }
        self.active = false;
        let clear = execute!(self.screen, Clear(ClearType::All));
        let leave = execute!(self.screen, LeaveAlternateScreen);
        clear.and(leave).map_err(PairingDisplayError::io)
    }

    /// Consumes the display and reports any cleanup failure instead of claiming restoration.
    ///
    /// # Errors
    ///
    /// Returns a redacted I/O failure if clearing or leaving the alternate screen fails.
    pub fn close(mut self) -> Result<(), PairingDisplayError> {
        self.restore()
    }
}

impl fmt::Debug for PairingDisplay {
    /// Omits handles, state, and all display content from diagnostics.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("PairingDisplay { fields: [REDACTED] }")
    }
}

impl Drop for PairingDisplay {
    /// Reports failed owned-screen cleanup without exposing the underlying I/O cause.
    fn drop(&mut self) {
        if self.restore().is_err() {
            eprintln!("pairing terminal cleanup failed; restoration is not confirmed");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Checks that process-local display ownership cannot be stolen and is released on drop.
    #[test]
    fn display_ownership_is_exclusive_and_released() {
        let first = DisplayLease::claim().expect("first display");
        assert!(matches!(
            DisplayLease::claim(),
            Err(PairingDisplayError::Busy)
        ));
        drop(first);
        let next = DisplayLease::claim().expect("released ownership");
        drop(next);
    }

    /// Checks all redirection combinations before any encoding or terminal output.
    #[test]
    fn every_standard_stream_must_be_interactive() {
        for flags in 0..8 {
            let result = require_terminal(flags & 1 != 0, flags & 2 != 0, flags & 4 != 0);
            assert_eq!(result.is_ok(), flags == 7);
        }
    }

    /// Checks canonical 43-character encoding against the exact invented bytes.
    #[test]
    fn encoding_is_exact_and_rejects_wrong_sizes() {
        let key = encode(Zeroizing::new(vec![42; 32])).expect("synthetic encoding");
        assert_eq!(key.len(), 43);
        assert!(!key.contains('='));
        assert_eq!(URL_SAFE_NO_PAD.decode(&*key).expect("decode"), vec![42; 32]);
        for length in [0, 31, 33] {
            assert!(matches!(
                encode(Zeroizing::new(vec![42; length])),
                Err(PairingDisplayError::InvalidKey)
            ));
        }
    }

    /// Checks readable, bounded plain text and explicit absence of a login-success claim.
    #[test]
    fn layout_keeps_key_whole_and_does_not_claim_login() {
        let key = encode(Zeroizing::new(vec![42; 32])).expect("synthetic key");
        let mut output = Vec::new();
        render(&mut output, &key).expect("synthetic layout");
        let output = String::from_utf8(output).expect("ASCII");
        assert_eq!(output.matches(&*key).count(), 1);
        assert!(
            output
                .lines()
                .all(|line| line.trim_end_matches('\r').len() < 60)
        );
        assert!(output.contains("does not validate or save a session"));
    }

    /// Checks that I/O source and debug chains cannot reflect dependency messages.
    #[test]
    fn terminal_errors_keep_safe_typed_causes() {
        let error = PairingDisplayError::io(io::Error::other("synthetic-secret-marker"));
        assert!(!format!("{error:?}").contains("synthetic-secret-marker"));
        let cause = error.source().expect("safe cause");
        assert!(!cause.to_string().contains("synthetic-secret-marker"));
        assert!(cause.source().is_none());
    }

    /// Checks elapsed and overlong display deadlines without opening a terminal.
    #[test]
    fn display_deadlines_do_not_extend_pending_lifetime() {
        assert!(matches!(
            check_deadline(Instant::now()),
            Err(PairingDisplayError::TimedOut)
        ));
        assert!(check_deadline(Instant::now() + Duration::from_secs(299)).is_ok());
        assert!(matches!(
            check_deadline(Instant::now() + Duration::from_secs(301)),
            Err(PairingDisplayError::InvalidDeadline)
        ));
    }
}
