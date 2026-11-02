/** Safe startup evidence for isolated regular-browser tests; never prints driver output. */
import { execFile } from "node:child_process";
import { access, readFile } from "node:fs/promises";
import { join } from "node:path";
import { release } from "node:os";
import { promisify } from "node:util";
import puppeteer from "puppeteer-core";

const execute = promisify(execFile);
const STARTUP_METHODS = new Set([
  "Browser.getVersion", "Browser.getBrowserCommandLine", "Browser.setDownloadBehavior",
  "Target.getBrowserContexts", "Target.setDiscoverTargets", "Target.setAutoAttach",
  "Target.getTargets", "Target.attachToTarget", "Target.createTarget",
  "Page.enable", "Page.getFrameTree", "Page.setLifecycleEventsEnabled",
  "Runtime.enable", "Network.enable", "Performance.enable", "Log.enable",
  "Emulation.setDeviceMetricsOverride", "Emulation.setTouchEmulationEnabled",
]);

/** Finds a regular installed browser without reading personal browser configuration. */
export async function executable(browser) {
  const roots = [process.env.ProgramFiles, process.env["ProgramFiles(x86)"]].filter(Boolean);
  const suffix = browser === "chrome"
    ? ["Google", "Chrome", "Application", "chrome.exe"]
    : ["Microsoft", "Edge", "Application", "msedge.exe"];
  for (const root of roots) {
    const path = join(root, ...suffix);
    try {
      await access(path);
      return path;
    } catch (error) {
      if (error.code !== "ENOENT") throw error;
    }
  }
  const error = new Error("required regular browser executable is missing");
  error.code = "ENOENT";
  throw error;
}

/** Reads executable version metadata without starting a browser or touching a profile. */
async function versions(path) {
  const { stdout } = await execute("pwsh", ["-NoProfile", "-Command",
    "$ErrorActionPreference = 'Stop'; " +
    "[System.Diagnostics.FileVersionInfo]::GetVersionInfo($env:BEYOND_GATE_TEST_EXECUTABLE).ProductVersion",
  ], {
    env: { ...process.env, BEYOND_GATE_TEST_EXECUTABLE: path },
    timeout: 15000, maxBuffer: 4096,
  });
  const browser = stdout.trim();
  if (!/^\d{1,4}(?:\.\d{1,6}){3}$/.test(browser)) {
    throw new Error("invalid browser version metadata");
  }
  const driver = JSON.parse(await readFile(
    new URL("./node_modules/puppeteer-core/package.json", import.meta.url), "utf8",
  )).version;
  if (!/^\d+\.\d+\.\d+$/.test(driver)) throw new Error("invalid driver version metadata");
  return { installed_browser_version: browser, driver_version: driver };
}

/** Classifies only known driver failures; arbitrary text, paths, and stderr never leave this function. */
export function safeLaunchCause(error) {
  const message = typeof error?.message === "string" ? error.message.slice(0, 8192) : "";
  const method = /^\(?([A-Za-z]+\.[A-Za-z]+)\)?(?: timed out|:)/.exec(message)?.[1] ??
    /^Protocol error \(([A-Za-z]+\.[A-Za-z]+)\):/.exec(message)?.[1];
  const kind = ["TimeoutError", "ProtocolError", "TargetCloseError", "ConnectionClosedError"].includes(error?.name)
    ? error.name : "Error";
  let reason = "unclassified";
  if (error?.code === "ENOENT") reason = "executable-missing";
  else if (error?.code === "EACCES" || error?.code === "EPERM") reason = "access-denied";
  else if (message.startsWith("The browser cannot write to ")) reason = "profile-not-writable";
  else if (message.startsWith("The browser is already running for ")) reason = "profile-in-use";
  else if (kind === "TimeoutError" || message.includes("timed out") ||
    message.startsWith("Timed out")) reason = "timed-out";
  else if (kind === "TargetCloseError" || kind === "ConnectionClosedError" || message.includes("Target closed") ||
    message.includes("Connection closed")) reason = "connection-closed";
  else if (message.startsWith("Failed to launch the browser process:")) reason = "process-launch-failed";
  else if (kind === "ProtocolError" || message.startsWith("Protocol error (")) reason = "protocol-error";
  const exit = /^Failed to launch the browser process:\s+Code: (-?\d{1,10})\b/.exec(message);
  return {
    cause_kind: kind, cause_reason: reason,
    ...(STARTUP_METHODS.has(method) ? { protocol_method: method } : {}),
    ...(exit ? { process_exit_code: Number(exit[1]) } : {}),
  };
}

/** Launches once with unchanged isolation/deadlines and attaches bounded safe failure metadata. */
export async function launchBrowser(name, options, dependencies = {}) {
  const {
    findExecutable = executable, inspect = versions,
    launch = (options) => puppeteer.launch(options),
    report = (record) => console.error(JSON.stringify(record)),
    now = () => performance.now(),
  } = dependencies;
  const started = now();
  const metadata = {
    browser: name, browser_flavor: "regular-installed",
    os_version: release(), node_version: process.version,
    headless: true, synthetic_only: true,
  };
  let stage = "executable";
  let processStarted = false;
  let processExited = false;
  const methods = [];
  let replies = 0;
  let recording = true;
  try {
    const path = await findExecutable(name);
    stage = "version-metadata";
    Object.assign(metadata, await inspect(path));
    stage = "launch";
    report({ status: "starting", ...metadata });
    return await launch({
      executablePath: path, userDataDir: join(options.directory, "profile"),
      pipe: true, headless: true, enableExtensions: true,
      timeout: 15000, protocolTimeout: options.protocolTimeout,
      // The pinned driver logs paths, arguments, and environment data. Never forward its arguments.
      logger: (category) => {
        if (category === "puppeteer:browsers:launcher") return (message) => {
          if (!recording || typeof message !== "string") return;
          if (/^Launched \d+$/.test(message)) processStarted = true;
          if (/^Browser process \d+ onExit$/.test(message)) processExited = true;
        };
        if (category.startsWith("puppeteer:protocol:SEND")) return (message) => {
          if (!recording || typeof message !== "string") return;
          const method = /^\{"method":"([A-Za-z]+\.[A-Za-z]+)"/.exec(message)?.[1];
          if (STARTUP_METHODS.has(method)) {
            methods.push(method);
            if (methods.length > 8) methods.shift();
          }
        };
        if (category.startsWith("puppeteer:protocol:RECV")) return () => {
          if (recording) replies++;
        };
        return undefined;
      },
      args: ["--no-first-run", "--disable-background-networking",
        `--proxy-server=http://127.0.0.1:${options.proxyPort}`, "--proxy-bypass-list=<-loopback>"],
    });
  } catch (cause) {
    const error = new Error("regular browser launch failed", { cause });
    error.launch = {
      ...metadata, launch_stage: stage, elapsed_ms: Math.max(0, Math.round(now() - started)),
      process_started: processStarted, process_exited: processExited,
      startup_protocol_methods: methods, protocol_messages_received: replies,
      ...safeLaunchCause(cause),
    };
    throw error;
  } finally {
    recording = false;
  }
}

/** Attempts every owned cleanup action and retains only fixed action names in diagnostics. */
export async function cleanupActions(actions) {
  const failures = [];
  const names = [];
  for (const [name, action] of actions) {
    try {
      await action();
    } catch (cause) {
      failures.push(cause);
      names.push(name);
    }
  }
  if (failures.length) {
    const error = new AggregateError(failures, "owned fixture cleanup failed");
    error.cleanup = names;
    throw error;
  }
}

/** Preserves the primary failure when owned-resource cleanup also fails. */
export async function withCleanup(operation, cleanup) {
  let failure;
  try {
    return await operation();
  } catch (error) {
    failure = error;
    throw error;
  } finally {
    try {
      await cleanup();
    } catch (error) {
      if (!failure) throw error;
      failure.cleanup = error.cleanup ?? ["owned-resources"];
    }
  }
}
