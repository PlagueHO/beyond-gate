/** Safe startup evidence for isolated regular-browser tests; never prints driver output. */
import { execFile } from "node:child_process";
import { channel } from "node:diagnostics_channel";
import { once } from "node:events";
import { access, open, readFile } from "node:fs/promises";
import { join } from "node:path";
import { release } from "node:os";
import { promisify } from "node:util";
import { fileURLToPath } from "node:url";
import puppeteer from "puppeteer-core";

const execute = promisify(execFile);
const CHILD_PROCESSES = channel("child_process");
const STARTUP_METHODS = new Set([
  "Browser.getVersion", "Browser.getBrowserCommandLine", "Browser.setDownloadBehavior",
  "Target.getBrowserContexts", "Target.setDiscoverTargets", "Target.setAutoAttach",
  "Target.getTargets", "Target.attachToTarget", "Target.createTarget",
  "Page.enable", "Page.getFrameTree", "Page.setLifecycleEventsEnabled",
  "Runtime.enable", "Network.enable", "Performance.enable", "Log.enable",
  "Emulation.setDeviceMetricsOverride", "Emulation.setTouchEmulationEnabled",
]);
const LOG_SOURCES = new Set([
  "chrome_main.cc", "chrome_main_delegate.cc", "chrome_browser_main.cc",
  "process_singleton_win.cc", "devtools_pipe_handler.cc", "devtools_http_handler.cc",
  "browser_main_loop.cc", "viz_main_impl.cc", "gpu_process_host.cc",
  "sandbox_win.cc", "network_service_instance_impl.cc", "headless_shell.cc",
  "main_dll_loader_win.cc", "chrome_exe_main_win.cc",
  "crashpad_client_win.cc", "crash_helper.cc", "chrome_crash_reporter_client_win.cc",
]);

/** Reduces an owned browser log to fixed error categories and known source names only. */
export function safeBrowserLog(text) {
  const sources = new Set();
  for (const match of text.matchAll(/[:\\/]([a-z_]+\.cc)(?:\(\d+\)|:\d+)\]/g)) {
    if (LOG_SOURCES.has(match[1])) sources.add(match[1]);
  }
  const reasons = [
    ["default-profile-debugging-denied", /remote debugging requires a non-default data directory/i],
    ["debugging-pipe-unavailable", /Remote debugging pipe file descriptors are not open/i],
    ["profile-singleton-failed", /Failed to create a ProcessSingleton/i],
    ["gpu-process-unusable", /GPU process isn't usable/i],
    ["gpu-process-launch-failed", /Failed to launch GPU process/i],
    ["browser-dll-load-failed", /Failed to load (?:Chrome DLL|chrome\.dll)/i],
    ["access-denied", /Access is denied/i],
  ].filter(([, pattern]) => pattern.test(text)).map(([reason]) => reason);
  return { startup_log_sources: [...sources], startup_log_reasons: reasons };
}

/** Reads fixed process counters and startup milestones for the owned browser only. */
async function startupProcess(child, path, directory) {
  if (child.exitCode !== null || child.signalCode !== null) return { status: "exited" };
  try {
    const { stdout } = await execute("pwsh", [
      "-NoProfile", "-File", fileURLToPath(new URL("./startup-process.ps1", import.meta.url)),
      "-BrowserProcessId", String(child.pid), "-BrowserExecutable", path,
      "-RunDirectory", directory,
    ], { timeout: 5000, maxBuffer: 16384, windowsHide: true });
    return JSON.parse(stdout);
  } catch (error) {
    return { status: "unreadable", ...safeLaunchCause(error) };
  }
}

/** Reports public module identities after the cases, never preloading a cold browser. */
export async function reportPublicBrowserSymbols(name, browser, directory, {
  enabled = process.env.BEYOND_GATE_STARTUP_STACKS === "1",
  inspect = startupProcess,
  report = (record) => console.error(JSON.stringify(record)),
} = {}) {
  if (!enabled) return;
  const child = browser.process();
  const state = child ? await inspect(child, child.spawnfile, directory) : { status: "not-observed" };
  report({
    status: state.status === "read" ? "public-build-identities" : "public-build-identities-unavailable",
    browser: name,
    phase: "after-browser-cases",
    inspection: state.status === "read"
      ? { status: "read", public_module_symbols: state.public_module_symbols }
      : state,
  });
}

/** Collects bounded public stack hints only in explicitly selected hosted sampling runs. */
async function startupStack(child, path, directory, { keepRunning = false } = {}) {
  if (process.env.BEYOND_GATE_STARTUP_STACKS !== "1") return { status: "not-requested" };
  if (child.exitCode !== null || child.signalCode !== null) return { status: "exited" };
  try {
    const { stdout } = await execute("pwsh", [
      "-NoProfile", "-File", fileURLToPath(new URL("./startup-stack.ps1", import.meta.url)),
      "-BrowserProcessId", String(child.pid), "-BrowserExecutable", path,
      "-RunDirectory", directory, "-DeadlineUnixMs", String(Date.now() + 6000),
      ...(keepRunning ? ["-KeepRunning"] : []),
    ], { timeout: 8000, maxBuffer: 8192, windowsHide: true });
    return JSON.parse(stdout);
  } catch (error) {
    return { status: "unreadable", ...safeLaunchCause(error) };
  }
}

/** Observes only the exact owned launch; never forwards raw stderr or process details. */
export function observeStartup(path, directory, inspect = startupProcess, inspectStack = startupStack) {
  const pending = new Map();
  const buffer = Buffer.alloc(16384);
  let bytes = 0;
  let owned;
  const onData = (chunk) => {
    const data = Buffer.isBuffer(chunk) ? chunk : Buffer.from(chunk);
    bytes += data.copy(buffer, bytes, 0, Math.min(data.length, buffer.length - bytes));
  };
  const onConstruct = ({ process: child }) => {
    const onSpawn = () => {
      pending.delete(child);
      if (child.spawnfile !== path ||
        !child.spawnargs.includes(`--user-data-dir=${join(directory, "profile")}`)) return;
      owned = child;
      child.stderr?.on("data", onData);
    };
    pending.set(child, onSpawn);
    child.once("spawn", onSpawn);
  };
  CHILD_PROCESSES.subscribe(onConstruct);
  return {
    /** Returns bounded categories and fixed state during pending or failed startup. */
    async read({ keepRunning = false } = {}) {
      const log = safeBrowserLog(buffer.subarray(0, bytes).toString("utf8"));
      const write = owned?.stdio?.[3];
      const read = owned?.stdio?.[4];
      const pipe = write && read ? {
        status: "observed",
        written_bytes: write.bytesWritten, write_buffered_bytes: write.writableLength,
        write_destroyed: write.destroyed, write_ended: write.writableEnded,
        received_bytes: read.bytesRead, read_buffered_bytes: read.readableLength,
        read_destroyed: read.destroyed, read_ended: read.readableEnded,
      } : { status: "not-observed" };
      const processState = owned ? await inspect(owned, path, directory) : { status: "not-observed" };
      const stack = owned ? await inspectStack(owned, path, directory, { keepRunning }) :
        { status: "not-observed" };
      return {
        startup_stderr_bytes_read: bytes,
        startup_stderr_limit_reached: bytes === buffer.length,
        startup_stderr_sources: log.startup_log_sources,
        startup_stderr_reasons: log.startup_log_reasons,
        startup_pipe: pipe,
        startup_process: processState,
        startup_stack: stack,
      };
    },
    /** Waits for the owned aborted process before its profile can be removed. */
    async waitForExit() {
      if (!owned || owned.exitCode !== null || owned.signalCode !== null) return;
      await once(owned, "exit", { signal: AbortSignal.timeout(5000) });
    },
    /** Releases constructor, spawn, and stderr listeners on every result. */
    stop() {
      CHILD_PROCESSES.unsubscribe(onConstruct);
      for (const [child, handler] of pending) child.off("spawn", handler);
      pending.clear();
      owned?.stderr?.off("data", onData);
    },
  };
}

/** Reads at most 16 KiB from this launch's owned log; never returns its raw contents. */
async function browserLog(directory) {
  try {
    const file = await open(join(directory, "startup.log"), "r");
    try {
      const buffer = Buffer.alloc(16384);
      const { bytesRead } = await file.read(buffer, 0, buffer.length, 0);
      return {
        startup_log: "read", startup_log_bytes_read: bytesRead,
        startup_log_limit_reached: bytesRead === buffer.length,
        ...safeBrowserLog(buffer.subarray(0, bytesRead).toString("utf8")),
      };
    } finally {
      await file.close();
    }
  } catch (error) {
    if (error.code === "ENOENT") return { startup_log: "not-created" };
    return { startup_log: "unreadable", startup_log_error: safeLaunchCause(error) };
  }
}

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
    observe = observeStartup,
    report = (record) => console.error(JSON.stringify(record)),
    now = () => performance.now(),
    earlyDiagnostics = process.env.BEYOND_GATE_STARTUP_STACKS === "1",
  } = dependencies;
  const started = now();
  const controller = new AbortController();
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
  let driverCloseRequested = false;
  let recording = true;
  let observer;
  let earlyTimer;
  let earlyObservation;
  /** Settles typed inspection failures without replacing the browser's launch result. */
  const inspectStartup = (keepRunning = false) => {
    const began = Math.max(0, Math.round(now() - started));
    return Promise.allSettled([Promise.resolve().then(() => observer.read({ keepRunning }))
      .then((value) => keepRunning ? {
        ...value,
        startup_snapshot: {
          phase: "pending-launch", started_ms: began,
          completed_ms: Math.max(0, Math.round(now() - started)),
        },
      } : value)]);
  };
  try {
    const path = await findExecutable(name);
    stage = "version-metadata";
    Object.assign(metadata, await inspect(path));
    stage = "launch";
    report({ status: "starting", ...metadata });
    observer = observe(path, options.directory);
    if (earlyDiagnostics) {
      earlyTimer = setTimeout(() => {
        if (replies === 0) earlyObservation = inspectStartup(true);
      }, 10000);
    }
    const browser = await launch({
      executablePath: path, userDataDir: join(options.directory, "profile"),
      pipe: true, headless: true, enableExtensions: true,
      timeout: 15000, protocolTimeout: options.protocolTimeout,
      signal: controller.signal,
      env: { ...process.env, CHROME_LOG_FILE: join(options.directory, "startup.log") },
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
          if (method === "Browser.close") driverCloseRequested = true;
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
      args: ["--no-first-run", "--disable-background-networking", "--enable-logging",
        `--proxy-server=http://127.0.0.1:${options.proxyPort}`, "--proxy-bypass-list=<-loopback>"],
    });
    clearTimeout(earlyTimer);
    if (earlyObservation) {
      const [inspection] = await earlyObservation;
      report(inspection.status === "fulfilled" ? {
        status: "startup-observation", browser: name, ...inspection.value,
      } : {
        status: "startup-observation-failed", browser: name, ...safeLaunchCause(inspection.reason),
      });
    }
    return browser;
  } catch (cause) {
    clearTimeout(earlyTimer);
    const error = new Error("regular browser launch failed", { cause });
    error.launch = {
      ...metadata, launch_stage: stage, elapsed_ms: Math.max(0, Math.round(now() - started)),
      process_started: processStarted, process_exited: processExited,
      startup_protocol_methods: [...methods], protocol_messages_received: replies,
      ...(driverCloseRequested ? { driver_close_requested: true } : {}),
      ...safeLaunchCause(cause),
      ...(stage === "launch" ? await browserLog(options.directory) : {}),
    };
    if (observer) {
      const [inspection] = await (earlyObservation ?? inspectStartup());
      if (inspection.status === "fulfilled") {
        Object.assign(error.launch, inspection.value);
      } else {
        error.observationCause = inspection.reason;
        error.launch.startup_inspection_error = safeLaunchCause(inspection.reason);
      }
    }
    if (replies > error.launch.protocol_messages_received) {
      error.launch.late_protocol_replies = replies - error.launch.protocol_messages_received;
    }
    if (processExited && !error.launch.process_exited) {
      error.launch.exited_during_inspection = true;
    }
    if (observer) {
      try {
        controller.abort();
        await observer.waitForExit();
        error.launch.startup_cleanup = "passed";
      } catch (cleanup) {
        error.cleanup = ["browser-process"];
        error.cleanupCause = cleanup;
        error.launch.startup_cleanup = "failed";
        error.launch.startup_cleanup_error = safeLaunchCause(cleanup);
      }
    }
    throw error;
  } finally {
    clearTimeout(earlyTimer);
    recording = false;
    observer?.stop();
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
      failure.cleanup = [...new Set([
        ...(failure.cleanup ?? []), ...(error.cleanup ?? ["owned-resources"]),
      ])];
    }
  }
}
