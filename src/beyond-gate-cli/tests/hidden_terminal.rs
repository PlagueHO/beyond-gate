//! Synthetic PTY/ConPTY checks for hidden input and terminal restoration.

use std::{
    io::{self, Read, Write},
    process::{Command, Stdio},
    sync::mpsc,
    time::{Duration, Instant},
};

use beyond_gate_cli::ReadOperations;
use beyond_gate_core::auth::{
    establishment::{
        EstablishmentError, LoginControl, SavedSession, SessionEstablishmentInput,
        import_fixture::{COOKIE, ImportFixture, Scenario as ImportScenario},
    },
    session::fixture::{self, Scenario},
};
use portable_pty::{CommandBuilder, PtySize, native_pty_system};

/// Routes prompt imports through the real lifecycle with a synthetic store.
struct ImportOperations(ImportFixture);

impl ReadOperations for ImportOperations {
    /// Validates and saves only the fixture cookie through local mock services.
    async fn establish_session(
        &self,
        input: SessionEstablishmentInput,
        control: LoginControl,
    ) -> Result<SavedSession, EstablishmentError> {
        self.0.establish(input, control).await
    }
}

/// Runs the actual CLI input path with local mock exchange and no native store.
fn child() -> i32 {
    let session = fixture::session(Scenario::Locked)
        .expect("mock session")
        .with_environment_value(Some("".into()));
    let code = beyond_gate_cli::run_with_session_input(
        ["beyond-gate", "--auth-prompt", "auth", "validate"],
        &beyond_gate_cli::LiveReads,
        &session,
        &mut io::empty(),
        &mut io::stdout().lock(),
        &mut io::stderr().lock(),
    );
    assert_eq!(fixture::request_count(&session), usize::from(code == 0));
    finish_child(code)
}

/// Checks that supplied-input login and import save only valid input to the mock entry.
fn save_child(login: bool) -> i32 {
    let session = fixture::session(Scenario::Locked)
        .expect("mock session")
        .with_environment_value(Some("".into()));
    let operations = ImportOperations(ImportFixture::new(ImportScenario::Success));
    let arguments = if login {
        ["beyond-gate", "auth", "login", "--prompt"]
    } else {
        ["beyond-gate", "auth", "import", "--prompt"]
    };
    let code = beyond_gate_cli::run_with_session_input(
        arguments,
        &operations,
        &session,
        &mut io::empty(),
        &mut io::stdout().lock(),
        &mut io::stderr().lock(),
    );
    assert_eq!(operations.0.imported_cookie_is_saved(), code == 0);
    assert_eq!(operations.0.prior_is_unchanged(), code != 0);
    assert_eq!(fixture::request_count(&session), 0);
    finish_child(code)
}

/// Confirms normal terminal input resumes after either hidden-input command.
fn finish_child(code: i32) -> i32 {
    eprintln!("input-complete:{code}");
    eprintln!("restoration-check:");
    let mut line = String::new();
    io::stdin().read_line(&mut line).expect("restored input");
    assert_eq!(line.trim(), "visible-after-input");
    eprintln!("restoration-complete");
    code
}

/// Redirects only result stdout while retaining the PTY for stdin and stderr.
fn redirected_child() -> i32 {
    let path = std::env::temp_dir().join(format!("beyond-gate-hidden-{}.json", std::process::id()));
    let output = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .expect("owned output file");
    let status = Command::new(std::env::current_exe().expect("test path"))
        .arg("--child")
        .stdout(Stdio::from(output))
        .status()
        .expect("redirected child");
    let bytes = std::fs::read(&path).expect("read output");
    std::fs::remove_file(&path).expect("remove owned output");
    let value: serde_json::Value = serde_json::from_slice(&bytes).expect("result JSON");
    assert_eq!(value["validation"], "service_validated");
    status.code().expect("child exit")
}

/// Answers ConPTY cursor requests while waiting for a fixed, unprinted marker.
fn until(
    receive: &mpsc::Receiver<Vec<u8>>,
    output: &mut Vec<u8>,
    writer: &mut impl Write,
    marker: &[u8],
) -> Result<(), &'static str> {
    let deadline = Instant::now() + Duration::from_secs(15);
    while !output.windows(marker.len()).any(|bytes| bytes == marker) {
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .ok_or("terminal deadline")?;
        let bytes = receive
            .recv_timeout(remaining)
            .map_err(|_| "terminal response missing")?;
        let query_start = output.len().saturating_sub(3);
        output.extend(bytes);
        if output.len() > 64 * 1024 {
            return Err("terminal output exceeded its bound");
        }
        // portable-pty enables PSEUDOCONSOLE_INHERIT_CURSOR; act as its terminal host.
        for _ in output[query_start..]
            .windows(4)
            .filter(|bytes| *bytes == b"\x1b[6n")
        {
            writer
                .write_all(b"\x1b[1;1R")
                .and_then(|()| writer.flush())
                .map_err(|_| "cursor response failed")?;
        }
    }
    Ok(())
}

/// Checks split cursor requests, repeated requests, and replies across marker waits.
fn check_cursor_reports() {
    for split in 0..=4 {
        let (send, receive) = mpsc::channel();
        let mut output = Vec::new();
        let mut replies = Vec::new();
        let query = b"\x1b[6n";
        send.send(query[..split].to_vec()).expect("query prefix");
        send.send(query[split..].to_vec()).expect("query suffix");
        send.send(b"first-marker".to_vec()).expect("first marker");
        until(&receive, &mut output, &mut replies, b"first-marker").expect("first cursor response");
        assert_eq!(replies, b"\x1b[1;1R");
        send.send(b"\x1b[?25h".to_vec())
            .expect("unrelated sequence");
        send.send(query.to_vec()).expect("repeated query");
        send.send(b"second-marker".to_vec()).expect("second marker");
        until(&receive, &mut output, &mut replies, b"second-marker")
            .expect("second cursor response");
        assert_eq!(replies, b"\x1b[1;1R\x1b[1;1R");
    }
}

/// Checks no echo, editing/cancellation, restoration, and bounded process exit.
fn check(case: &str, mode: &str, input: &[u8], expected: i32) {
    check_chunks(case, mode, &[input], expected);
}

/// Separates paste chunks so a newline cannot submit before the tail arrives.
fn check_chunks(case: &str, mode: &str, chunks: &[&[u8]], expected: i32) {
    let pair = native_pty_system()
        .openpty(PtySize {
            rows: 24,
            cols: 120,
            pixel_width: 0,
            pixel_height: 0,
        })
        .expect("synthetic terminal");
    let mut command = CommandBuilder::new(std::env::current_exe().expect("test path"));
    command.arg(mode);
    command.env("BEYOND_GATE_COBALT_SESSION", "");
    let mut child = pair.slave.spawn_command(command).expect("terminal child");
    drop(pair.slave);
    let mut reader = pair.master.try_clone_reader().expect("terminal reader");
    let mut writer = pair.master.take_writer().expect("terminal writer");
    let (send, receive) = mpsc::channel();
    let reader_thread = std::thread::spawn(move || {
        let mut bytes = [0; 1024];
        loop {
            match reader.read(&mut bytes) {
                Ok(0) | Err(_) => break,
                Ok(count) => {
                    if send.send(bytes[..count].to_vec()).is_err() {
                        break;
                    }
                }
            }
        }
    });
    let mut output = Vec::new();
    let result = (|| {
        until(
            &receive,
            &mut output,
            &mut writer,
            b"Esc or Ctrl+C cancels): ",
        )
        .map_err(|_| "hidden prompt missing")?;
        for (index, input) in chunks.iter().enumerate() {
            writer
                .write_all(input)
                .map_err(|_| "terminal input failed")?;
            writer.flush().map_err(|_| "terminal flush failed")?;
            if index + 1 < chunks.len() {
                std::thread::sleep(Duration::from_millis(150));
            }
        }
        until(&receive, &mut output, &mut writer, b"restoration-check:")
            .map_err(|_| "input did not finish")?;
        if !output
            .windows(format!("input-complete:{expected}").len())
            .any(|bytes| bytes == format!("input-complete:{expected}").as_bytes())
        {
            return Err("unexpected input outcome");
        }
        if [b"invented-terminal".as_slice(), COOKIE.as_bytes()]
            .iter()
            .any(|secret| output.windows(secret.len()).any(|bytes| bytes == *secret))
        {
            return Err("hidden input was echoed");
        }
        writer
            .write_all(b"visible-after-input\r")
            .map_err(|_| "restored input failed")?;
        writer.flush().map_err(|_| "restored flush failed")?;
        until(&receive, &mut output, &mut writer, b"restoration-complete")?;
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if let Some(status) = child.try_wait().map_err(|_| "child wait failed")? {
                if status.exit_code() != expected as u32 {
                    return Err("terminal child returned a different exit status");
                }
                break;
            }
            if Instant::now() >= deadline {
                return Err("child exit deadline");
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        Ok(())
    })();
    if result.is_err() {
        child.kill().expect("stop this owned child");
        child.wait().expect("reap owned child");
    }
    drop(writer);
    drop(pair.master);
    drop(receive);
    reader_thread.join().expect("terminal reader");
    if let Err(message) = result {
        panic!("{case}: {message}");
    }
}

/// Rejects delayed multiline input without echo or a first-line exchange.
fn check_pastes() {
    check(
        "Ctrl+V followed by terminal paste",
        "--child",
        b"\x16invented-terminal-cookie\x04",
        0,
    );
    check("Ctrl+V without text", "--child", b"\x16\x04", 1);
    check(
        "Ctrl+V preserves cancellation",
        "--child",
        b"\x16invented-terminal-cookie\x03",
        1,
    );
    check_chunks(
        "Ctrl+V cannot clear an invalid paste",
        "--child",
        &[
            b"invented-terminal-first\r",
            b"\x16invented-terminal-tail\x04",
        ],
        2,
    );
    check_chunks(
        "split single-line paste",
        "--child",
        &[b"invented-terminal-", b"cookie", b"\x04"],
        0,
    );
    check_chunks(
        "split multiline CR paste",
        "--child",
        &[
            b"invented-terminal-first\r",
            b"invented-terminal-tail",
            b"\x04",
        ],
        2,
    );
    check_chunks(
        "split multiline LF paste",
        "--child",
        &[
            b"invented-terminal-first\n",
            b"invented-terminal-tail",
            b"\x04",
        ],
        2,
    );
    check_chunks(
        "split multiline CRLF paste",
        "--child",
        &[
            b"invented-terminal-first\r",
            b"\ninvented-terminal-tail",
            b"\x04",
        ],
        2,
    );
    let mut paste = b"invented-terminal-".to_vec();
    paste.resize(8_192, b'x');
    check_chunks(
        "split oversized paste",
        "--child",
        &[&paste, b"invented-terminal-tail", b"\x04"],
        2,
    );
    for child in ["--login-child", "--import-child"] {
        let mut paste = b"\x16".to_vec();
        paste.extend_from_slice(COOKIE.as_bytes());
        paste.push(4);
        check("Ctrl+V followed by prompt save paste", child, &paste, 0);
        check_chunks("prompt save", child, &[COOKIE.as_bytes(), b"\x04"], 0);
        check_chunks(
            "split multiline prompt save",
            child,
            &[COOKIE.as_bytes(), b"\r", b"invented-terminal-tail", b"\x04"],
            2,
        );
        check(
            "cancel prompt save",
            child,
            b"invented-terminal-cancel\x03",
            1,
        );
        check(
            "escape prompt save",
            child,
            b"invented-terminal-cancel\x1b",
            1,
        );
        check(
            "rejected prompt save",
            child,
            b"invented-terminal-rejected\x04",
            1,
        );
    }
}

/// Selects synthetic children before running the bounded terminal matrix.
fn main() {
    match std::env::args().nth(1).as_deref() {
        Some("--child") => std::process::exit(child()),
        Some("--import-child") => std::process::exit(save_child(false)),
        Some("--login-child") => std::process::exit(save_child(true)),
        Some("--redirected-child") => std::process::exit(redirected_child()),
        Some("--paste-check") => check_pastes(),
        _ => {
            check_cursor_reports();
            let editing: &[u8] = if cfg!(windows) {
                b"invented-terminal \x08-cookie\x04"
            } else {
                b"invented-terminal \x7f-cookie\x04"
            };
            check("editing", "--child", editing, 0);
            check("Ctrl+C", "--child", b"invented-terminal-cancel\x03", 1);
            check("Escape", "--child", b"invented-terminal-cancel\x1b", 1);
            check(
                "non-ASCII",
                "--child",
                b"invented-terminal-invalid\xc3\xa9-tail\x04",
                2,
            );
            check("tab", "--child", b"invented-terminal-invalid\t-tail\x04", 2);
            let mut maximum = b"invented-terminal-".to_vec();
            maximum.resize(2_552, b'x');
            maximum.push(4);
            check("maximum length", "--child", &maximum, 0);
            let mut oversized = b"invented-terminal-".to_vec();
            oversized.extend(std::iter::repeat_n(b'x', 2_553));
            oversized.extend_from_slice(b"invented-terminal-tail");
            oversized.push(4);
            check("oversized", "--child", &oversized, 2);
            check(
                "redirected stdout",
                "--redirected-child",
                b"invented-terminal-cookie\x04",
                0,
            );
            check_pastes();
        }
    }
}
