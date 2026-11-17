//! Synthetic CLI import processes using the real cancellation and save lifecycle.

use std::{
    io::{self, Read, Write},
    process::Command,
    sync::{Arc, mpsc},
    time::{Duration, Instant},
};

use beyond_gate_cli::ReadOperations;
use beyond_gate_core::auth::{
    establishment::{
        EstablishmentError, LoginControl, SavedSession, SessionEstablishmentInput,
        import_fixture::{COOKIE, ImportFixture, Scenario},
    },
    input::SESSION_ENV_VAR,
    session::fixture,
};
use portable_pty::{CommandBuilder, PtySize, native_pty_system};

/// Routes only explicit import through a compiled synthetic establishment owner.
struct ImportOperations(Arc<ImportFixture>);

impl ReadOperations for ImportOperations {
    /// Preserves the adapter's input, control owner, and actual core lifecycle.
    async fn establish_session(
        &self,
        input: SessionEstablishmentInput,
        control: LoginControl,
    ) -> Result<SavedSession, EstablishmentError> {
        self.0.establish(input, control).await
    }
}

/// Runs a child-only fixture, with no fixture controls in product arguments.
pub fn child(mode: &str) -> i32 {
    let scenario = match mode {
        "success" => Scenario::Success,
        "before" => Scenario::BeforeWrite,
        "during" => Scenario::DuringWrite,
        _ => panic!("unknown import fixture"),
    };
    let fixture = Arc::new(ImportFixture::new(scenario));
    let observer = if scenario != Scenario::Success {
        let fixture = fixture.clone();
        Some(std::thread::spawn(move || {
            fixture.wait_ready();
            eprintln!("import-phase-ready");
        }))
    } else {
        None
    };
    let session = fixture::session(fixture::Scenario::Absent).expect("mock session");
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = beyond_gate_cli::run_with_session_input(
        ["beyond-gate", "auth", "import", "--env"],
        &ImportOperations(fixture.clone()),
        &session,
        &mut io::empty(),
        &mut stdout,
        &mut stderr,
    );
    if let Some(observer) = observer {
        observer.join().expect("phase observer");
        assert!(
            stdout.is_empty(),
            "cancelled import must not report success"
        );
        assert_eq!(code, 1);
        assert_eq!(
            String::from_utf8_lossy(&stderr),
            if scenario == Scenario::BeforeWrite {
                "error: cancelled\n"
            } else {
                "error: save_indeterminate\n"
            }
        );
        assert!(fixture.prior_is_unchanged());
        fixture.release();
        fixture.wait_finished();
        if scenario == Scenario::DuringWrite {
            assert!(fixture.imported_cookie_is_saved());
        } else {
            assert!(fixture.prior_is_unchanged());
        }
        eprintln!("import-store-checked");
    } else {
        assert!(fixture.imported_cookie_is_saved());
    }
    io::stdout().write_all(&stdout).expect("result output");
    io::stderr().write_all(&stderr).expect("diagnostic output");
    code
}

/// Checks exact success output, empty diagnostics, and zero process exit.
fn success() {
    let output = Command::new(std::env::current_exe().expect("test path"))
        .args(["--import-child", "success"])
        .env(SESSION_ENV_VAR, COOKIE)
        .output()
        .expect("import process");
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(
        output.stdout,
        b"saved: Beyond Gate session; validation=service_validated; expiry=unknown.\n"
    );
    assert!(output.stderr.is_empty(), "{output:?}");
}

/// Sends a real terminal Ctrl+C only after the selected write phase is reached.
fn cancellation(mode: &str, expected: &[u8]) {
    let pair = native_pty_system()
        .openpty(PtySize {
            rows: 24,
            cols: 120,
            pixel_width: 0,
            pixel_height: 0,
        })
        .expect("import terminal");
    let mut command = CommandBuilder::new(std::env::current_exe().expect("test path"));
    command.args(["--import-child", mode]);
    command.env(SESSION_ENV_VAR, COOKIE);
    let mut child = pair.slave.spawn_command(command).expect("import child");
    drop(pair.slave);
    let mut writer = pair.master.take_writer().expect("terminal writer");
    let mut reader = pair.master.try_clone_reader().expect("terminal reader");
    let (send, receive) = mpsc::channel();
    let reader = std::thread::spawn(move || {
        let mut bytes = [0; 4096];
        while let Ok(count) = reader.read(&mut bytes) {
            if count == 0 || send.send(bytes[..count].to_vec()).is_err() {
                break;
            }
        }
    });
    let deadline = Instant::now() + Duration::from_secs(20);
    let mut output = Vec::new();
    let mut cancelled = false;
    loop {
        if output
            .windows(18)
            .any(|bytes| bytes == b"import-phase-ready")
            && !cancelled
        {
            writer.write_all(b"\x03").expect("Ctrl+C");
            writer.flush().expect("flush Ctrl+C");
            cancelled = true;
        }
        match receive.recv_timeout(Duration::from_millis(100)) {
            Ok(bytes) => {
                let start = output.len().saturating_sub(3);
                output.extend(bytes);
                for _ in output[start..]
                    .windows(4)
                    .filter(|bytes| *bytes == b"\x1b[6n")
                {
                    writer.write_all(b"\x1b[1;1R").expect("cursor report");
                    writer.flush().expect("cursor flush");
                }
                assert!(output.len() < 65_536, "bounded fixture output");
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
        if child.try_wait().expect("observe child exit").is_some() {
            while let Ok(bytes) = receive.try_recv() {
                output.extend(bytes);
            }
            break;
        }
        if Instant::now() >= deadline {
            child.kill().expect("stop owned child");
            panic!(
                "import child exceeded deadline: {}",
                String::from_utf8_lossy(&output)
            );
        }
    }
    let status = child.wait().expect("import exit");
    drop(writer);
    drop(pair.master);
    drop(receive);
    reader.join().expect("terminal reader");
    assert_eq!(
        status.exit_code(),
        1,
        "{}",
        String::from_utf8_lossy(&output)
    );
    assert!(cancelled);
    assert!(
        output
            .windows(expected.len())
            .any(|bytes| bytes == expected)
    );
    assert!(
        output
            .windows(20)
            .any(|bytes| bytes == b"import-store-checked")
    );
    assert!(!output.windows(6).any(|bytes| bytes == b"saved:"));
    assert!(
        !output
            .windows(COOKIE.len())
            .any(|bytes| bytes == COOKIE.as_bytes())
    );
}

/// Covers success and both sides of actual write admission through CLI processes.
pub fn check() {
    success();
    cancellation("before", b"error: cancelled");
    cancellation("during", b"error: save_indeterminate");
}
