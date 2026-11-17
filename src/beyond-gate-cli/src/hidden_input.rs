//! Explicit no-echo terminal input, isolated from result and protocol streams.

use std::{
    io::{self, IsTerminal, Write},
    time::{Duration, Instant},
};

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    terminal,
};
use zeroize::Zeroizing;

use crate::error::Failure;

/// Restores terminal processing if an input operation returns unexpectedly.
struct RawMode(bool);

impl Drop for RawMode {
    /// Performs best-effort unwinding cleanup; normal exits report cleanup errors.
    fn drop(&mut self) {
        if self.0 {
            let _ = terminal::disable_raw_mode();
        }
    }
}

/// Requests one cookie without echo, accepting redirected stdout but not input.
pub(crate) fn read(stderr: &mut impl Write) -> Result<Vec<u8>, Failure> {
    if !io::stdin().is_terminal() || !io::stderr().is_terminal() {
        return Err(Failure::Usage(
            "Hidden input requires terminal stdin and stderr; redirected stdout is allowed.",
        ));
    }
    terminal::enable_raw_mode().map_err(Failure::HiddenInput)?;
    let mut guard = RawMode(true);
    let result = stderr
        .write_all(b"CobaltSession (hidden; Enter submits, Esc or Ctrl+C cancels): ")
        .and_then(|()| stderr.flush())
        .map_err(Failure::Output)
        .and_then(|()| read_events());
    let restored = terminal::disable_raw_mode().map_err(Failure::HiddenInput);
    guard.0 = restored.is_err();
    restored?;
    stderr.write_all(b"\n").map_err(Failure::Output)?;
    result.map(|mut bytes| std::mem::take(&mut *bytes))
}

/// Reads bounded input on one thread and treats control keys as local actions.
fn read_events() -> Result<Zeroizing<Vec<u8>>, Failure> {
    let deadline = Instant::now() + Duration::from_secs(300);
    let mut bytes = Zeroizing::new(Vec::with_capacity(2_552));
    // Keep an invalid paste hidden until submission instead of releasing an echoing terminal.
    let mut invalid = None;
    loop {
        if Instant::now() >= deadline {
            return Err(Failure::Usage("Hidden input timed out."));
        }
        if !event::poll(Duration::from_millis(100)).map_err(Failure::HiddenInput)? {
            continue;
        }
        if let Event::Key(key) = event::read().map_err(Failure::HiddenInput)? {
            if key.kind == KeyEventKind::Release {
                continue;
            }
            match key.code {
                KeyCode::Esc => return Err(Failure::Cancelled),
                KeyCode::Char('c' | 'C') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    return Err(Failure::Cancelled);
                }
                KeyCode::Enter => {
                    return match invalid {
                        Some(message) => Err(Failure::Usage(message)),
                        None => Ok(bytes),
                    };
                }
                KeyCode::Backspace => {
                    if let Some(last) = bytes.last_mut() {
                        *last = 0;
                        bytes.pop();
                    }
                }
                KeyCode::Char(character)
                    if character.is_ascii()
                        && !key.modifiers.intersects(
                            KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SUPER,
                        ) =>
                {
                    if invalid.is_some() {
                        continue;
                    }
                    if bytes.len() == 2_552 {
                        invalid = Some("Hidden cookie input exceeds 2,552 bytes.");
                    } else {
                        bytes.push(character as u8);
                    }
                }
                KeyCode::Char(_) => {
                    invalid = Some("Hidden cookie input must be ASCII text.");
                }
                KeyCode::Tab | KeyCode::BackTab => {
                    invalid = Some("Hidden cookie input must not contain tabs.");
                }
                _ => {}
            }
        }
    }
}
