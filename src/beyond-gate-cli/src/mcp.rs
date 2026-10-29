//! Local MCP server launch and user-level client registration.

use std::{
    env, fs,
    fs::OpenOptions,
    io::{self, BufRead, Write},
    path::{Path, PathBuf},
    process::{Command, ExitStatus},
    sync::atomic::{AtomicUsize, Ordering},
};

use serde_json::{Map, Value, json};

static TEMP_FILE_SEQUENCE: AtomicUsize = AtomicUsize::new(0);

/// Identifies one supported MCP client configuration.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Client {
    Vscode,
    ClaudeCode,
    CopilotCli,
}

impl Client {
    /// Returns the fixed display name used in messages.
    fn display_name(self) -> &'static str {
        match self {
            Self::Vscode => "VS Code",
            Self::ClaudeCode => "Claude Code",
            Self::CopilotCli => "GitHub Copilot CLI",
        }
    }

    /// Returns the executable used to detect an installed client.
    fn executable_name(self) -> &'static str {
        match self {
            Self::Vscode => "code",
            Self::ClaudeCode => "claude",
            Self::CopilotCli => "copilot",
        }
    }

    /// Returns the user's documented MCP configuration path.
    fn config_path(self) -> Result<PathBuf, McpError> {
        match self {
            Self::Vscode | Self::CopilotCli => {
                let directory = match env::var_os("COPILOT_HOME") {
                    Some(directory) if !directory.is_empty() => PathBuf::from(directory),
                    Some(_) => return Err(McpError::HomeDirectory),
                    None => home_directory()?.join(".copilot"),
                };
                Ok(directory.join("mcp-config.json"))
            }
            Self::ClaudeCode => Ok(home_directory()?.join(".claude.json")),
        }
    }

    /// Builds the documented local stdio entry for this client.
    fn server_entry(self, executable: &Path) -> Result<Value, McpError> {
        let command = executable
            .to_str()
            .ok_or(McpError::ExecutablePathNotUnicode)?;
        let mut entry = json!({
            "command": command,
            "args": []
        });
        if matches!(self, Self::Vscode | Self::CopilotCli) {
            entry["type"] = Value::String("local".to_owned());
        }
        Ok(entry)
    }
}

/// Reports safe launch, configuration, and confirmation failures.
#[derive(Debug)]
pub(crate) enum McpError {
    CurrentExecutable(io::Error),
    MissingCompanion,
    Launch(io::Error),
    MissingClient(&'static str),
    HomeDirectory,
    ExecutablePathNotUnicode,
    SettingsRead(io::Error),
    SettingsWrite(io::Error),
    SettingsJson(serde_json::Error),
    InvalidSettingsShape,
    ServerNameConflict,
    ConfirmationRead(io::Error),
    Output(io::Error),
}

impl std::fmt::Display for McpError {
    /// Keeps diagnostics useful without displaying raw configuration content.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CurrentExecutable(_) => {
                formatter.write_str("The CLI executable location could not be resolved.")
            }
            Self::MissingCompanion => formatter.write_str(
                "The sibling `beyond-gate-mcp` executable is missing; install or build both binaries together.",
            ),
            Self::Launch(error) => {
                write!(formatter, "The local MCP server could not start: {}.", io_reason(error))
            }
            Self::MissingClient(client) => {
                write!(formatter, "{client} was not found on PATH.")
            }
            Self::HomeDirectory => {
                formatter.write_str("The current user's configuration directory could not be resolved.")
            }
            Self::ExecutablePathNotUnicode => {
                formatter.write_str("The MCP executable path cannot be stored in client JSON.")
            }
            Self::SettingsRead(error) => write!(
                formatter,
                "Client settings could not be read: {}.",
                io_reason(error)
            ),
            Self::SettingsWrite(error) => write!(
                formatter,
                "Client settings could not be written: {}.",
                io_reason(error)
            ),
            Self::SettingsJson(_) => formatter.write_str(
                "Client settings contain malformed JSON; no changes were made.",
            ),
            Self::InvalidSettingsShape => formatter.write_str(
                "Client settings must be a JSON object with an object-valued `mcpServers` field; no changes were made.",
            ),
            Self::ServerNameConflict => formatter.write_str(
                "A different server already uses the `beyond-gate` name; no changes were made.",
            ),
            Self::ConfirmationRead(error) => write!(
                formatter,
                "The install confirmation could not be read: {}.",
                io_reason(error)
            ),
            Self::Output(_) => formatter.write_str("The MCP install preview could not be written."),
        }
    }
}

impl std::error::Error for McpError {
    /// Retains the operating-system or JSON cause for callers.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::CurrentExecutable(error)
            | Self::Launch(error)
            | Self::SettingsRead(error)
            | Self::SettingsWrite(error)
            | Self::ConfirmationRead(error) => Some(error),
            Self::Output(error) => Some(error),
            Self::SettingsJson(error) => Some(error),
            Self::MissingCompanion
            | Self::MissingClient(_)
            | Self::HomeDirectory
            | Self::ExecutablePathNotUnicode
            | Self::InvalidSettingsShape
            | Self::ServerNameConflict => None,
        }
    }
}

/// Describes an operating-system failure without exposing a path or config value.
fn io_reason(error: &io::Error) -> &'static str {
    match error.kind() {
        io::ErrorKind::PermissionDenied => "permission was denied",
        io::ErrorKind::NotFound => "a file or directory was not found",
        _ => "an operating-system error occurred",
    }
}

/// Returns the current user's home directory without a plaintext fallback.
fn home_directory() -> Result<PathBuf, McpError> {
    env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
        .map(PathBuf::from)
        .ok_or(McpError::HomeDirectory)
}

/// Finds a client command in the supplied search paths.
fn command_in_path(name: &str, directories: &[PathBuf], extensions: &[&str]) -> bool {
    directories.iter().any(|directory| {
        extensions.iter().any(|extension| {
            let filename = if extension.is_empty() {
                name.to_owned()
            } else {
                format!("{name}{extension}")
            };
            directory.join(filename).is_file()
        })
    })
}

/// Checks whether the selected client command is available on the current PATH.
fn client_is_available(client: Client) -> bool {
    let directories: Vec<_> = env::split_paths(&env::var_os("PATH").unwrap_or_default()).collect();
    #[cfg(windows)]
    let extensions: Vec<String> = env::var("PATHEXT")
        .unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".to_owned())
        .split(';')
        .map(str::to_owned)
        .collect();
    #[cfg(not(windows))]
    let extensions = vec![String::new()];

    command_in_path(
        client.executable_name(),
        &directories,
        &extensions.iter().map(String::as_str).collect::<Vec<_>>(),
    )
}

/// Returns the platform-specific name of the companion MCP executable.
fn companion_path(executable: &Path) -> Result<PathBuf, McpError> {
    let directory = executable.parent().ok_or(McpError::MissingCompanion)?;
    let name = if cfg!(windows) {
        "beyond-gate-mcp.exe"
    } else {
        "beyond-gate-mcp"
    };
    Ok(directory.join(name))
}

/// Resolves the installed MCP companion beside the current CLI executable.
pub(crate) fn companion_executable() -> Result<PathBuf, McpError> {
    let current = env::current_exe().map_err(McpError::CurrentExecutable)?;
    companion_executable_at(&current)
}

/// Resolves an MCP companion beside a known CLI executable path.
fn companion_executable_at(executable: &Path) -> Result<PathBuf, McpError> {
    let companion = companion_path(executable)?;
    if companion.is_file() {
        Ok(companion)
    } else {
        Err(McpError::MissingCompanion)
    }
}

/// Starts the sibling MCP process with inherited stdio and no shell.
pub(crate) fn start_server() -> Result<ExitStatus, McpError> {
    let companion = companion_executable()?;
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;

        Err(McpError::Launch(Command::new(companion).exec()))
    }
    #[cfg(windows)]
    {
        Command::new(companion).status().map_err(McpError::Launch)
    }
    #[cfg(not(any(unix, windows)))]
    {
        Command::new(companion).status().map_err(McpError::Launch)
    }
}

/// Starts one executable directly so paths with spaces need no shell quoting.
#[cfg(test)]
fn run_companion(executable: &Path, arguments: &[&str]) -> Result<ExitStatus, McpError> {
    Command::new(executable)
        .args(arguments)
        .status()
        .map_err(McpError::Launch)
}

/// Loads client settings or creates an empty portable settings object.
fn read_settings(path: &Path) -> Result<Value, McpError> {
    match fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes).map_err(McpError::SettingsJson),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(json!({})),
        Err(error) => Err(McpError::SettingsRead(error)),
    }
}

/// Adds one named server without changing unrelated JSON values.
fn updated_settings(
    mut settings: Value,
    client: Client,
    executable: &Path,
) -> Result<(Value, bool), McpError> {
    let root = settings
        .as_object_mut()
        .ok_or(McpError::InvalidSettingsShape)?;
    let servers = root
        .entry("mcpServers")
        .or_insert_with(|| Value::Object(Map::new()))
        .as_object_mut()
        .ok_or(McpError::InvalidSettingsShape)?;
    let entry = client.server_entry(executable)?;
    if let Some(existing) = servers.get("beyond-gate") {
        if *existing == entry {
            return Ok((settings, false));
        }
        return Err(McpError::ServerNameConflict);
    }
    servers.insert("beyond-gate".to_owned(), entry);
    Ok((settings, true))
}

/// Writes complete JSON through a same-directory temporary file and rename.
fn write_settings(path: &Path, settings: &Value) -> Result<(), McpError> {
    let parent = path.parent().ok_or(McpError::InvalidSettingsShape)?;
    fs::create_dir_all(parent).map_err(McpError::SettingsWrite)?;
    let existing_permissions = match fs::metadata(path) {
        Ok(metadata) => Some(metadata.permissions()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(error) => return Err(McpError::SettingsWrite(error)),
    };
    if existing_permissions
        .as_ref()
        .is_some_and(std::fs::Permissions::readonly)
    {
        return Err(McpError::SettingsWrite(io::Error::from(
            io::ErrorKind::PermissionDenied,
        )));
    }
    let contents = serde_json::to_vec_pretty(settings).map_err(McpError::SettingsJson)?;
    let mut contents = contents;
    contents.push(b'\n');

    let temporary = loop {
        let sequence = TEMP_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let name = path
            .file_name()
            .ok_or(McpError::InvalidSettingsShape)?
            .to_string_lossy();
        let candidate = parent.join(format!(".{name}.{}.{}.tmp", std::process::id(), sequence));
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
            Ok(mut file) => {
                if let Err(error) = file.write_all(&contents).and_then(|()| file.sync_all()) {
                    drop(file);
                    let _ = fs::remove_file(&candidate);
                    return Err(McpError::SettingsWrite(error));
                }
                if let Some(permissions) = existing_permissions.as_ref()
                    && let Err(error) = fs::set_permissions(&candidate, permissions.clone())
                {
                    drop(file);
                    let _ = fs::remove_file(&candidate);
                    return Err(McpError::SettingsWrite(error));
                }
                break candidate;
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(McpError::SettingsWrite(error)),
        }
    };

    if let Err(error) = fs::rename(&temporary, path) {
        let _ = fs::remove_file(&temporary);
        return Err(McpError::SettingsWrite(error));
    }
    Ok(())
}

/// Shows the exact registration entry before any settings change.
fn preview(client: Client, path: &Path, entry: &Value) -> Result<String, McpError> {
    let entry = serde_json::to_string_pretty(entry).map_err(McpError::SettingsJson)?;
    Ok(format!(
        "{} user settings: {}\nProposed `mcpServers.beyond-gate` entry:\n{}\n",
        client.display_name(),
        path.display(),
        entry
    ))
}

/// Installs a server entry after preview and explicit user confirmation.
pub(crate) fn install(
    client: Client,
    executable: &Path,
    preview_only: bool,
    input: &mut impl BufRead,
    output: &mut impl Write,
) -> Result<(), McpError> {
    if !client_is_available(client) {
        return Err(McpError::MissingClient(client.display_name()));
    }
    let path = client.config_path()?;
    install_at(client, &path, executable, preview_only, input, output)
}

/// Previews and applies one client registration at an explicit settings path.
fn install_at(
    client: Client,
    path: &Path,
    executable: &Path,
    preview_only: bool,
    input: &mut impl BufRead,
    output: &mut impl Write,
) -> Result<(), McpError> {
    let settings = read_settings(path)?;
    let (updated, changed) = updated_settings(settings, client, executable)?;
    if !changed {
        writeln!(
            output,
            "{} is already registered. No settings changed.",
            client.display_name()
        )
        .map_err(McpError::Output)?;
        return Ok(());
    }
    let entry = updated
        .get("mcpServers")
        .and_then(Value::as_object)
        .and_then(|servers| servers.get("beyond-gate"))
        .ok_or(McpError::InvalidSettingsShape)?
        .clone();
    output
        .write_all(preview(client, path, &entry)?.as_bytes())
        .map_err(McpError::Output)?;
    if preview_only {
        writeln!(output, "Preview only; no settings changed.").map_err(McpError::Output)?;
        return Ok(());
    }

    output
        .write_all(b"Apply this change? [y/N] ")
        .map_err(McpError::Output)?;
    output.flush().map_err(McpError::Output)?;
    let mut answer = String::new();
    input
        .read_line(&mut answer)
        .map_err(McpError::ConfirmationRead)?;
    if !matches!(answer.trim().to_ascii_lowercase().as_str(), "y" | "yes") {
        writeln!(output, "Canceled; no settings changed.").map_err(McpError::Output)?;
        return Ok(());
    }
    let latest = read_settings(path)?;
    let (updated, changed) = updated_settings(latest, client, executable)?;
    if !changed {
        writeln!(
            output,
            "{} is already registered. No settings changed.",
            client.display_name()
        )
        .map_err(McpError::Output)?;
        return Ok(());
    }
    write_settings(path, &updated)?;
    writeln!(output, "Registered with {}.", client.display_name()).map_err(McpError::Output)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static TEST_DIRECTORY_SEQUENCE: AtomicUsize = AtomicUsize::new(0);

    /// Holds a unique temporary directory and removes only that directory.
    struct TestDirectory(PathBuf);

    impl TestDirectory {
        /// Creates a temporary directory with a space in its name.
        fn new() -> Self {
            let sequence = TEST_DIRECTORY_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let path = env::temp_dir().join(format!(
                "beyond gate test {} {sequence}",
                std::process::id()
            ));
            fs::create_dir_all(&path).expect("temporary directory");
            Self(path)
        }

        /// Returns the temporary directory path.
        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TestDirectory {
        /// Removes the test's resolved temporary directory.
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    /// Preserves unrelated settings and makes repeat installation a no-op.
    #[test]
    fn settings_merge_is_idempotent() {
        let executable = Path::new(r"C:\Program Files\Beyond Gate\beyond-gate-mcp.exe");
        let settings = json!({
            "custom": {"keep": true},
            "mcpServers": {"other": {"command": "other"}}
        });
        let (updated, changed) =
            updated_settings(settings, Client::CopilotCli, executable).expect("merge");
        assert!(changed);
        assert_eq!(updated["custom"]["keep"], true);
        assert_eq!(updated["mcpServers"]["other"]["command"], "other");
        assert_eq!(
            updated["mcpServers"]["beyond-gate"]["command"],
            executable.to_str().expect("Unicode path")
        );
        let (repeated, changed) =
            updated_settings(updated.clone(), Client::CopilotCli, executable).expect("repeat");
        assert!(!changed);
        assert_eq!(repeated, updated);
    }

    /// Rejects malformed JSON, invalid root types, and name collisions.
    #[test]
    fn invalid_or_conflicting_settings_are_rejected() {
        assert!(matches!(
            updated_settings(json!([]), Client::ClaudeCode, Path::new("server")),
            Err(McpError::InvalidSettingsShape)
        ));
        assert!(matches!(
            updated_settings(
                json!({"mcpServers": {"beyond-gate": {"command": "different"}}}),
                Client::ClaudeCode,
                Path::new("server")
            ),
            Err(McpError::ServerNameConflict)
        ));
    }

    /// Detects malformed client JSON without changing the source file.
    #[test]
    fn malformed_settings_are_rejected() {
        let directory = TestDirectory::new();
        let path = directory.path().join("mcp-config.json");
        fs::write(&path, b"{").expect("malformed settings");
        assert!(matches!(
            read_settings(&path),
            Err(McpError::SettingsJson(_))
        ));
    }

    /// Writes and rereads a synthetic settings file without losing other values.
    #[test]
    fn settings_write_preserves_existing_values() {
        let directory = TestDirectory::new();
        let path = directory.path().join("mcp-config.json");
        let settings = json!({"other": {"value": 7}});
        write_settings(&path, &settings).expect("write settings");
        assert_eq!(read_settings(&path).expect("read settings"), settings);
    }

    /// Applies changes only after confirmation and leaves canceled previews alone.
    #[test]
    fn install_confirmation_controls_writes() {
        let directory = TestDirectory::new();
        let path = directory.path().join("mcp-config.json");
        let executable = directory
            .path()
            .join("binary with spaces")
            .join("beyond-gate-mcp");
        let original = json!({
            "custom": {"keep": true},
            "mcpServers": {"other": {"command": "other"}}
        });
        write_settings(&path, &original).expect("existing client settings");

        let mut output = Vec::new();
        install_at(
            Client::CopilotCli,
            &path,
            &executable,
            false,
            &mut io::Cursor::new(b"n\n"),
            &mut output,
        )
        .expect("cancellation");
        assert_eq!(
            read_settings(&path).expect("settings after cancel"),
            original
        );
        assert!(String::from_utf8_lossy(&output).contains("Canceled"));

        output.clear();
        install_at(
            Client::CopilotCli,
            &path,
            &executable,
            true,
            &mut io::Cursor::new(b"y\n"),
            &mut output,
        )
        .expect("preview");
        assert_eq!(
            read_settings(&path).expect("settings after preview"),
            original
        );
        assert!(String::from_utf8_lossy(&output).contains("Preview only"));
        assert!(!String::from_utf8_lossy(&output).contains("Apply this change?"));

        install_at(
            Client::CopilotCli,
            &path,
            &executable,
            false,
            &mut io::Cursor::new(b"yes\n"),
            &mut output,
        )
        .expect("confirmed installation");
        let settings = read_settings(&path).expect("written settings");
        assert_eq!(settings["custom"]["keep"], true);
        assert_eq!(settings["mcpServers"]["other"]["command"], "other");
        assert_eq!(
            settings["mcpServers"]["beyond-gate"]["command"],
            executable.to_str().expect("Unicode executable path")
        );

        output.clear();
        install_at(
            Client::CopilotCli,
            &path,
            &executable,
            false,
            &mut io::Cursor::new(b""),
            &mut output,
        )
        .expect("repeat installation");
        assert!(String::from_utf8_lossy(&output).contains("already registered"));
        assert_eq!(
            read_settings(&path).expect("settings after repeat"),
            settings
        );
    }

    /// Reports file-write failures and a clear permission-denied category.
    #[test]
    fn settings_write_failures_are_explicit() {
        let directory = TestDirectory::new();
        let blocking_file = directory.path().join("not-a-directory");
        fs::write(&blocking_file, b"").expect("blocking file");
        assert!(matches!(
            write_settings(&blocking_file.join("mcp-config.json"), &json!({})),
            Err(McpError::SettingsWrite(_))
        ));

        let read_only = directory.path().join("read-only.json");
        fs::write(&read_only, b"{}").expect("read-only settings");
        let mut permissions = fs::metadata(&read_only)
            .expect("read-only metadata")
            .permissions();
        permissions.set_readonly(true);
        fs::set_permissions(&read_only, permissions).expect("set read-only permissions");
        let error = write_settings(&read_only, &json!({})).expect_err("read-only settings");
        assert!(matches!(error, McpError::SettingsWrite(_)));
        assert!(error.to_string().contains("permission was denied"));
        assert_eq!(
            io_reason(&io::Error::from(io::ErrorKind::PermissionDenied)),
            "permission was denied"
        );
    }

    /// Rejects a missing companion beside an explicitly supplied executable.
    #[test]
    fn missing_companion_is_rejected() {
        let directory = TestDirectory::new();
        let executable = directory.path().join(if cfg!(windows) {
            "beyond-gate.exe"
        } else {
            "beyond-gate"
        });
        assert!(matches!(
            companion_executable_at(&executable),
            Err(McpError::MissingCompanion)
        ));
    }

    /// Retains restrictive file permissions when settings are replaced on Unix.
    #[cfg(unix)]
    #[test]
    fn settings_write_preserves_unix_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let directory = TestDirectory::new();
        let path = directory.path().join("mcp-config.json");
        fs::write(&path, b"{}").expect("existing settings");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))
            .expect("restrict settings permissions");
        write_settings(&path, &json!({"mcpServers": {}})).expect("replace settings");
        assert_eq!(
            fs::metadata(&path)
                .expect("settings metadata")
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }

    /// Keeps the distinct client locations and client-specific local transport fields.
    #[test]
    fn client_paths_and_entries_are_distinct() {
        let executable = Path::new(r"C:\Program Files\Beyond Gate\beyond-gate-mcp.exe");
        assert_eq!(
            Client::CopilotCli
                .server_entry(executable)
                .expect("Copilot entry")["type"],
            "local"
        );
        assert!(
            Client::ClaudeCode
                .server_entry(executable)
                .expect("Claude entry")["type"]
                .is_null()
        );
    }

    /// Detects a client command in a supplied search path without running it.
    #[test]
    fn command_search_checks_only_candidate_files() {
        let directory = TestDirectory::new();
        fs::write(directory.path().join("copilot.cmd"), b"").expect("fake command");
        assert!(command_in_path(
            "copilot",
            &[directory.path().to_owned()],
            &[".cmd", ".exe"]
        ));
        assert!(!command_in_path(
            "claude",
            &[directory.path().to_owned()],
            &[".cmd", ".exe"]
        ));
    }

    /// Launches a fake child from a path with spaces and preserves its exit code.
    #[test]
    fn fake_companion_path_and_exit_status_are_preserved() {
        let directory = TestDirectory::new();
        let cli_name = if cfg!(windows) {
            "beyond-gate.exe"
        } else {
            "beyond-gate"
        };
        assert_eq!(
            companion_path(&directory.path().join(cli_name)).expect("companion path"),
            directory.path().join(if cfg!(windows) {
                "beyond-gate-mcp.exe"
            } else {
                "beyond-gate-mcp"
            })
        );
        let executable_name = if cfg!(windows) {
            "fake companion.exe"
        } else {
            "fake companion"
        };
        let executable = directory.path().join(executable_name);
        fs::copy(env::current_exe().expect("test executable"), &executable)
            .expect("copy test executable");
        let status = run_companion(
            &executable,
            &[
                "--ignored",
                "--exact",
                "mcp::tests::fake_companion_child_exits_with_37",
                "--nocapture",
            ],
        )
        .expect("launch fake companion");
        assert_eq!(status.code(), Some(37));
    }

    /// Exits with a known status when launched as a fake MCP companion.
    #[test]
    #[ignore]
    fn fake_companion_child_exits_with_37() {
        std::process::exit(37);
    }
}
