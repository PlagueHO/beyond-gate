//! Feature-only real-process check that redirected output cannot receive a key.
#![cfg(feature = "pairing-fixture")]

use std::{
    path::PathBuf,
    process::{Command, Stdio},
};

/// Uses only this test build's explicitly compiled sibling fixture.
fn fixture() -> PathBuf {
    let executable = std::env::current_exe().expect("test executable");
    let directory = executable
        .parent()
        .and_then(|path| path.parent())
        .expect("target directory");
    let path = directory.join("examples").join(format!(
        "pairing-terminal-fixture{}",
        std::env::consts::EXE_SUFFIX
    ));
    assert!(
        path.is_file(),
        "build --example pairing-terminal-fixture --features pairing-fixture first"
    );
    path
}

/// Checks the actual terminal gate without recording any generated key or account material.
#[test]
fn redirected_process_never_emits_pairing_material() {
    let output = Command::new(fixture())
        .arg("preview")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("synthetic display process");
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    let error = String::from_utf8(output.stderr).expect("fixed diagnostics");
    assert_eq!(
        error.trim(),
        "pairing requires interactive stdin, stdout, and stderr"
    );
    assert!(!error.contains("Kioq"));
}
