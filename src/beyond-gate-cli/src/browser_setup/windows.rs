//! Conflict-safe per-user registration for the pinned Windows browser package.

use std::{
    fs::{self, File, OpenOptions},
    io::{self, BufRead, Read, Write},
    path::{Path, PathBuf},
};

use beyond_gate_core::auth::handoff::browser::{EXTENSION_ORIGIN, HOST_NAME};
use clap::ArgMatches;
use serde_json::{Value, json};
use winreg::{RegKey, enums::*};

use super::SetupError;

const CHROME: &str = r"Software\Google\Chrome\NativeMessagingHosts";
const EDGE: &str = r"Software\Microsoft\Edge\NativeMessagingHosts";
const CHROMIUM: &str = r"Software\Chromium\NativeMessagingHosts";
const VIEWS: [u32; 2] = [KEY_WOW64_32KEY, KEY_WOW64_64KEY];
const MANIFEST_LIMIT: u64 = 16 * 1024;

/// Holds only installed paths and the exact public native-host declaration.
struct Setup {
    directory: PathBuf,
    manifest_path: PathBuf,
    manifest: Value,
    browser: &'static str,
}

impl Setup {
    /// Resolves an explicit stable directory, never an implicit development binary.
    fn new(directory: &Path, browser: &'static str, install: bool) -> Result<Self, SetupError> {
        let directory = fs::canonicalize(directory)?;
        if !directory.is_dir()
            || !matches!(directory.components().next(), Some(std::path::Component::Prefix(prefix))
                if matches!(prefix.kind(), std::path::Prefix::Disk(_) | std::path::Prefix::VerbatimDisk(_)))
            || directory
                .components()
                .any(|part| part.as_os_str().eq_ignore_ascii_case("target"))
        {
            return Err(SetupError::Directory);
        }
        for ancestor in directory.ancestors() {
            match fs::symlink_metadata(ancestor.join(".git")) {
                Ok(_) => return Err(SetupError::Directory),
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
        }
        let directory = ordinary_path(directory)?;
        let executable = directory.join("beyond-gate-native-host.exe");
        if install {
            for path in [directory.join("beyond-gate.exe"), executable.clone()] {
                if !fs::symlink_metadata(path)?.file_type().is_file() {
                    return Err(SetupError::Directory);
                }
            }
        }
        let manifest = json!({
            "name": HOST_NAME,
            "description": "Beyond Gate human-controlled Windows login",
            "path": executable.to_str().ok_or(SetupError::Directory)?,
            "type": "stdio",
            "allowed_origins": [EXTENSION_ORIGIN],
        });
        let manifest_path = directory.join(format!("beyond-gate-native-host.{browser}.json"));
        Ok(Self {
            directory,
            manifest_path,
            manifest,
            browser,
        })
    }

    /// Checks only the two known sibling manifests, never a foreign registry path.
    fn owns_mapping(&self, value: &str) -> Result<bool, SetupError> {
        for browser in ["chrome", "edge"] {
            let path = self
                .directory
                .join(format!("beyond-gate-native-host.{browser}.json"));
            if path.to_str() == Some(value) {
                return Ok(read_manifest(&path)?.as_ref() == Some(&self.manifest));
            }
        }
        Ok(false)
    }

    /// Returns the selected browser's explicit registration rather than a fallback.
    fn target(&self) -> &'static str {
        if self.browser == "chrome" {
            CHROME
        } else {
            EDGE
        }
    }

    /// Returns every native-host lookup family relevant to this browser.
    fn families(&self) -> &[&'static str] {
        if self.browser == "chrome" {
            &[CHROME]
        } else {
            &[EDGE, CHROMIUM, CHROME]
        }
    }
}

/// Converts a canonical local drive path to the normal Windows form used in JSON.
fn ordinary_path(path: PathBuf) -> Result<PathBuf, SetupError> {
    let text = path.to_str().ok_or(SetupError::Directory)?;
    if text.chars().any(char::is_control) {
        return Err(SetupError::Directory);
    }
    Ok(PathBuf::from(text.strip_prefix(r"\\?\").unwrap_or(text)))
}

/// Reads a bounded regular manifest without accepting a link or malformed content.
fn read_manifest(path: &Path) -> Result<Option<Value>, SetupError> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    if !metadata.file_type().is_file() || metadata.len() > MANIFEST_LIMIT {
        return Err(SetupError::Conflict);
    }
    let mut bytes = Vec::new();
    File::open(path)?
        .take(MANIFEST_LIMIT + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MANIFEST_LIMIT {
        return Err(SetupError::Conflict);
    }
    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(SetupError::Json)
}

/// Requires the whole leaf to be one REG_SZ default value, with no extra children.
fn mapping(key: &RegKey) -> Result<String, SetupError> {
    if key.enum_keys().next().transpose()?.is_some() {
        return Err(SetupError::Conflict);
    }
    let values = key.enum_values().take(2).collect::<io::Result<Vec<_>>>()?;
    if values.len() != 1 || !values[0].0.is_empty() || values[0].1.vtype != REG_SZ {
        return Err(SetupError::Conflict);
    }
    key.get_value("").map_err(SetupError::Io)
}

/// Opens a leaf in one view; only a missing key means absence.
fn open_mapping(root: &RegKey, family: &str, view: u32) -> Result<Option<RegKey>, SetupError> {
    match root.open_subkey_with_flags(format!(r"{family}\{HOST_NAME}"), KEY_READ | view) {
        Ok(key) => Ok(Some(key)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

/// Rejects foreign setup in both hives and views, including Edge fallback families.
fn check(setup: &Setup, user: &RegKey, machine: &RegKey) -> Result<(), SetupError> {
    if let Some(manifest) = read_manifest(&setup.manifest_path)?
        && manifest != setup.manifest
    {
        return Err(SetupError::Conflict);
    }
    for root in [user, machine] {
        for family in setup.families() {
            for view in VIEWS {
                if let Some(key) = open_mapping(root, family, view)?
                    && !setup.owns_mapping(&mapping(&key)?)?
                {
                    return Err(SetupError::Conflict);
                }
            }
        }
    }
    for view in VIEWS {
        if let Some(key) = open_mapping(user, setup.target(), view)?
            && mapping(&key)? != setup.manifest_path.to_str().ok_or(SetupError::Directory)?
        {
            return Err(SetupError::Conflict);
        }
    }
    Ok(())
}

/// Creates public JSON exclusively; an existing file must already match exactly.
fn ensure_manifest(setup: &Setup) -> Result<(), SetupError> {
    match OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&setup.manifest_path)
    {
        Ok(mut file) => {
            serde_json::to_writer_pretty(&mut file, &setup.manifest).map_err(SetupError::Json)?;
            file.write_all(b"\n")?;
            file.sync_all()?;
            Ok(())
        }
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
            if read_manifest(&setup.manifest_path)?.as_ref() == Some(&setup.manifest) {
                Ok(())
            } else {
                Err(SetupError::Changed)
            }
        }
        Err(error) => Err(error.into()),
    }
}

/// Atomically creates the default value; a concurrently created leaf is never overwritten.
fn install(setup: &Setup, user: &RegKey, machine: &RegKey) -> Result<(), SetupError> {
    check(setup, user, machine)?;
    ensure_manifest(setup)?;
    let transaction = winreg::transaction::Transaction::new()?;
    let (key, disposition) = user.create_subkey_transacted_with_flags(
        format!(r"{}\{HOST_NAME}", setup.target()),
        &transaction,
        KEY_READ | KEY_WRITE | KEY_WOW64_32KEY,
    )?;
    if disposition == REG_CREATED_NEW_KEY {
        key.set_value(
            "",
            &setup.manifest_path.to_str().ok_or(SetupError::Directory)?,
        )?;
    } else if mapping(&key)? != setup.manifest_path.to_str().ok_or(SetupError::Directory)? {
        return Err(SetupError::Changed);
    }
    drop(key);
    transaction.commit()?;
    check(setup, user, machine)
}

/// Checks ownership and stages selected-browser removal in one registry transaction.
fn stage_removal(
    setup: &Setup,
    user: &RegKey,
    transaction: &winreg::transaction::Transaction,
) -> Result<(), SetupError> {
    for view in VIEWS {
        let path = format!(r"{}\{HOST_NAME}", setup.target());
        let key = match user.open_subkey_transacted_with_flags(
            &path,
            transaction,
            KEY_READ | KEY_WRITE | view,
        ) {
            Ok(key) => key,
            Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error.into()),
        };
        if mapping(&key)? != setup.manifest_path.to_str().ok_or(SetupError::Directory)? {
            return Err(SetupError::Conflict);
        }
        user.delete_subkey_transacted_with_flags(&path, transaction, view)?;
    }
    Ok(())
}

/// Removes only matching selected-browser per-user leaves, preserving machine/fallback entries.
fn uninstall(setup: &Setup, user: &RegKey, machine: &RegKey) -> Result<(), SetupError> {
    check(setup, user, machine)?;
    let transaction = winreg::transaction::Transaction::new()?;
    stage_removal(setup, user, &transaction)?;
    transaction.commit()?;
    for root in [user, machine] {
        for family in [EDGE, CHROMIUM, CHROME] {
            for view in VIEWS {
                if let Some(key) = open_mapping(root, family, view)?
                    && mapping(&key)?
                        == setup.manifest_path.to_str().ok_or(SetupError::Directory)?
                {
                    return Ok(());
                }
            }
        }
    }
    if let Some(manifest) = read_manifest(&setup.manifest_path)? {
        if manifest != setup.manifest {
            return Err(SetupError::Changed);
        }
        fs::remove_file(&setup.manifest_path)?;
    }
    Ok(())
}

/// Shows all relevant lookup locations and asks before touching installed configuration.
fn execute(
    setup: &Setup,
    install_action: bool,
    preview_only: bool,
    user: &RegKey,
    machine: &RegKey,
    stdin: &mut impl BufRead,
    stdout: &mut impl Write,
) -> Result<(), SetupError> {
    check(setup, user, machine)?;
    writeln!(
        stdout,
        "{} Windows {} native host:",
        if install_action { "Install" } else { "Remove" },
        setup.browser
    )?;
    writeln!(
        stdout,
        "Manifest: {}",
        serde_json::to_string(&setup.manifest_path.to_str()).map_err(SetupError::Json)?
    )?;
    writeln!(
        stdout,
        "{}",
        serde_json::to_string_pretty(&setup.manifest).map_err(SetupError::Json)?
    )?;
    writeln!(
        stdout,
        "{} scope: HKCU\\{}\\{HOST_NAME} ({}).",
        if install_action { "Write" } else { "Remove" },
        setup.target(),
        if install_action {
            "32-bit view"
        } else {
            "32-bit and 64-bit views"
        },
    )?;
    writeln!(
        stdout,
        "Conflict checks: HKCU and HKLM, 32-bit and 64-bit views."
    )?;
    writeln!(
        stdout,
        "Machine-wide keys and other browsers are preserved."
    )?;
    if setup.browser == "edge" {
        writeln!(
            stdout,
            "Edge, Chromium, and Chrome lookup locations are checked. Chrome fallback can remain after Edge removal."
        )?;
    }
    writeln!(
        stdout,
        "Registry entries, manifests, and launch mapping are trusted installed configuration."
    )?;
    writeln!(
        stdout,
        "No account request, cookie read, or credential-store access occurs."
    )?;
    stdout.flush()?;
    if preview_only {
        return Ok(());
    }
    write!(stdout, "Apply this local setup? [y/N] ")?;
    stdout.flush()?;
    let mut answer = String::new();
    stdin.take(16).read_line(&mut answer)?;
    if !matches!(answer.trim().to_ascii_lowercase().as_str(), "y" | "yes") {
        writeln!(stdout, "Cancelled; no setup changes were made.")?;
    } else {
        if install_action {
            install(setup, user, machine)?;
        } else {
            uninstall(setup, user, machine)?;
        }
        writeln!(
            stdout,
            "{}",
            if install_action {
                "Local native-host registration complete. Browser integration and account login are separate checks."
            } else {
                "Matching per-user removal complete. Machine-wide and fallback registrations can remain."
            }
        )?;
    }
    stdout.flush()?;
    Ok(())
}

/// Uses real user/machine roots only for an explicit parsed setup operation.
pub(super) fn run(
    arguments: &ArgMatches,
    stdin: &mut impl BufRead,
    stdout: &mut impl Write,
) -> Result<(), SetupError> {
    let (action, args) = arguments.subcommand().ok_or(SetupError::Unsupported)?;
    let browser = match args.get_one::<String>("browser").map(String::as_str) {
        Some("chrome") => "chrome",
        Some("edge") => "edge",
        _ => return Err(SetupError::Unsupported),
    };
    let install_action = action == "install";
    let setup = Setup::new(
        args.get_one::<PathBuf>("directory")
            .ok_or(SetupError::Directory)?,
        browser,
        install_action,
    )?;
    execute(
        &setup,
        install_action,
        args.get_flag("dry-run"),
        &RegKey::predef(HKEY_CURRENT_USER),
        &RegKey::predef(HKEY_LOCAL_MACHINE),
        stdin,
        stdout,
    )
}

#[cfg(test)]
mod tests;
