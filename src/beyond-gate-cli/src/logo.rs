//! Embedded Unicode Braille artwork converted from the Beyond Gate logo.

use std::io::Write;

use clap::builder::styling::AnsiColor;

use crate::error::Failure;

/// Generated with ascii-image-converter 1.13.1: --braille --dimensions 80,18 --threshold 50.
const LOGO: &str = include_str!("../../../assets/beyond-gate-logo-braille.txt");

/// The first eight image rows contain BEYOND; the remaining rows contain GATE.
const BEYOND_ROWS: usize = 8;

/// Writes styled lettering; the native stderr adapter selects color or plain output.
///
/// # Errors
///
/// Preserves a writer failure as the CLI's typed output error.
pub(crate) fn write(writer: &mut impl Write) -> Result<(), Failure> {
    let white = AnsiColor::White.on_default();
    let red = AnsiColor::Red.on_default();
    write!(writer, "{white}").map_err(Failure::Output)?;
    for (index, line) in LOGO.lines().enumerate() {
        if index == BEYOND_ROWS {
            write!(writer, "{white:#}\n{red}").map_err(Failure::Output)?;
        }
        writeln!(writer, "{line}").map_err(Failure::Output)?;
    }
    writeln!(writer, "{red:#}").map_err(Failure::Output)
}

#[cfg(test)]
mod tests {
    use std::io;

    use super::*;

    /// Keeps the uncolored image intact and within an 80-column terminal.
    #[test]
    fn plain_logo_fits_terminal_and_preserves_both_words() {
        let mut output = anstream::AutoStream::never(Vec::new());
        write(&mut output).expect("plain logo");
        let bytes = output.into_inner();
        let text = String::from_utf8(bytes).expect("UTF-8 logo");
        assert_eq!(
            text.lines()
                .filter(|line| !line.is_empty())
                .collect::<Vec<_>>(),
            LOGO.lines().collect::<Vec<_>>()
        );
        assert!(
            text.chars().all(
                |character| character == '\n' || ('\u{2800}'..='\u{28ff}').contains(&character)
            )
        );
        assert!(!text.contains('\x1b'));
        assert!(text.lines().all(|line| line.chars().count() <= 80));
    }

    /// Retains the concrete writer cause rather than reporting a successful banner.
    #[test]
    fn logo_write_failure_preserves_io_cause() {
        struct FailedWriter;

        impl Write for FailedWriter {
            /// Simulates a closed output pipe before the first logo byte.
            fn write(&mut self, _bytes: &[u8]) -> io::Result<usize> {
                Err(io::ErrorKind::BrokenPipe.into())
            }

            /// Permits flush so the test isolates the write failure.
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }

        assert!(matches!(
            write(&mut FailedWriter),
            Err(Failure::Output(error)) if error.kind() == io::ErrorKind::BrokenPipe
        ));
    }
}
