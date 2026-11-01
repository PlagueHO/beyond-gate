//! Real registry API checks under unique synthetic roots, never product registration.

use super::*;
use std::sync::atomic::{AtomicU64, Ordering};

static SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Owns one synthetic directory and private registry subtree for a single test.
struct Fixture {
    path: PathBuf,
    registry_path: String,
    user: RegKey,
    machine: RegKey,
}

impl Fixture {
    /// Creates non-executable invented files and roots unrelated to browser lookup.
    fn new() -> Self {
        let name = format!(
            "beyond-gate-setup-{}-{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        );
        let path = std::env::temp_dir().join(&name);
        fs::create_dir(&path).expect("unique synthetic directory");
        for binary in ["beyond-gate.exe", "beyond-gate-native-host.exe"] {
            fs::write(path.join(binary), b"invented non-executable test file")
                .expect("invented binary");
        }
        let registry_path = format!(r"Software\BeyondGateSetupTests\{name}");
        let (root, disposition) = RegKey::predef(HKEY_CURRENT_USER)
            .create_subkey(&registry_path)
            .expect("private synthetic registry root");
        assert_eq!(disposition, REG_CREATED_NEW_KEY);
        let user = root.create_subkey("user").expect("synthetic user root").0;
        let machine = root
            .create_subkey("machine")
            .expect("synthetic machine root")
            .0;
        Self {
            path,
            registry_path,
            user,
            machine,
        }
    }

    /// Resolves the invented installation through the production path checks.
    fn setup(&self, browser: &'static str) -> Setup {
        Setup::new(&self.path, browser, true).expect("synthetic stable installation")
    }

    /// Adds a known synthetic leaf to one private hive and view.
    fn put(&self, machine: bool, family: &str, view: u32, value: &str) -> RegKey {
        let root = if machine { &self.machine } else { &self.user };
        let key = root
            .create_subkey_with_flags(
                format!(r"{family}\{HOST_NAME}"),
                KEY_READ | KEY_WRITE | view,
            )
            .expect("synthetic leaf")
            .0;
        key.set_value("", &value).expect("synthetic mapping");
        key
    }
}

impl Drop for Fixture {
    /// Removes only this exact owned subtree and the known invented files.
    fn drop(&mut self) {
        let result = RegKey::predef(HKEY_CURRENT_USER).delete_subkey_all(&self.registry_path);
        assert!(result.is_ok(), "owned synthetic registry cleanup failed");
        for file in [
            "beyond-gate.exe",
            "beyond-gate-native-host.exe",
            "beyond-gate-native-host.chrome.json",
            "beyond-gate-native-host.edge.json",
            ".git",
        ] {
            match fs::remove_file(self.path.join(file)) {
                Ok(()) => {}
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(_) => panic!("owned synthetic file cleanup failed"),
            }
        }
        fs::remove_dir(&self.path).expect("empty owned test directory");
    }
}

/// Preview and rejected confirmation leave files and both synthetic hives unchanged.
#[test]
fn preview_and_cancel_do_not_write() {
    let fixture = Fixture::new();
    let setup = fixture.setup("edge");
    for (preview, input) in [
        (true, "yes\n"),
        (false, "no\n"),
        (false, ""),
        (false, "yes followed by extra input\n"),
    ] {
        let mut output = Vec::new();
        execute(
            &setup,
            true,
            preview,
            &fixture.user,
            &fixture.machine,
            &mut io::Cursor::new(input),
            &mut output,
        )
        .expect("preview or cancellation");
        assert!(
            String::from_utf8(output)
                .expect("public preview")
                .contains(EXTENSION_ORIGIN)
        );
        assert!(!setup.manifest_path.exists());
        assert!(
            open_mapping(&fixture.user, EDGE, KEY_WOW64_32KEY)
                .expect("private lookup")
                .is_none()
        );
    }
}

/// Both browsers can share one installation; repeated install and removal preserve siblings.
#[test]
fn installation_is_idempotent_and_removal_is_owned() {
    let fixture = Fixture::new();
    let chrome = fixture.setup("chrome");
    let edge = fixture.setup("edge");
    for setup in [&chrome, &chrome, &edge, &edge] {
        install(setup, &fixture.user, &fixture.machine).expect("transactional installation");
        let key = open_mapping(&fixture.user, setup.target(), KEY_WOW64_32KEY)
            .expect("lookup")
            .expect("registered");
        assert_eq!(
            mapping(&key).expect("strict default"),
            setup.manifest_path.to_str().expect("path")
        );
        assert_eq!(
            read_manifest(&setup.manifest_path).expect("manifest"),
            Some(setup.manifest.clone())
        );
    }
    uninstall(&edge, &fixture.user, &fixture.machine).expect("remove Edge");
    assert!(!edge.manifest_path.exists());
    assert!(chrome.manifest_path.exists());
    assert!(
        open_mapping(&fixture.user, CHROME, KEY_WOW64_32KEY)
            .expect("lookup")
            .is_some()
    );
    uninstall(&chrome, &fixture.user, &fixture.machine).expect("remove Chrome");
    uninstall(&chrome, &fixture.user, &fixture.machine).expect("repeat removal");
    assert!(!chrome.manifest_path.exists());
    assert!(fixture.path.join("beyond-gate-native-host.exe").exists());
}

/// Staged removal is invisible until commit and rollback preserves the original mapping.
#[test]
fn removal_transaction_preserves_mapping_until_commit() {
    let fixture = Fixture::new();
    let setup = fixture.setup("edge");
    install(&setup, &fixture.user, &fixture.machine).expect("own install");
    let transaction = winreg::transaction::Transaction::new().expect("removal transaction");
    stage_removal(&setup, &fixture.user, &transaction).expect("staged owned removal");
    let key = open_mapping(&fixture.user, EDGE, KEY_WOW64_32KEY)
        .expect("committed lookup")
        .expect("mapping remains before commit");
    assert_eq!(
        mapping(&key).expect("committed mapping"),
        setup.manifest_path.to_str().expect("path")
    );
    transaction.rollback().expect("rollback staged removal");
    assert!(
        open_mapping(&fixture.user, EDGE, KEY_WOW64_32KEY)
            .expect("preserved lookup")
            .is_some()
    );
    assert!(setup.manifest_path.exists());
}

/// A concurrent replacement either fails or aborts removal without losing the replacement.
#[test]
fn concurrent_mapping_change_cannot_be_deleted_by_removal() {
    let fixture = Fixture::new();
    let setup = fixture.setup("edge");
    install(&setup, &fixture.user, &fixture.machine).expect("own install");
    let key = fixture
        .user
        .open_subkey_with_flags(format!(r"{EDGE}\{HOST_NAME}"), KEY_READ | KEY_WRITE)
        .expect("separate writer");
    let transaction = winreg::transaction::Transaction::new().expect("removal transaction");
    stage_removal(&setup, &fixture.user, &transaction).expect("staged owned removal");
    let foreign = r"C:\invented-concurrent-host.json";
    if key.set_value("", &foreign).is_ok() {
        assert!(
            transaction.commit().is_err(),
            "changed mapping aborts removal"
        );
        assert_eq!(mapping(&key).expect("preserved replacement"), foreign);
    } else {
        transaction.rollback().expect("rollback removal");
        assert_eq!(
            mapping(&key).expect("preserved owned mapping"),
            setup.manifest_path.to_str().expect("path")
        );
    }
    assert!(setup.manifest_path.exists());
}

/// A foreign entry in either hive, either view, or any Edge fallback blocks writes.
#[test]
fn foreign_lookup_entries_are_never_overwritten() {
    for machine in [false, true] {
        for view in VIEWS {
            for family in [EDGE, CHROMIUM, CHROME] {
                let fixture = Fixture::new();
                let setup = fixture.setup("edge");
                let key = fixture.put(machine, family, view, r"C:\invented-foreign-host.json");
                assert!(matches!(
                    install(&setup, &fixture.user, &fixture.machine),
                    Err(SetupError::Conflict)
                ));
                assert!(matches!(
                    uninstall(&setup, &fixture.user, &fixture.machine),
                    Err(SetupError::Conflict)
                ));
                assert_eq!(
                    mapping(&key).expect("unchanged foreign mapping"),
                    r"C:\invented-foreign-host.json"
                );
                assert!(!setup.manifest_path.exists());
            }
        }
    }
}

/// Extra registry data, alternate value types, and foreign manifest content are not adopted.
#[test]
fn malformed_or_changed_setup_is_preserved() {
    let fixture = Fixture::new();
    let setup = fixture.setup("edge");
    install(&setup, &fixture.user, &fixture.machine).expect("own install");
    let key = fixture.put(
        false,
        EDGE,
        KEY_WOW64_32KEY,
        setup.manifest_path.to_str().expect("path"),
    );
    key.set_value("foreign", &1u32)
        .expect("invented extra value");
    assert!(matches!(
        uninstall(&setup, &fixture.user, &fixture.machine),
        Err(SetupError::Conflict)
    ));
    key.delete_value("foreign").expect("remove invented extra");
    key.set_value("", &1u32).expect("wrong type");
    assert!(matches!(
        check(&setup, &fixture.user, &fixture.machine),
        Err(SetupError::Conflict)
    ));
    key.set_value("", &setup.manifest_path.to_str().expect("path"))
        .expect("restore invented mapping");
    fs::write(&setup.manifest_path, b"{\"invented\":\"foreign\"}").expect("changed public file");
    assert!(matches!(
        uninstall(&setup, &fixture.user, &fixture.machine),
        Err(SetupError::Conflict)
    ));
    assert_eq!(
        fs::read(&setup.manifest_path).expect("preserved file"),
        b"{\"invented\":\"foreign\"}"
    );
    fs::write(&setup.manifest_path, b"not-json").expect("malformed public file");
    let error = uninstall(&setup, &fixture.user, &fixture.machine)
        .expect_err("malformed JSON blocks removal");
    assert!(matches!(error, SetupError::Json(_)));
    assert!(std::error::Error::source(&error).is_some());
    assert_eq!(
        fs::read(&setup.manifest_path).expect("preserved malformed file"),
        b"not-json"
    );
}

/// A missing binary blocks installation, but matching removal remains possible.
#[test]
fn missing_binary_and_source_directories_fail_before_setup() {
    let fixture = Fixture::new();
    let setup = fixture.setup("chrome");
    install(&setup, &fixture.user, &fixture.machine).expect("own install");
    fs::remove_file(fixture.path.join("beyond-gate-native-host.exe"))
        .expect("remove invented binary");
    assert!(Setup::new(&fixture.path, "chrome", true).is_err());
    let removal = Setup::new(&fixture.path, "chrome", false).expect("removal does not need binary");
    uninstall(&removal, &fixture.user, &fixture.machine).expect("remove stale own setup");
    fs::write(fixture.path.join(".git"), b"invented source marker").expect("source marker");
    assert!(matches!(
        Setup::new(&fixture.path, "chrome", false),
        Err(SetupError::Directory)
    ));
    assert!(matches!(
        Setup::new(Path::new(env!("CARGO_MANIFEST_DIR")), "chrome", true),
        Err(SetupError::Directory)
    ));
    assert_eq!(setup.manifest["allowed_origins"], json!([EXTENSION_ORIGIN]));
}

/// A deleted root returns its registry failure rather than treating it as absent.
#[test]
fn deleted_registry_root_reports_failure() {
    let fixture = Fixture::new();
    let setup = fixture.setup("edge");
    RegKey::predef(HKEY_CURRENT_USER)
        .delete_subkey(format!(r"{}\user", fixture.registry_path))
        .expect("delete empty owned root");
    assert!(matches!(
        install(&setup, &fixture.user, &fixture.machine),
        Err(SetupError::Io(_))
    ));
    assert!(!setup.manifest_path.exists());
}
