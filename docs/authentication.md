# Authentication

Before Beyond Gate can read your account data, it needs to confirm that you are
signed in to D&D Beyond. This is called **authentication**.

You do not give Beyond Gate your password. Instead, it uses a **session**:
a secret value that D&D Beyond uses to remember your sign-in.
Your browser keeps this value in a cookie called `CobaltSession`.
The Beyond Gate extension can pass it to the app after you approve, so you do
not have to find or copy it yourself.

If you choose to save the session, Beyond Gate first checks it with D&D Beyond.
It then keeps it in your operating system's secure storage.
Your terminal commands and connected AI tools can use that saved session.
If the session stops working, you can sign in again.
Signing in does not change what your D&D Beyond account can access.

After [installing Beyond Gate](installation.md), choose the method that suits you:

- **Browser extension (recommended on Windows):** Sign in normally with Chrome
  or Edge, then approve the connection to Beyond Gate.
- **Supplied `CobaltSession` (Windows or Linux):** Use this if you already have
  a session value.

macOS authentication for production reads is not available yet.
Keep session values private, just as you would a password.
Do not put them in commands, chat messages, logs, or client configuration files.
Read [Privacy](privacy.md) for environment inheritance, protected storage,
extension permissions, and client retention.
Read [Support and compatibility](support.md) for tested browser and OS limits.

## Use the browser extension

This method is for Windows. It saves a session that the CLI and MCP server can
use under the same Windows account.

### 1. Install the extension

1. Find `beyond-gate-browser.zip` in your Windows installation folder.
2. Extract it into a `beyond-gate-browser-extension` subfolder.
3. Open `edge://extensions` or `chrome://extensions` in your browser.
4. Enable **Developer mode**, select **Load unpacked**, and choose the extracted folder.
5. Check that the extension ID is `gnhdconfgjfdbbgkemadegjcjnnodefh`.
   Stop if it differs.

Keep `beyond-gate-native-host.exe` beside `beyond-gate.exe`.
There is no browser-store package yet.

### 2. Connect your browser to Beyond Gate

Open PowerShell, the Windows app for entering commands.
Use your permanent installation folder.
This example uses the folder from the Windows installation guide:

```powershell
$installation = Join-Path $env:LOCALAPPDATA 'Programs\Beyond Gate'
beyond-gate auth install edge --directory "$installation" --dry-run
```

Check the preview. Then run the setup and confirm the change:

```powershell
beyond-gate auth install edge --directory "$installation"
```

For Chrome, replace `edge` with `chrome` in both commands.
Do not register a temporary build or source folder.

### 3. Sign in

1. Sign in to [D&D Beyond](https://www.dndbeyond.com/) in your selected browser.
   Use a regular browser window, not a private window.
2. Run this command in an interactive terminal:

   ```sh
   beyond-gate auth login
   ```

3. Open the Beyond Gate extension from the browser toolbar.
4. Choose your browser in the **This browser** list.
   Enter the **One-use pairing key** from the terminal.
5. Read the consent text, select its checkbox, and select **Pair and save session**.
   This replaces any earlier saved Beyond Gate session.
6. Keep both windows open until the terminal confirms that the session was saved.

The extension reads one `CobaltSession` cookie after your approval.
It does not read your D&D Beyond password.
If pairing expires, run `beyond-gate auth login` again and use the new key.

## Use a supplied CobaltSession

If you already have a `CobaltSession` value, you can use it without the extension.
Enter just the value. Leave out `CobaltSession=`, a `Cookie:` header, spaces,
and line breaks.

### Save the session

Use `auth login` for each sign-in method. Without a method option, it uses
Windows browser pairing. To save a supplied value on Windows or Linux:

1. Open a terminal and run:

   ```sh
   beyond-gate auth login --prompt
   ```

2. Paste the supplied value into the hidden prompt.
   Nothing appears as you paste. This is normal.
3. Press **Ctrl+D** to submit. Do not press Enter.
   Press Escape or Ctrl+C to cancel.
4. Wait for validation and the protected save to finish.
   Unlock the OS credential store through its normal prompt if needed.

Use Ctrl+Shift+V or your terminal's Paste command.
Ctrl+V can send only a shortcut instead of the clipboard text.
Beyond Gate ignores that shortcut, but this does not paste the value.
The terminal must send the text; Beyond Gate does not read the clipboard directly.

Saving replaces any earlier Beyond Gate session.
After a successful save, you can use terminal commands or MCP without pasting
the value again.
On Linux, protected saving needs an available, unlocked Secret Service provider.
If you do not have one, use the method below.

If your caller has already supplied `BEYOND_GATE_COBALT_SESSION` in the
environment, you can validate and save that value without a terminal prompt:

```sh
beyond-gate auth login --from-env
```

`--from-env` reads that fixed variable. Do not put a value after the flag.
Select either `--prompt` or `--from-env`, not both.
Missing or invalid selected input fails without trying another source or browser
pairing. Environment presence alone does not change `auth login` without options.
Existing `auth import --prompt` and `auth import --env` commands keep the same
behavior for compatibility.

Do not pass a session value through `--auth-key` or any other command argument.
Arguments can appear in process listings and shell history, even when a shell
variable supplies the value. Use hidden input or the environment instead.

### Use the value without saving

To supply the value for one CLI command only:

```sh
beyond-gate --auth-prompt book list
```

Paste it into the hidden prompt and press **Ctrl+D**.
Use your terminal's paste command, as described above.
The value is used for this command only. You will need to enter it again for
the next command. The global `--auth-prompt` is only for resource reads and
`auth validate`. For saving login, use `auth login --prompt`.

For process-only MCP use, supply `BEYOND_GATE_COBALT_SESSION` in the server's
environment. Keep the value out of configuration files.
See [MCP startup settings](mcp.md#startup-and-lifecycle) for a configuration that
references an environment variable.

## Check your session

Use `auth status` to check the selected source locally. Use `--stored` to select
only the saved entry. See [source selection and status meanings](privacy.md#session-input-and-lifetime)
for precedence and failure behavior.

```sh
beyond-gate auth status
beyond-gate auth status --stored
```

To check the selected environment or saved session with D&D Beyond, run:

```sh
beyond-gate auth validate
```

Look for `"validation":"service_validated"` in the result.
This means Beyond Gate checked the session with D&D Beyond and it was accepted.
It does not prove access to every book or other resource.

The MCP server snapshots its selected environment at startup. Restart it after
changing its input. Process-only input is not saved and must be supplied again
after restart. See [environment and client limits](privacy.md#environment-input-and-mcp-clients).

## Sign out

To remove the saved Beyond Gate session from this computer:

```sh
beyond-gate auth logout --stored
```

This does not sign you out of the D&D Beyond website.
It does not revoke a session supplied through an environment variable.
See [logout and removal limits](privacy.md#status-logout-and-uncertain-saves)
to remove process-only input or explicitly select the saved entry.

For command options and error details, see the
[CLI authentication reference](cli.md#authentication-commands).
Contributors can read the [Authentication design](authentication-design.md)
under Development.
