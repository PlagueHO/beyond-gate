//! Offline process checks for the actual host, with no product-pipe connection.
//!
//! Every input fails before pairing. These checks do not contact a browser,
//! account service, pending product login, or protected credential entry.

#![cfg(windows)]

use std::{
    io::Write,
    process::{Child, Command, Output, Stdio},
    thread,
    time::{Duration, Instant},
};

use beyond_gate_core::auth::handoff::browser::EXTENSION_ORIGIN;
use serde_json::{Value, json};

/// Owns exactly one test child and stops it if an assertion or I/O step fails.
struct Host(Option<Child>);

impl Host {
    /// Starts the actual executable with owned input/output pipes and no environment mode.
    fn start(arguments: &[&str]) -> Self {
        Self(Some(
            Command::new(env!("CARGO_BIN_EXE_beyond-gate-native-host"))
                .args(arguments)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("owned host"),
        ))
    }

    /// Sends invented bytes and optionally keeps the pipe open to test the read deadline.
    fn input(&mut self, bytes: &[u8], keep_open: bool) {
        let child = self.0.as_mut().expect("owned child");
        child
            .stdin
            .as_mut()
            .expect("input")
            .write_all(bytes)
            .expect("test input");
        if !keep_open {
            drop(child.stdin.take());
        }
    }

    /// Bounds process lifetime before collecting its fixed, small error output.
    fn finish(mut self, bound: Duration) -> Output {
        let deadline = Instant::now() + bound;
        loop {
            if self
                .0
                .as_mut()
                .expect("owned child")
                .try_wait()
                .expect("status")
                .is_some()
            {
                return self
                    .0
                    .take()
                    .expect("finished child")
                    .wait_with_output()
                    .expect("output");
            }
            assert!(
                Instant::now() < deadline,
                "native host exceeded its process bound"
            );
            thread::sleep(Duration::from_millis(10));
        }
    }
}

impl Drop for Host {
    /// Stops only this test's process and reports cleanup errors explicitly.
    fn drop(&mut self) {
        if let Some(child) = &mut self.0 {
            if let Err(error) = child.kill() {
                eprintln!("owned native host cleanup failed ({:?})", error.kind());
            }
            if let Err(error) = child.wait() {
                eprintln!("owned native host wait failed ({:?})", error.kind());
            }
        }
    }
}

/// Builds one native-endian browser frame from an explicitly invented JSON object.
fn frame(value: Value) -> Vec<u8> {
    let bytes = serde_json::to_vec(&value).expect("invented JSON");
    let mut result = (bytes.len() as u32).to_ne_bytes().to_vec();
    result.extend(bytes);
    result
}

/// Checks exactly one safe protocol error and no unframed stdout or reflected input.
fn failure(output: Output, code: &str) {
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(output.stderr, format!("error: {code}\n").as_bytes());
    let length = u32::from_ne_bytes(output.stdout[..4].try_into().expect("prefix")) as usize;
    assert_eq!(output.stdout.len(), length + 4);
    let message: Value = serde_json::from_slice(&output.stdout[4..]).expect("framed JSON");
    assert_eq!(
        message,
        json!({
            "protocol": "bg-browser-handoff", "version": 1, "type": "error",
            "transaction": null, "code": code,
        })
    );
}

/// Rejects missing, foreign, and malformed origin arguments without reading stdin.
#[test]
fn rejected_launch_has_no_protocol_output_or_input_wait() {
    for arguments in [
        vec![],
        vec!["chrome-extension://aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa/"],
        vec![EXTENSION_ORIGIN, "--parent-window=-1"],
    ] {
        let output = Host::start(&arguments).finish(Duration::from_secs(3));
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        assert_eq!(output.stderr, b"error: origin_rejected\n");
    }
}

/// Rejects invalid prefacing before the connection factory can contact the product pipe.
#[test]
fn invalid_prefaces_never_admit_pairing() {
    for (message, expected) in [
        (
            json!({
                "protocol": "bg-browser-handoff", "version": 1, "type": "pair",
                "browser": "edge", "profile": "cobalt-session-v1", "consent": false,
                "key": "synthetic-invalid",
            }),
            "consent_required",
        ),
        (
            json!({
                "protocol": "bg-browser-handoff", "version": 1, "type": "pair",
                "browser": "edge", "profile": "cobalt-session-v1", "consent": true,
                "key": "synthetic-invalid",
            }),
            "invalid_material",
        ),
        (
            json!({
                "protocol": "bg-browser-handoff", "version": 1, "type": "hello",
                "browser": "edge", "profile": "cobalt-session-v1", "consent": true,
            }),
            "protocol_error",
        ),
        (
            json!({
                "protocol": "bg-browser-handoff", "version": 1, "type": "pair",
                "browser": "edge", "profile": "cobalt-session-v1", "consent": true,
                "key": "synthetic-invalid", "unexpected": "synthetic-only",
            }),
            "protocol_error",
        ),
    ] {
        let mut host = Host::start(&[EXTENSION_ORIGIN]);
        host.input(&frame(message), false);
        failure(host.finish(Duration::from_secs(3)), expected);
    }
    let mut host = Host::start(&[EXTENSION_ORIGIN]);
    host.input(&16_385_u32.to_ne_bytes(), false);
    failure(host.finish(Duration::from_secs(3)), "protocol_error");
}

/// Keeps an incomplete frame open and requires the host's fixed ten-second input cap.
#[test]
fn stalled_preface_stops_without_waiting_for_stdin_eof() {
    let mut host = Host::start(&[EXTENSION_ORIGIN]);
    let start = Instant::now();
    host.input(&[1], true);
    failure(host.finish(Duration::from_secs(13)), "timed_out");
    assert!(start.elapsed() >= Duration::from_secs(10));
}
