//! Explicit feature-built native-process checks; missing fixture infrastructure fails.
//!
//! Build the example before running with `browser-fixture`. Normal builds expose
//! neither the helper nor these development-only process scenarios.
#![cfg(feature = "browser-fixture")]

use std::{
    io::{BufRead, BufReader, Read},
    path::PathBuf,
    process::{Child, Command, ExitStatus, Stdio},
    sync::mpsc::{self, Receiver},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use serde_json::Value;

#[cfg(windows)]
use std::io::Write;

#[cfg(windows)]
mod support;

/// Owns a newly created fixture directory with only fixed, synthetic test files.
#[cfg(windows)]
struct NativeFixtureDirectory(PathBuf);

#[cfg(windows)]
impl Drop for NativeFixtureDirectory {
    /// Removes only known test files and then the empty directory, never recursively.
    fn drop(&mut self) {
        for name in [
            "browser-handoff-fixture.exe",
            "control.json",
            "fixture.json",
            "host-report.json",
        ] {
            let path = self.0.join(name);
            match path.try_exists() {
                Ok(false) => {}
                Ok(true) => {
                    if std::fs::remove_file(path).is_err() {
                        eprintln!("owned native fixture file cleanup failed");
                    }
                }
                Err(_) => eprintln!("owned native fixture file state could not be checked"),
            }
        }
        if std::fs::remove_dir(&self.0).is_err() {
            eprintln!("owned native fixture directory cleanup failed");
        }
    }
}

/// Owns exactly the child created by this test, never a process selected by name or file PID.
struct OwnedChild {
    child: Child,
}

impl OwnedChild {
    /// Reads only bounded fixed fixture diagnostics after the owned process exits.
    fn error(&mut self) -> String {
        let stderr = self.child.stderr.take().expect("owned diagnostic pipe");
        let mut text = String::new();
        stderr
            .take(4096)
            .read_to_string(&mut text)
            .expect("fixed diagnostics");
        text
    }

    /// Waits within a fixed bound; the drop guard stops this owned child on assertion failure.
    fn wait(&mut self) -> ExitStatus {
        let deadline = Instant::now() + Duration::from_secs(12);
        loop {
            if let Some(status) = self.child.try_wait().expect("inspect owned child") {
                return status;
            }
            assert!(
                Instant::now() < deadline,
                "owned fixture process exceeded its lifetime"
            );
            thread::sleep(Duration::from_millis(10));
        }
    }
}

impl Drop for OwnedChild {
    /// Cleans up only the spawned child when a test fails before its normal exit.
    fn drop(&mut self) {
        match self.child.try_wait() {
            Ok(Some(_)) => {}
            Ok(None) => {
                if self.child.kill().is_err() {
                    eprintln!("owned paired fixture process cleanup failed");
                } else if self.child.wait().is_err() {
                    eprintln!("owned paired fixture process exit was not confirmed");
                }
            }
            Err(_) => eprintln!("owned paired fixture process state could not be checked"),
        }
    }
}

/// Locates only this build's sibling example; never searches PATH or another checkout.
fn binary() -> PathBuf {
    example_binary("paired-handoff-fixture")
}

/// Resolves a feature-built helper in this test build, never PATH or another checkout.
fn example_binary(name: &str) -> PathBuf {
    let path = std::env::current_exe().expect("test executable");
    let directory = path
        .parent()
        .and_then(|path| path.parent())
        .expect("target directory");
    let path = directory
        .join("examples")
        .join(format!("{name}{}", std::env::consts::EXE_SUFFIX));
    assert!(
        path.is_file(),
        "required fixture is missing; build --example {name} --features browser-fixture first"
    );
    path
}

/// Frames only invented messages using Chromium's native-endian length prefix.
#[cfg(windows)]
fn browser_frame(message: &Value) -> Vec<u8> {
    let payload = serde_json::to_vec(message).expect("invented message");
    let mut frame = u32::try_from(payload.len())
        .expect("bounded test frame")
        .to_ne_bytes()
        .to_vec();
    frame.extend_from_slice(&payload);
    frame
}

/// Checks real profile rejection and preserves synthetic receipt without saved success.
#[cfg(windows)]
#[test]
fn synthetic_native_host_enforces_profile_without_changing_synthetic_receipt() {
    for (production_open, production_submit) in [(true, false), (false, true), (false, false)] {
        let rejected = production_open || production_submit;
        let binary = example_binary("browser-handoff-fixture");
        let transaction =
            beyond_gate_core::auth::handoff::Transaction::generate().expect("test namespace");
        let path = binary
            .parent()
            .expect("example directory")
            .join(format!("native-profile-test-{}", transaction.as_str()));
        std::fs::create_dir(&path).expect("create new owned directory");
        let directory = NativeFixtureDirectory(path);
        let executable = directory.0.join("browser-handoff-fixture.exe");
        std::fs::copy(binary, &executable).expect("copy known fixture");
        let origin = "chrome-extension://aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa/";
        let control = serde_json::json!({
            "context": "beyond-gate-synthetic-v1",
            "browser": "edge",
            "origin": origin,
            "mode": "receive"
        });
        std::fs::write(
            directory.0.join("control.json"),
            serde_json::to_vec(&control).expect("synthetic control"),
        )
        .expect("write synthetic control");
        let fixture = support::fixture();
        std::fs::write(
            directory.0.join("fixture.json"),
            serde_json::to_vec(&fixture).expect("synthetic fixture"),
        )
        .expect("write synthetic fixture");
        let mut hello = fixture["messages"]["browser_hello"].clone();
        let mut submit = fixture["messages"]["browser_submit"].clone();
        if production_open {
            hello["profile"] = "cobalt-session-v1".into();
        } else if production_submit {
            submit["profile"] = "cobalt-session-v1".into();
        }
        let mut child = OwnedChild {
            child: Command::new(executable)
                .arg(origin)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("start owned native fixture"),
        };
        let mut input = browser_frame(&hello);
        input.extend_from_slice(&browser_frame(&submit));
        {
            let mut stdin = child.child.stdin.take().expect("owned input pipe");
            stdin.write_all(&input).expect("send invented frames");
        }
        let status = child.wait();
        let error = child.error();
        let mut stdout = Vec::new();
        child
            .child
            .stdout
            .take()
            .expect("owned output pipe")
            .take(16 * 1024 + 5)
            .read_to_end(&mut stdout)
            .expect("bounded fixture output");
        let report: Value = serde_json::from_slice(
            &std::fs::read(directory.0.join("host-report.json")).expect("report"),
        )
        .expect("fixed report");
        drop(child);
        assert_eq!(status.success(), !rejected);
        assert_eq!(
            error.trim(),
            if rejected {
                "synthetic host message is unexpected"
            } else {
                ""
            }
        );
        assert_eq!(report["hello_received"], !production_open);
        assert_eq!(report["submit_received"], !rejected);
        assert_eq!(report["validation_performed"], false);
        assert_eq!(report["storage_used"], false);
        if production_open {
            assert!(stdout.is_empty(), "no authorization for production open");
        } else if production_submit {
            assert_eq!(
                stdout,
                browser_frame(&fixture["messages"]["browser_pending"]),
                "only the synthetic pending frame was sent"
            );
        } else {
            let mut expected = browser_frame(&fixture["messages"]["browser_pending"]);
            let mut error = fixture["messages"]["browser_error"].clone();
            error["transaction"] = fixture["messages"]["browser_pending"]["transaction"].clone();
            error["code"] = "unsupported_contract".into();
            expected.extend_from_slice(&browser_frame(&error));
            assert_eq!(stdout, expected, "synthetic receipt is not saved success");
        }
    }
}

/// Reads bounded fixed metadata lines, never a secret or handoff payload.
fn reports(child: &mut OwnedChild) -> (Receiver<String>, JoinHandle<()>) {
    let stdout = child.child.stdout.take().expect("owned metadata pipe");
    let (send, receive) = mpsc::channel();
    let worker = thread::spawn(move || {
        let reader = BufReader::new(stdout).take(4096);
        for line in reader.lines() {
            let line = line.expect("read fixed fixture metadata");
            if send.send(line).is_err() {
                return;
            }
        }
    });
    (receive, worker)
}

/// Starts one known helper role with no secret argument, environment input, or stdin.
fn spawn(arguments: &[&str]) -> OwnedChild {
    OwnedChild {
        child: Command::new(binary())
            .args(arguments)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("start owned synthetic fixture"),
    }
}

/// Runs two actual Rust processes and confirms receipt or proof rejection without a save claim.
fn scenario(mode: &str) {
    scenario_transport(mode, false)
}

/// Runs identical application assertions on the selected real local transport.
fn scenario_transport(mode: &str, pipe: bool) {
    let namespace =
        beyond_gate_core::auth::handoff::Transaction::generate().expect("public namespace");
    let mut cli = if pipe {
        spawn(&["listen-pipe", namespace.as_str()])
    } else {
        spawn(&["listen"])
    };
    let (cli_reports, cli_reader) = reports(&mut cli);
    let line = cli_reports
        .recv_timeout(Duration::from_secs(5))
        .expect("listener metadata");
    let listening: Value = serde_json::from_str(&line).expect("fixed listener metadata");
    let mut host = if pipe {
        assert_eq!(listening["fixture"], "bg-paired-pipe-v1");
        assert_eq!(listening["listening"], true);
        spawn(&["host-pipe", namespace.as_str(), mode])
    } else {
        assert_eq!(listening["fixture"], "bg-paired-process-v1");
        let port = listening["port"].as_u64().expect("public local port");
        assert!((1..=65535).contains(&port));
        spawn(&["host", &port.to_string(), mode])
    };
    let (host_reports, host_reader) = reports(&mut host);
    let host_status = host.wait();
    let cli_status = cli.wait();
    let host_error = host.error();
    let cli_error = cli.error();
    host_reader.join().expect("host metadata reader");
    cli_reader.join().expect("CLI metadata reader");
    if mode == "good" {
        assert!(host_status.success());
        assert!(cli_status.success());
        assert!(host_error.is_empty());
        assert!(cli_error.is_empty());
        for report in [&host_reports, &cli_reports] {
            let line = report
                .recv_timeout(Duration::from_secs(1))
                .expect("receipt metadata");
            let value: Value = serde_json::from_str(&line).expect("receipt JSON");
            assert_eq!(value["received"], true);
            assert_eq!(value["validated"], false);
            assert_eq!(value["saved"], false);
            assert!(report.try_recv().is_err(), "no extra output");
        }
    } else {
        let expected = match mode {
            "wrong-key" => "unauthorized_peer",
            "wrong-transaction" => "transaction_mismatch",
            _ => panic!("unknown rejection scenario"),
        };
        assert!(!host_status.success());
        assert!(!cli_status.success());
        assert!(
            cli_error.contains(expected),
            "expected fixed handoff rejection"
        );
        assert!(host_reports.try_recv().is_err(), "no false receipt");
        assert!(
            cli_reports.try_recv().is_err(),
            "no false authorization result"
        );
    }
}

/// Checks a genuine encrypted open/authorization/submit/error exchange in separate processes.
#[test]
fn synthetic_process_handoff_receives_without_login_success() {
    scenario("good");
}

/// Checks that separate processes reject a wrong PSK before application authorization.
#[test]
fn synthetic_process_handoff_rejects_wrong_pairing_key() {
    scenario("wrong-key");
}

/// Checks that an authenticated peer cannot submit to a different pending transaction.
#[test]
fn synthetic_process_handoff_rejects_wrong_transaction() {
    scenario("wrong-transaction");
}

/// Checks a protected single-instance pipe delivers a paired, bound synthetic receipt.
#[cfg(windows)]
#[test]
fn windows_pipe_process_handoff_receives_without_login_success() {
    scenario_transport("good", true);
}

/// Rejects a wrong key over the real Windows pipe before application authorization.
#[cfg(windows)]
#[test]
fn windows_pipe_process_handoff_rejects_wrong_pairing_key() {
    scenario_transport("wrong-key", true);
}

/// Rejects an authenticated submission for a different pending transaction.
#[cfg(windows)]
#[test]
fn windows_pipe_process_handoff_rejects_wrong_transaction() {
    scenario_transport("wrong-transaction", true);
}

#[cfg(windows)]
mod pending_slot {
    //! Separate Windows processes test reservation ownership and kernel cleanup.

    use super::*;
    use beyond_gate_core::auth::handoff::Transaction;

    /// Starts a known reservation helper with only a role and public correlation suffix.
    fn spawn_slot(role: &str, namespace: &Transaction) -> OwnedChild {
        OwnedChild {
            child: Command::new(example_binary("pending-slot-fixture"))
                .args([role, namespace.as_str()])
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("start owned reservation fixture"),
        }
    }

    /// Requires a bounded readiness record with no validation or protected-save claim.
    fn ready(receive: &Receiver<String>) {
        let line = receive
            .recv_timeout(Duration::from_secs(5))
            .expect("reservation readiness");
        let value: Value = serde_json::from_str(&line).expect("fixed reservation metadata");
        assert_eq!(
            value,
            serde_json::json!({
                "fixture": "bg-pending-slot-v1",
                "claimed": true,
                "validated": false,
                "saved": false,
            })
        );
    }

    /// Checks a denied second process emits no successful acquisition metadata.
    fn denied(namespace: &Transaction) {
        let mut child = spawn_slot("probe", namespace);
        let (receive, reader) = reports(&mut child);
        assert!(!child.wait().success());
        assert_eq!(
            child.error(),
            "pending slot fixture reservation is unavailable\n"
        );
        reader.join().expect("reservation metadata reader");
        assert!(receive.try_recv().is_err(), "no false reservation claim");
    }

    /// Requires a new actual process to claim and release a now-available namespace.
    fn recovered(namespace: &Transaction) {
        let mut child = spawn_slot("probe", namespace);
        let (receive, reader) = reports(&mut child);
        ready(&receive);
        assert!(child.wait().success());
        assert!(child.error().is_empty());
        reader.join().expect("reservation metadata reader");
        assert!(receive.try_recv().is_err(), "no extra metadata");
    }

    /// Checks exactly one of two concurrently started CLI fixtures can own the slot.
    #[test]
    fn pending_slot_two_process_race_has_one_owner() {
        let namespace = Transaction::generate().expect("public fixture randomness");
        let mut first = spawn_slot("hold", &namespace);
        let mut second = spawn_slot("hold", &namespace);
        let (first_reports, first_reader) = reports(&mut first);
        let (second_reports, second_reader) = reports(&mut second);
        let first_success = first.wait().success();
        let second_success = second.wait().success();
        assert_ne!(
            first_success, second_success,
            "exactly one reservation owner"
        );
        first_reader.join().expect("first metadata reader");
        second_reader.join().expect("second metadata reader");
        for (success, child, receive) in [
            (first_success, &mut first, &first_reports),
            (second_success, &mut second, &second_reports),
        ] {
            if success {
                ready(receive);
                assert!(child.error().is_empty());
            } else {
                assert_eq!(
                    child.error(),
                    "pending slot fixture reservation is unavailable\n"
                );
            }
            assert!(receive.try_recv().is_err(), "no extra or false metadata");
        }
        recovered(&namespace);
    }

    /// Checks normal process exit releases the reservation after rejecting a live contender.
    #[test]
    fn pending_slot_normal_process_exit_releases_ownership() {
        let namespace = Transaction::generate().expect("public fixture randomness");
        let mut holder = spawn_slot("hold", &namespace);
        let (receive, reader) = reports(&mut holder);
        ready(&receive);
        denied(&namespace);
        assert!(holder.wait().success());
        assert!(holder.error().is_empty());
        reader.join().expect("holder metadata reader");
        recovered(&namespace);
    }

    /// Checks kernel cleanup after terminating only the owned fixture process.
    #[test]
    fn pending_slot_owned_process_termination_releases_ownership() {
        let namespace = Transaction::generate().expect("public fixture randomness");
        let mut holder = spawn_slot("hold", &namespace);
        let (receive, reader) = reports(&mut holder);
        ready(&receive);
        denied(&namespace);
        holder
            .child
            .kill()
            .expect("terminate owned reservation fixture");
        holder
            .child
            .wait()
            .expect("confirm owned fixture termination");
        reader.join().expect("holder metadata reader");
        recovered(&namespace);
    }
}
