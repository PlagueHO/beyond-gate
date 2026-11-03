/** Offline checks for the development-only extension's narrow synthetic scope. */
import assert from "node:assert/strict";
import { mkdir, mkdtemp, readFile, rm, stat, writeFile } from "node:fs/promises";
import { test } from "node:test";
import { channel } from "node:diagnostics_channel";
import { EventEmitter, once } from "node:events";
import { execFile, spawn } from "node:child_process";
import { createHash } from "node:crypto";
import { PassThrough } from "node:stream";
import { checkSyntheticCookie, syntheticHandoff, SYNTHETIC } from "./extension/service_worker.js";
import {
  launchBrowser, reportPublicBrowserSymbols, safeLaunchCause, safeBrowserLog,
  observeStartup, cleanupActions, withCleanup,
} from "./browser-launch.mjs";
import { join } from "node:path";
import { promisify } from "node:util";
import { fileURLToPath } from "node:url";
import puppeteer from "puppeteer-core";
import { cleanupProbe, TEST_ROOT } from "./run.mjs";

/** Builds invented metadata, not a real browser or account snapshot. */
function cookie() {
  return {
    name: SYNTHETIC.name,
    value: SYNTHETIC.value,
    domain: SYNTHETIC.domain,
    path: "/",
    hostOnly: true,
    secure: true,
    httpOnly: true,
    sameSite: "lax",
    session: true,
  };
}

test("only the invented cookie scope has permission", async () => {
  const manifest = JSON.parse(
    await readFile(new URL("./extension/manifest.json", import.meta.url), "utf8"),
  );
  assert.deepEqual(manifest.host_permissions, [`${SYNTHETIC.url}*`]);
  assert.deepEqual(manifest.permissions, ["cookies", "nativeMessaging"]);
  assert.equal(manifest.content_scripts, undefined);
});

test("exact synthetic metadata passes without general cookie selection", () => {
  const input = cookie();
  assert.equal(checkSyntheticCookie([input]), input);
});

test("missing and ambiguous cookies fail", () => {
  assert.throws(() => checkSyntheticCookie([]));
  assert.throws(() => checkSyntheticCookie([cookie(), cookie()]));
});

test("wrong scope, flags, material, and partitioning fail", () => {
  for (const change of [
    { name: "OtherSession" },
    { domain: "example.invalid" },
    { path: "/other" },
    { hostOnly: false },
    { secure: false },
    { httpOnly: false },
    { sameSite: "none" },
    { session: false },
    { value: "synthetic-other" },
    { partitionKey: { topLevelSite: "https://example.invalid" } },
  ]) {
    assert.throws(() => checkSyntheticCookie([{ ...cookie(), ...change }]));
  }
});

/** Uses listener sets to test the worker without starting a browser or native process. */
function event() {
  const listeners = new Set();
  return {
    addListener: (listener) => listeners.add(listener),
    removeListener: (listener) => listeners.delete(listener),
    emit: (value) => Promise.all([...listeners].map((listener) => listener(value))),
  };
}

/** Captures invented frames and cookie-query order, not external browser data. */
function nativeApi() {
  const messages = [];
  const queries = [];
  const port = {
    onMessage: event(), onDisconnect: event(),
    postMessage: (message) => messages.push(message),
    disconnect() {},
  };
  return {
    messages, queries, port,
    runtime: { connectNative: () => port },
    cookies: { getAll: async (query) => { queries.push(query); return [cookie()]; } },
  };
}

const HOST = `com.beyondgate.synthetic.${"a".repeat(32)}`;
const TRANSACTION = "1".repeat(32);

/** Builds the exact proposed synthetic pending reply. */
function pending() {
  return {
    protocol: "bg-browser-handoff", version: 1, type: "pending",
    transaction: TRANSACTION, profile: "synthetic-v1", expires_in_ms: 10000,
  };
}

test("consent is required before connecting or querying cookies", async () => {
  const api = {
    runtime: { connectNative() { assert.fail("must not connect"); } },
    cookies: { getAll() { assert.fail("must not query"); } },
  };
  const result = await syntheticHandoff(api, HOST, "chrome", false);
  assert.equal(result.code, "consent_required");
  assert.equal(result.saved, false);
});

test("a pending reply gates one query and submit; receipt is not success", async () => {
  const api = nativeApi();
  const result = syntheticHandoff(api, HOST, "chrome", true);
  assert.equal(api.queries.length, 0);
  assert.equal(api.messages.length, 1);
  await api.port.onMessage.emit(pending());
  assert.deepEqual(api.queries, [{ name: SYNTHETIC.name, domain: SYNTHETIC.domain }]);
  assert.equal(api.messages[1].transaction, TRANSACTION);
  await api.port.onMessage.emit({
    protocol: "bg-browser-handoff", version: 1, type: "error",
    transaction: TRANSACTION, code: "unsupported_contract",
  });
  assert.deepEqual(await result, {
    code: "unsupported_contract", cookie_reads: 1, received: true,
    validated: false, saved: false,
  });
});

test("no pending authorization does not permit a cookie query", async () => {
  const api = nativeApi();
  const result = syntheticHandoff(api, HOST, "edge", true);
  await api.port.onMessage.emit({
    protocol: "bg-browser-handoff", version: 1, type: "error",
    transaction: null, code: "no_pending_login",
  });
  assert.equal((await result).code, "no_pending_login");
  assert.equal(api.queries.length, 0);
});

test("malformed authorization and false success messages fail safely", async () => {
  for (const message of [
    null, { ...pending(), extra: true }, { ...pending(), version: 2 },
    { ...pending(), expires_in_ms: 0 }, { ...pending(), profile: "production" },
    { ...pending(), transaction: null },
    { protocol: "bg-browser-handoff", version: 1, type: "result",
      transaction: TRANSACTION, saved: true },
  ]) {
    const api = nativeApi();
    const result = syntheticHandoff(api, HOST, "chrome", true);
    await api.port.onMessage.emit(message);
    assert.equal((await result).code, "protocol_error");
    assert.equal(api.queries.length, 0);
  }
});

test("a wrong result transaction cannot confirm receipt", async () => {
  const api = nativeApi();
  const result = syntheticHandoff(api, HOST, "chrome", true);
  await api.port.onMessage.emit(pending());
  await api.port.onMessage.emit({
    protocol: "bg-browser-handoff", version: 1, type: "error",
    transaction: "2".repeat(32), code: "unsupported_contract",
  });
  assert.equal((await result).received, false);
});

test("disconnect after submit remains indeterminate, never saved", async () => {
  const api = nativeApi();
  const result = syntheticHandoff(api, HOST, "chrome", true);
  await api.port.onMessage.emit(pending());
  await api.port.onDisconnect.emit();
  assert.equal((await result).code, "save_indeterminate");
  assert.equal((await result).saved, false);
});

test("cookie permission failure does not submit or expose the browser error", async () => {
  const api = nativeApi();
  api.cookies.getAll = async () => { throw new Error("synthetic private diagnostic"); };
  const result = syntheticHandoff(api, HOST, "chrome", true);
  await api.port.onMessage.emit(pending());
  assert.equal((await result).code, "cookie_unavailable");
  assert.equal(api.messages.length, 1);
});

test("pending expiry and the cookie-query budget reject late progress", async (context) => {
  let clock = 0;
  context.mock.method(performance, "now", () => clock);
  for (const scenario of [
    { expires: 1, elapsed: 2 },
    { expires: 100000, elapsed: 15001 },
  ]) {
    clock = 0;
    const api = nativeApi();
    api.cookies.getAll = async () => { clock = scenario.elapsed; return [cookie()]; };
    const result = syntheticHandoff(api, HOST, "chrome", true);
    await api.port.onMessage.emit({ ...pending(), expires_in_ms: scenario.expires });
    assert.equal((await result).code, "timed_out");
    assert.equal(api.messages.length, 1);
  }
});

test("duplicate pending authorization stops an in-flight query without a submit", async () => {
  const api = nativeApi();
  const deferred = Promise.withResolvers();
  api.cookies.getAll = () => deferred.promise;
  const result = syntheticHandoff(api, HOST, "chrome", true);
  const first = api.port.onMessage.emit(pending());
  await api.port.onMessage.emit(pending());
  deferred.resolve([cookie()]);
  await first;
  assert.equal((await result).code, "protocol_error");
  assert.equal(api.messages.length, 1);
});

test("general host names and unknown browsers are rejected before connection", () => {
  const api = { runtime: { connectNative() { assert.fail("must not connect"); } } };
  assert.throws(() => syntheticHandoff(api, "com.beyondgate.product", "chrome", true));
  assert.throws(() => syntheticHandoff(api, HOST, "unknown", true));
});

/** Provides public invented version metadata without reading an installed browser. */
function launchDependencies(launch, records = []) {
  return {
    findExecutable: async () => "C:\\synthetic-private\\browser.exe",
    inspect: async () => ({ installed_browser_version: "156.0.8078.48", driver_version: "25.16.0" }),
    launch, report: (record) => records.push(record), now: () => 10,
  };
}

test("both browser runners retain launch isolation and their exact protocol deadlines", async () => {
  for (const [name, protocolTimeout] of [
    ["chrome", 15000], ["edge", 15000], ["chrome", 35000], ["edge", 35000],
  ]) {
    const records = [];
    const browser = {};
    let calls = 0;
    const result = await launchBrowser(name, {
      directory: "synthetic-owned-directory", proxyPort: 12345, protocolTimeout,
    }, launchDependencies(async (options) => {
      calls++;
      assert.equal(options.executablePath, "C:\\synthetic-private\\browser.exe");
      assert.equal(options.userDataDir, join("synthetic-owned-directory", "profile"));
      assert.equal(options.pipe, true);
      assert.equal(options.headless, true);
      assert.equal(options.enableExtensions, true);
      assert.equal(options.timeout, 15000);
      assert.equal(options.protocolTimeout, protocolTimeout);
      assert.equal(options.signal.aborted, false);
      assert.equal(options.dumpio, undefined);
      assert.equal(options.env.CHROME_LOG_FILE, join("synthetic-owned-directory", "startup.log"));
      assert.deepEqual(options.args, [
        "--no-first-run", "--disable-background-networking", "--enable-logging",
        "--proxy-server=http://127.0.0.1:12345", "--proxy-bypass-list=<-loopback>",
      ]);
      return browser;
    }, records));
    assert.equal(result, browser);
    assert.equal(calls, 1);
    assert.equal(records.length, 1);
    assert.equal(records[0].status, "starting");
    assert.equal(records[0].browser, name);
    assert.equal(records[0].installed_browser_version, "156.0.8078.48");
    assert.equal(JSON.stringify(records).includes("synthetic-private"), false);
  }
});

test("launch rejection retains safe cause and process evidence but never driver text", async () => {
  const cause = new Error(
    "Protocol error (Target.setDiscoverTargets): Target closed; " +
    "C:\\synthetic-private\\profile https://private.invalid/?token=synthetic-secret\u001b[31m",
  );
  cause.name = "TargetCloseError";
  cause.stderr = "synthetic-secret".repeat(10000);
  cause.env = { SECRET: "synthetic-secret" };
  await assert.rejects(launchBrowser("chrome", {
    directory: "synthetic-owned-directory", proxyPort: 12345, protocolTimeout: 15000,
  }, launchDependencies(async (options) => {
    const log = options.logger("puppeteer:browsers:launcher");
    log("Launching C:\\synthetic-private\\browser.exe", { env: cause.env });
    log("Launched 1234");
    log("Browser process 1234 onExit");
    const send = options.logger("puppeteer:protocol:SEND");
    for (let index = 0; index < 20; index++) {
      send('{"method":"Target.setDiscoverTargets","params":{"private":"synthetic-secret"}}');
    }
    send('{"method":"Private.secret","params":{"private":"synthetic-secret"}}');
    send("synthetic-secret");
    options.logger("puppeteer:protocol:RECV")('{"private":"synthetic-secret"}');
    throw cause;
  })), (error) => {
    assert.equal(error.cause, cause);
    assert.equal(error.launch.cause_kind, "TargetCloseError");
    assert.equal(error.launch.cause_reason, "connection-closed");
    assert.equal(error.launch.protocol_method, "Target.setDiscoverTargets");
    assert.equal(error.launch.process_started, true);
    assert.equal(error.launch.process_exited, true);
    assert.deepEqual(error.launch.startup_protocol_methods, Array(8).fill("Target.setDiscoverTargets"));
    assert.equal(error.launch.protocol_messages_received, 1);
    assert.equal(error.launch.elapsed_ms, 0);
    assert.equal(error.launch.startup_log, "not-created");
    const output = JSON.stringify(error.launch);
    for (const privateText of ["synthetic-secret", "synthetic-private", "private.invalid", "\\u001b"]) {
      assert.equal(output.includes(privateText), false);
    }
    assert.ok(output.length < 1024);
    return true;
  });

});

test("late replies and shutdown keep the original failed startup snapshot", async () => {
  let options;
  const cause = new Error("Target.setDiscoverTargets timed out");
  cause.name = "ProtocolError";
  const dependencies = launchDependencies(async (value) => {
    options = value;
    options.logger("puppeteer:browsers:launcher")("Launched 1234");
    options.logger("puppeteer:protocol:SEND")('{"method":"Target.setDiscoverTargets"}');
    throw cause;
  });
  dependencies.observe = () => ({
    read: async () => {
      options.logger("puppeteer:protocol:SEND")('{"method":"Browser.close"}');
      options.logger("puppeteer:protocol:SEND")('{"method":"Target.setAutoAttach"}');
      options.logger("puppeteer:protocol:RECV")('{"private":"synthetic-secret"}');
      options.logger("puppeteer:browsers:launcher")("Browser process 1234 onExit");
      return {};
    },
    waitForExit: async () => {},
    stop: () => {},
  });
  await assert.rejects(launchBrowser("chrome", {
    directory: "synthetic-owned-directory", proxyPort: 12345, protocolTimeout: 15000,
  }, dependencies), (error) => {
    assert.equal(error.cause, cause);
    assert.deepEqual(error.launch.startup_protocol_methods, ["Target.setDiscoverTargets"]);
    assert.equal(error.launch.protocol_messages_received, 0);
    assert.equal(error.launch.late_protocol_replies, 1);
    assert.equal(error.launch.process_exited, false);
    assert.equal(error.launch.exited_during_inspection, true);
    assert.equal(error.launch.driver_close_requested, undefined);
    assert.equal(JSON.stringify(error.launch).includes("synthetic-secret"), false);
    return true;
  });
});

test("manual early evidence precedes driver shutdown without changing the failed deadline", async (context) => {
  context.mock.timers.enable({ apis: ["setTimeout"] });
  const entered = Promise.withResolvers();
  const inspected = Promise.withResolvers();
  const completed = Promise.withResolvers();
  const cause = Object.assign(new Error("Target.setDiscoverTargets timed out"), { name: "ProtocolError" });
  let clock = 0;
  let reads = 0;
  let stopped = 0;
  let closed = false;
  const dependencies = launchDependencies((options) => {
    options.logger("puppeteer:browsers:launcher")("Launched 1234");
    options.logger("puppeteer:protocol:SEND")('{"method":"Target.setDiscoverTargets"}');
    entered.resolve(options);
    return completed.promise;
  });
  dependencies.earlyDiagnostics = true;
  dependencies.now = () => clock;
  dependencies.observe = () => ({
    read: async ({ keepRunning }) => {
      reads++;
      assert.equal(keepRunning, true);
      assert.equal(closed, false);
      inspected.resolve();
      return { startup_stack: { status: "read", capture_mode: "non-suspending" } };
    },
    waitForExit: async () => assert.equal(closed, true),
    stop: () => { stopped++; },
  });
  const pending = launchBrowser("chrome", {
    directory: "synthetic-owned-directory", proxyPort: 12345, protocolTimeout: 15000,
  }, dependencies);
  const options = await entered.promise;
  assert.equal(options.timeout, 15000);
  assert.equal(options.protocolTimeout, 15000);
  clock = 9999;
  context.mock.timers.tick(9999);
  assert.equal(reads, 0);
  clock = 10000;
  context.mock.timers.tick(1);
  await inspected.promise;
  await new Promise(setImmediate);
  clock = 15000;
  closed = true;
  options.logger("puppeteer:protocol:SEND")('{"method":"Browser.close"}');
  completed.reject(cause);
  await assert.rejects(pending, (error) => {
    assert.equal(error.cause, cause);
    assert.equal(error.launch.elapsed_ms, 15000);
    assert.equal(error.launch.driver_close_requested, true);
    assert.equal(error.launch.protocol_messages_received, 0);
    assert.deepEqual(error.launch.startup_protocol_methods, ["Target.setDiscoverTargets"]);
    assert.deepEqual(error.launch.startup_snapshot, {
      phase: "pending-launch", started_ms: 10000, completed_ms: 10000,
    });
    assert.equal(error.launch.startup_stack.capture_mode, "non-suspending");
    assert.equal(error.launch.startup_cleanup, "passed");
    return true;
  });
  context.mock.timers.tick(30000);
  assert.equal(reads, 1);
  assert.equal(stopped, 1);
});

for (const replied of [false, true]) {
  test(`early inspection is ${replied ? "skipped after a reply" : "canceled on a fast success"}`, async (context) => {
    context.mock.timers.enable({ apis: ["setTimeout"] });
    const entered = Promise.withResolvers();
    const completed = Promise.withResolvers();
    const browser = {};
    let stopped = 0;
    const dependencies = launchDependencies((options) => {
      if (replied) options.logger("puppeteer:protocol:RECV")('{"private":"synthetic-secret"}');
      entered.resolve();
      return completed.promise;
    });
    dependencies.earlyDiagnostics = true;
    dependencies.observe = () => ({
      read: () => assert.fail("must not inspect a completed or responsive launch"),
      stop: () => { stopped++; },
    });
    const pending = launchBrowser("chrome", {
      directory: "synthetic-owned-directory", proxyPort: 12345, protocolTimeout: 15000,
    }, dependencies);
    await entered.promise;
    context.mock.timers.tick(replied ? 10000 : 9999);
    completed.resolve(browser);
    assert.equal(await pending, browser);
    context.mock.timers.tick(30000);
    assert.equal(stopped, 1);
  });
}

for (const outcome of ["success", "observation-error", "launch-error"]) {
  test(`pending early inspection joins before its ${outcome} result`, async (context) => {
    context.mock.timers.enable({ apis: ["setTimeout"] });
    const entered = Promise.withResolvers();
    const inspecting = Promise.withResolvers();
    const captured = Promise.withResolvers();
    const completed = Promise.withResolvers();
    const browser = {};
    const cause = Object.assign(new Error("Target.setDiscoverTargets timed out"), { name: "ProtocolError" });
    const observationCause = new Error("synthetic-secret C:\\synthetic-private");
    const records = [];
    const failed = outcome === "launch-error";
    let finished = false;
    let settled = false;
    let reads = 0;
    const dependencies = launchDependencies(() => {
      entered.resolve();
      return completed.promise;
    }, records);
    dependencies.earlyDiagnostics = true;
    dependencies.observe = () => ({
      read: () => {
        reads++;
        inspecting.resolve();
        return captured.promise;
      },
      waitForExit: async () => assert.equal(finished, true),
      stop: () => assert.equal(finished, true),
    });
    const pending = launchBrowser("chrome", {
      directory: "synthetic-owned-directory", proxyPort: 12345, protocolTimeout: 15000,
    }, dependencies);
    pending.then(() => { settled = true; }, () => { settled = true; });
    await entered.promise;
    context.mock.timers.tick(10000);
    await inspecting.promise;
    if (failed) completed.reject(cause);
    else completed.resolve(browser);
    await new Promise(setImmediate);
    assert.equal(settled, false);
    finished = true;
    if (outcome === "success") {
      captured.resolve({ startup_stack: { status: "read", capture_mode: "non-suspending" } });
    } else {
      captured.reject(observationCause);
    }
    if (failed) {
      await assert.rejects(pending, (error) => {
        assert.equal(error.cause, cause);
        assert.equal(error.observationCause, observationCause);
        assert.equal(error.launch.startup_inspection_error.cause_kind, "Error");
        assert.equal(error.launch.startup_cleanup, "passed");
        assert.equal(JSON.stringify(error.launch).includes("synthetic-"), false);
        return true;
      });
    } else {
      assert.equal(await pending, browser);
      assert.equal(records[1].status, outcome === "success" ?
        "startup-observation" : "startup-observation-failed");
      if (outcome === "success") assert.equal(records[1].startup_snapshot.phase, "pending-launch");
      assert.equal(JSON.stringify(records).includes("synthetic-"), false);
    }
    assert.equal(reads, 1);
  });
}

test("owned startup logs expose only known source names and fixed cause categories", () => {
  const result = safeBrowserLog(
    "[1:2:timestamp:ERROR:devtools_pipe_handler.cc(12)] " +
    "Remote debugging pipe file descriptors are not open. synthetic-secret\n" +
    "[1:2:timestamp:ERROR:chrome\\app\\main_dll_loader_win.cc:42] " +
    "Failed to load Chrome DLL from C:\\synthetic-private\n" +
    "[1:2:timestamp:ERROR:synthetic_secret.cc(99)] " +
    "https://private.invalid C:\\synthetic-private synthetic-secret\n",
  );
  assert.deepEqual(result, {
    startup_log_sources: ["devtools_pipe_handler.cc", "main_dll_loader_win.cc"],
    startup_log_reasons: ["debugging-pipe-unavailable", "browser-dll-load-failed"],
  });
  assert.equal(JSON.stringify(result).includes("synthetic"), false);
  assert.equal(JSON.stringify(result).includes("private"), false);
});

test("startup observation waits for spawn and ignores foreign executable or profile", async () => {
  const directory = join(TEST_ROOT, "chrome-observed");
  const inspected = [];
  const observer = observeStartup("owned.exe", directory,
    async (child) => {
      inspected.push(child);
      child.stdio[3].bytesWritten = 128;
      return { status: "read", chrome_dll_loaded: false };
    },
    async (child, path, ownedDirectory, options) => {
      assert.equal(child, inspected[0]);
      assert.equal(path, "owned.exe");
      assert.equal(ownedDirectory, directory);
      assert.deepEqual(options, { keepRunning: true });
      return { status: "read", capture_mode: "non-suspending" };
    });
  const children = [];
  try {
    for (const [file, profile] of [
      ["foreign.exe", join(directory, "profile")],
      ["owned.exe", "foreign-profile"],
      ["owned.exe", join(directory, "profile")],
    ]) {
      const child = new EventEmitter();
      child.stderr = new PassThrough();
      child.stdio = [undefined, undefined, child.stderr,
        { bytesWritten: 64, writableLength: 64, destroyed: false, writableEnded: false },
        { bytesRead: 0, readableLength: 0, destroyed: false, readableEnded: false }];
      children.push(child);
      channel("child_process").publish({ process: child });
      child.spawnfile = file;
      child.spawnargs = [file, `--user-data-dir=${profile}`];
      child.emit("spawn");
      child.stderr.write(
        "Remote debugging pipe file descriptors are not open. synthetic-secret\n" +
        "synthetic-private".repeat(2000) + "\nAccess is denied.\n",
      );
    }
    const result = await observer.read({ keepRunning: true });
    assert.deepEqual(inspected, [children[2]]);
    assert.equal(result.startup_stderr_bytes_read, 16384);
    assert.equal(result.startup_stderr_limit_reached, true);
    assert.deepEqual(result.startup_stderr_reasons, ["debugging-pipe-unavailable"]);
    assert.deepEqual(result.startup_process, { status: "read", chrome_dll_loaded: false });
    assert.deepEqual(result.startup_pipe, {
      status: "observed", written_bytes: 64, write_buffered_bytes: 64,
      write_destroyed: false, write_ended: false, received_bytes: 0,
      read_buffered_bytes: 0, read_destroyed: false, read_ended: false,
    });
    assert.equal(JSON.stringify(result).includes("synthetic"), false);
    assert.equal(JSON.stringify(result).includes("foreign"), false);
  } finally {
    observer.stop();
  }
  for (const child of children) assert.equal(child.stderr.listenerCount("data"), 0);
});

test("startup observer removes pending spawn handlers and reports absent observation", async () => {
  const observer = observeStartup("owned.exe", TEST_ROOT,
    () => assert.fail("must not inspect a process that did not spawn"));
  const child = new EventEmitter();
  channel("child_process").publish({ process: child });
  assert.equal(child.listenerCount("spawn"), 1);
  observer.stop();
  assert.equal(child.listenerCount("spawn"), 0);
  assert.equal((await observer.read()).startup_process.status, "not-observed");
});

test("Windows stack filtering emits only allowed modules and public export hints", {
  skip: process.platform !== "win32", timeout: 10000,
}, async () => {
  const text = [
    "synthetic-private C:\\synthetic-private https://private.invalid",
    "Preparing the environment for Debugger Extensions Gallery repositories",
    "Cannot debug pid 1234: Win32 error 0n87",
    "BG_STACK_BEGIN",
    "00 00000001`00000002 00000003`00000004 ntdll!NtWaitForSingleObject+0x14",
    "01 00000001`00000002 00000003`00000004 KERNELBASE!WaitForSingleObjectEx+0x99",
    "02 00000001`00000002 00000003`00000004 chrome!synthetic_private+0x99",
    "03 00000001`00000002 00000003`00000004 foreign!ReadFile+0x99",
    "04 00000001`00000002 00000003`00000004 chrome+0x1234",
    "05 00000001`00000002 00000003`00000004 ntdll!LdrpDrainWorkQueue+0x12",
    "06 00000001`00000002 00000003`00000004 ntdll!NtSetInformationVirtualMemory+0x12",
    "07 00000001`00000002 00000003`00000004 KERNELBASE!PrefetchVirtualMemory+0x12",
    "08 00000001`00000002 00000003`00000004 chrome_exe!base::PreReadFile+0x12",
    "09 00000001`00000002 00000003`00000004 chrome_elf!third_party_dlls::Init+0x12",
    "0a 00000001`00000002 00000003`00000004 chrome_elf!synthetic_private::function<private value>+0x99",
    "BG_STACK_END",
    "05 00000001`00000002 00000003`00000004 crypt32!CryptQueryObject+0x99",
  ].join("\r\n");
  const { stdout } = await promisify(execFile)("pwsh", ["-NoProfile", "-Command",
    ". $env:BEYOND_GATE_TEST_STACK_SCRIPT -BrowserProcessId 1 -BrowserExecutable fake -RunDirectory fake; " +
    "@((ConvertTo-SafeStartupStack $env:BEYOND_GATE_TEST_STACK), " +
    "(ConvertTo-SafeStartupStack 'synthetic-private'), " +
    "(Get-SafeProbeProgress $env:BEYOND_GATE_TEST_STACK), " +
    "@((ConvertTo-SafeStartupStack $env:BEYOND_GATE_TEST_LONG_STACK).frame_count, " +
    "(ConvertTo-SafeStartupStack $env:BEYOND_GATE_TEST_LONG_STACK -MaxFrames 8).frame_count)) " +
    "| ConvertTo-Json -Depth 4 -Compress",
  ], {
    env: { ...process.env, BEYOND_GATE_TEST_STACK: text,
      BEYOND_GATE_TEST_LONG_STACK: [
        "BG_STACK_BEGIN",
        ...Array.from({ length: 40 }, () =>
          "00 0000000100000002 0000000300000004 ntdll!LdrpDrainWorkQueue+0x12"),
        "BG_STACK_END",
      ].join("\r\n"),
      BEYOND_GATE_TEST_STACK_SCRIPT: fileURLToPath(new URL("./startup-stack.ps1", import.meta.url)) },
    timeout: 8000, maxBuffer: 4096,
  });
  assert.deepEqual(JSON.parse(stdout), [{
    status: "read", thread: "debugger-thread-zero", frame_count: 11,
    modules: ["ntdll", "kernelbase", "chrome", "chrome_exe", "chrome_elf"],
    export_hints: ["NtWaitForSingleObject", "WaitForSingleObjectEx", "PrefetchVirtualMemory"],
    frame_hints: [
      ["ntdll", "NtWaitForSingleObject"], ["kernelbase", "WaitForSingleObjectEx"],
      ["chrome", "other"], ["other", "other"], ["chrome", "other"],
      ["ntdll", "LdrpDrainWorkQueue"],
      ["ntdll", "NtSetInformationVirtualMemory"], ["kernelbase", "PrefetchVirtualMemory"],
      ["chrome_exe", "base::PreReadFile"], ["chrome_elf", "third_party_dlls::Init"],
      ["chrome_elf", "other"],
    ],
  }, { status: "unreadable", reason: "markers-not-found" }, {
    gallery_preparing: true, gallery_waiting: false, engine_banner: false,
    symbol_path_reported: false, stack_started: true, stack_completed: true,
    attach_failure_reported: true, access_denied_reported: false,
    invalid_parameter_reported: true, process_exit_reported: false,
    partial_copy_reported: false,
  }, [32, 8]]);
  for (const privateText of ["synthetic", "private", "foreign", "00000001", "0x99", "CryptQueryObject"]) {
    assert.equal(stdout.includes(privateText), false);
  }
});

test("browser symbol status requires matched module rows within its markers", {
  skip: process.platform !== "win32", timeout: 10000,
}, async () => {
  const rows = [
    "BG_BROWSER_SYMBOLS_BEGIN",
    "00000001`00000002 00000003`00000004 chrome (private pdb symbols) C:\\synthetic-private\\chrome.exe.pdb",
    "00000001`00000002 00000003`00000004 chrome_elf (pdb symbols) C:\\synthetic-private\\chrome_elf.dll.pdb",
    "BG_BROWSER_SYMBOLS_END",
  ].join("\r\n");
  const { stdout } = await promisify(execFile)("pwsh", ["-NoProfile", "-Command",
    ". $env:BEYOND_GATE_TEST_STACK_SCRIPT -BrowserProcessId 1 -BrowserExecutable fake -RunDirectory fake; " +
    "@((Get-SafeBrowserSymbolStatus $env:BEYOND_GATE_TEST_STACK), " +
    "(Get-SafeBrowserSymbolStatus ($env:BEYOND_GATE_TEST_STACK.Replace('pdb symbols', 'export symbols'))), " +
    "(Get-SafeBrowserSymbolStatus ($env:BEYOND_GATE_TEST_STACK.Replace('chrome (', 'foreign (').Replace('chrome_elf (', 'foreign ('))), " +
    "(Get-SafeBrowserSymbolStatus ($env:BEYOND_GATE_TEST_STACK.Replace('chrome.exe.pdb', 'chrome_elf.dll.pdb'))), " +
    "(Get-SafeBrowserSymbolStatus ($env:BEYOND_GATE_TEST_STACK.Replace('BG_BROWSER_SYMBOLS_END', 'missing')))) " +
    "| ConvertTo-Json -Compress",
  ], {
    env: { ...process.env, BEYOND_GATE_TEST_STACK: rows,
      BEYOND_GATE_TEST_STACK_SCRIPT: fileURLToPath(new URL("./startup-stack.ps1", import.meta.url)) },
    timeout: 8000, maxBuffer: 4096,
  });
  assert.deepEqual(JSON.parse(stdout), [
    { chrome_exe: true, chrome_elf: true },
    ...Array.from({ length: 2 }, () => ({ chrome_exe: false, chrome_elf: false })),
    { chrome_exe: false, chrome_elf: true },
    { chrome_exe: false, chrome_elf: false },
  ]);
  assert.equal(stdout.includes("synthetic-private"), false);
  assert.equal(stdout.includes(".pdb"), false);
  assert.equal(stdout.includes("00000001"), false);
});

test("symbol policy preserves CodeView checks and Windows export fallback", async () => {
  const source = await readFile(new URL("./startup-stack.ps1", import.meta.url), "utf8");
  assert.ok(source.includes("$commands = '.symopt-0x4c0;.reload /f ntdll.dll;.echo BG_STACK_BEGIN;"));
  assert.equal(source.includes("ArgumentList.Add('-ses')"), false);
  assert.equal(source.includes(".reload /i"), false);
});

test("public startup symbol verification rejects changed bytes and oversized files", {
  skip: process.platform !== "win32", timeout: 10000,
}, async (context) => {
  await mkdir(TEST_ROOT, { recursive: true });
  const directory = await mkdtemp(join(TEST_ROOT, "symbols-"));
  context.after(() => rm(directory, { recursive: true, force: true }));
  const path = join(directory, "invented.pdb");
  await writeFile(path, "invented");
  const hash = createHash("sha256").update("invented").digest("hex").toUpperCase();
  const { stdout } = await promisify(execFile)("pwsh", ["-NoProfile", "-Command",
    ". $env:BEYOND_GATE_TEST_SETUP_SCRIPT; " +
    "Assert-PublicStartupSymbol $env:BEYOND_GATE_TEST_SYMBOL $env:BEYOND_GATE_TEST_HASH; " +
    "$states = @('verified'); " +
    "try { Assert-PublicStartupSymbol $env:BEYOND_GATE_TEST_SYMBOL ('0' * 64); throw 'accepted changed bytes' } " +
    "catch { $states += $_.Exception.Message }; " +
    "$file = [IO.File]::OpenWrite($env:BEYOND_GATE_TEST_SYMBOL); " +
    "try { $file.SetLength(134217728) } finally { $file.Dispose() }; " +
    "$boundaryHash = (Get-FileHash -LiteralPath $env:BEYOND_GATE_TEST_SYMBOL -Algorithm SHA256).Hash; " +
    "Assert-PublicStartupSymbol $env:BEYOND_GATE_TEST_SYMBOL $boundaryHash; $states += 'boundary-verified'; " +
    "$file = [IO.File]::OpenWrite($env:BEYOND_GATE_TEST_SYMBOL); " +
    "try { $file.SetLength(134217729) } finally { $file.Dispose() }; " +
    "try { Assert-PublicStartupSymbol $env:BEYOND_GATE_TEST_SYMBOL $env:BEYOND_GATE_TEST_HASH; throw 'accepted oversized file' } " +
    "catch { $states += $_.Exception.Message }; $states | ConvertTo-Json -Compress",
  ], {
    env: { ...process.env, BEYOND_GATE_TEST_SYMBOL: path, BEYOND_GATE_TEST_HASH: hash,
      BEYOND_GATE_TEST_SETUP_SCRIPT: fileURLToPath(new URL("./install-startup-debugger.ps1", import.meta.url)) },
    timeout: 8000, maxBuffer: 4096,
  });
  assert.deepEqual(JSON.parse(stdout), [
    "verified",
    "Public browser symbol checksum or size verification failed.",
    "boundary-verified",
    "Public browser symbol checksum or size verification failed.",
  ]);
});

test("post-case public symbol reporting is explicit, bounded, and manual-only", async () => {
  const records = [];
  const child = { spawnfile: "synthetic-private.exe" };
  const browser = { process: () => child };
  const symbols = [{ module: "chrome_elf", status: "read", pdb_id: "0123456789ABCDEF0123456789ABCDEF1" }];
  const report = (record) => records.push(record);
  await reportPublicBrowserSymbols("chrome", {
    process: () => assert.fail("default runs must not inspect post-case metadata"),
  }, TEST_ROOT, { enabled: false, report });
  assert.deepEqual(records, []);
  await reportPublicBrowserSymbols("chrome", browser, TEST_ROOT, {
    enabled: true, report,
    inspect: async (...args) => {
      assert.deepEqual(args, [child, child.spawnfile, TEST_ROOT]);
      return { status: "read", public_module_symbols: symbols, unrelated: "synthetic-private" };
    },
  });
  assert.deepEqual(records.pop(), {
    status: "public-build-identities", browser: "chrome", phase: "after-browser-cases",
    inspection: { status: "read", public_module_symbols: symbols },
  });
  for (const [target, state] of [
    [browser, { status: "unreadable", cause_reason: "access-denied" }],
    [{ process: () => null }, { status: "not-observed" }],
  ]) {
    await reportPublicBrowserSymbols("chrome", target, TEST_ROOT, {
      enabled: true, report, inspect: async () => state,
    });
    assert.deepEqual(records.pop(), {
      status: "public-build-identities-unavailable", browser: "chrome",
      phase: "after-browser-cases", inspection: state,
    });
  }
  for (const [file, completed] of [
    ["run.mjs", "const cases = await nativeCases("],
    ["product-flow.mjs", "const results = await cases("],
  ]) {
    const source = await readFile(new URL(file, import.meta.url), "utf8");
    assert.ok(source.includes(completed));
    assert.ok(source.indexOf("await reportPublicBrowserSymbols(") > source.indexOf(completed));
  }
});

test("public browser symbol IDs omit PDB paths and reject malformed images", {
  skip: process.platform !== "win32", timeout: 10000,
}, async (context) => {
  await mkdir(TEST_ROOT, { recursive: true });
  const directory = await mkdtemp(join(TEST_ROOT, "symbols-"));
  context.after(() => rm(directory, { recursive: true, force: true }));
  // Minimal PE32+ image: one section and one CodeView record with an invented path.
  const image = Buffer.alloc(1024);
  const pdbPath = Buffer.from("C:\\synthetic-private\\chrome_elf.dll.pdb\0");
  image.write("MZ");
  image.writeUInt32LE(0x80, 0x3c);
  image.write("PE\0\0", 0x80);
  image.writeUInt16LE(0x8664, 0x84);
  image.writeUInt16LE(1, 0x86);
  image.writeUInt16LE(0xf0, 0x94);
  image.writeUInt16LE(0x2022, 0x96);
  image.writeUInt16LE(0x20b, 0x98);
  image.writeUInt32LE(0x1000, 0xb8);
  image.writeUInt32LE(0x200, 0xbc);
  image.writeUInt32LE(0x2000, 0xd0);
  image.writeUInt32LE(0x200, 0xd4);
  image.writeUInt32LE(16, 0x104);
  image.writeUInt32LE(0x1000, 0x138);
  image.writeUInt32LE(28, 0x13c);
  image.write(".rdata", 0x188);
  image.writeUInt32LE(0x200, 0x190);
  image.writeUInt32LE(0x1000, 0x194);
  image.writeUInt32LE(0x200, 0x198);
  image.writeUInt32LE(0x200, 0x19c);
  image.writeUInt32LE(0x40000040, 0x1ac);
  image.writeUInt32LE(2, 0x20c);
  image.writeUInt32LE(24 + pdbPath.length, 0x210);
  image.writeUInt32LE(0x1040, 0x214);
  image.writeUInt32LE(0x240, 0x218);
  image.write("RSDS", 0x240);
  Buffer.from("67452301ab89efcd0123456789abcdef", "hex").copy(image, 0x244);
  image.writeUInt32LE(1, 0x254);
  pdbPath.copy(image, 0x258);
  const missing = Buffer.from(image);
  missing.writeUInt32LE(0, 0x138);
  missing.writeUInt32LE(0, 0x13c);
  const ambiguous = Buffer.from(image);
  ambiguous.writeUInt32LE(56, 0x13c);
  image.copy(ambiguous, 0x21c, 0x200, 0x21c);
  await Promise.all([
    writeFile(join(directory, "chrome_elf.dll"), image),
    writeFile(join(directory, "chrome.exe"), missing),
    writeFile(join(directory, "msedge_elf.dll"), ambiguous),
    writeFile(join(directory, "msedge.exe"), "synthetic-private"),
  ]);
  const { stdout } = await promisify(execFile)("pwsh", ["-NoProfile", "-Command",
    ". $env:BEYOND_GATE_TEST_PROCESS_SCRIPT -BrowserProcessId 1 -BrowserExecutable fake -RunDirectory fake; " +
    "@('chrome_elf.dll', 'chrome.exe', 'msedge_elf.dll', 'msedge.exe') | ForEach-Object { " +
    "Get-PublicBrowserSymbolId (Join-Path $env:BEYOND_GATE_TEST_DIRECTORY $_) } | ConvertTo-Json -Compress",
  ], {
    env: { ...process.env, BEYOND_GATE_TEST_DIRECTORY: directory,
      BEYOND_GATE_TEST_PROCESS_SCRIPT: fileURLToPath(new URL("./startup-process.ps1", import.meta.url)) },
    timeout: 8000, maxBuffer: 4096,
  });
  const [valid, noCodeView, multipleCodeViews, invalid] = JSON.parse(stdout);
  assert.equal(valid.module, "chrome_elf");
  assert.equal(valid.status, "read");
  assert.equal(valid.pdb_id, "0123456789ABCDEF0123456789ABCDEF1");
  assert.deepEqual(noCodeView, { module: "chrome_exe", status: "codeview-unavailable" });
  assert.deepEqual(multipleCodeViews, { module: "msedge_elf", status: "codeview-unavailable" });
  assert.deepEqual(invalid, {
    module: "msedge_exe", status: "unreadable", cause_kind: "BadImageFormatException",
  });
  assert.equal(stdout.includes(directory), false);
  assert.equal(stdout.includes("synthetic-private"), false);
  assert.equal(stdout.includes(".pdb"), false);
});

test("missing or ambiguous prepared symbols do not prevent export-only collection", {
  skip: process.platform !== "win32", timeout: 10000,
}, async (context) => {
  await mkdir(TEST_ROOT, { recursive: true });
  const directory = await mkdtemp(join(TEST_ROOT, "symbols-"));
  context.after(() => rm(directory, { recursive: true, force: true }));
  const { stdout } = await promisify(execFile)("pwsh", ["-NoProfile", "-Command",
    ". $env:BEYOND_GATE_TEST_STACK_SCRIPT -BrowserProcessId 1 -BrowserExecutable fake -RunDirectory fake; " +
    "$cache = Join-Path $env:BEYOND_GATE_TEST_DIRECTORY cache; " +
    "$owned = Join-Path $env:BEYOND_GATE_TEST_DIRECTORY owned; " +
    "$null = New-Item -ItemType Directory -Path $owned; $states = @(); " +
    "$states += Copy-PreparedStartupSymbols $cache $owned; " +
    "$null = New-Item -ItemType Directory -Path $cache; " +
    "$states += Copy-PreparedStartupSymbols $cache $owned; " +
    "$first = Join-Path $cache first; $null = New-Item -ItemType Directory -Path $first; " +
    "[IO.File]::WriteAllText((Join-Path $first ntdll.pdb), 'invented'); " +
    "$states += Copy-PreparedStartupSymbols $cache $owned; " +
    "$second = Join-Path $cache second; $null = New-Item -ItemType Directory -Path $second; " +
    "[IO.File]::WriteAllText((Join-Path $second ntdll.pdb), 'not-selected'); " +
    "$states += Copy-PreparedStartupSymbols $cache $owned; " +
    "@{ states = $states; copied = [IO.File]::ReadAllText((Join-Path $owned ntdll.pdb)) -ceq 'invented' } " +
    "| ConvertTo-Json -Compress",
  ], {
    env: { ...process.env, BEYOND_GATE_TEST_DIRECTORY: directory,
      BEYOND_GATE_TEST_STACK_SCRIPT: fileURLToPath(new URL("./startup-stack.ps1", import.meta.url)) },
    timeout: 8000, maxBuffer: 4096,
  });
  assert.deepEqual(JSON.parse(stdout), {
    states: ["missing", "missing", "copied", "ambiguous"], copied: true,
  });
});

test("debugger setup refuses a normal local machine before downloading or installing tools", {
  skip: process.platform !== "win32", timeout: 10000,
}, async () => {
  await assert.rejects(promisify(execFile)("pwsh", ["-NoProfile", "-File",
    fileURLToPath(new URL("./install-startup-debugger.ps1", import.meta.url)),
  ], {
    env: { ...process.env, GITHUB_ACTIONS: "false", RUNNER_TEMP: "" },
    timeout: 8000, maxBuffer: 4096,
  }), (error) => {
    assert.equal(error.code, 1);
    assert.ok(error.stderr.includes("Debugger setup requires"));
    return true;
  });
});

test("bounded probe capture drains both pipes after their storage limits", {
  skip: process.platform !== "win32", timeout: 10000,
}, async () => {
  const { stdout } = await promisify(execFile)("pwsh", ["-NoProfile", "-Command",
    ". $env:BEYOND_GATE_TEST_STACK_SCRIPT -BrowserProcessId 1 -BrowserExecutable fake -RunDirectory fake; " +
    "$probe = [Diagnostics.Process]::new(); " +
    "$output = [IO.MemoryStream]::new([byte[]]::new(16384), $true); " +
    "$errors = [IO.MemoryStream]::new([byte[]]::new(4096), $true); " +
    "try { " +
    "$probe.StartInfo.FileName = $env:BEYOND_GATE_TEST_NODE; " +
    "$probe.StartInfo.UseShellExecute = $false; " +
    "$probe.StartInfo.RedirectStandardOutput = $true; $probe.StartInfo.RedirectStandardError = $true; " +
    "$probe.StartInfo.ArgumentList.Add('--eval'); " +
    "$probe.StartInfo.ArgumentList.Add(\"process.stdout.write('synthetic-private'.repeat(8000)); " +
    "process.stderr.write('synthetic-private'.repeat(8000));\"); " +
    "$null = $probe.Start(); " +
    "$result = Read-BoundedProbeOutput $probe $output $errors 5000; " +
    "@{ completed = $result.completed; stdout_bytes = $result.streams[0].bytes; " +
    "stderr_bytes = $result.streams[1].bytes; stdout_stored = $output.Position; " +
    "stderr_stored = $errors.Position } | ConvertTo-Json -Compress " +
    "} finally { if (!$probe.HasExited) { $probe.Kill($true); $probe.WaitForExit() }; " +
    "$probe.Dispose(); $output.Dispose(); $errors.Dispose() }",
  ], {
    env: { ...process.env, BEYOND_GATE_TEST_NODE: process.execPath,
      BEYOND_GATE_TEST_STACK_SCRIPT: fileURLToPath(new URL("./startup-stack.ps1", import.meta.url)) },
    timeout: 8000, maxBuffer: 4096,
  });
  assert.deepEqual(JSON.parse(stdout), {
    completed: true, stdout_bytes: 136000, stderr_bytes: 136000,
    stdout_stored: 16384, stderr_stored: 4096,
  });
  assert.equal(stdout.includes("synthetic"), false);
  assert.equal(stdout.includes("private"), false);
});

test("launch failure aborts and waits for its owned browser before directory cleanup", async () => {
  const attempted = [];
  const cause = new Error("Target.setDiscoverTargets timed out");
  const dependencies = launchDependencies(async (options) => {
    options.signal.addEventListener("abort", () => attempted.push("abort"), { once: true });
    throw cause;
  });
  dependencies.observe = () => ({
    read: async () => { attempted.push("inspect"); return {}; },
    waitForExit: async () => { attempted.push("exit"); },
    stop: () => attempted.push("release"),
  });
  await assert.rejects(withCleanup(() => launchBrowser("chrome", {
    directory: TEST_ROOT, proxyPort: 12345, protocolTimeout: 15000,
  }, dependencies), async () => { attempted.push("directory"); }), (error) => {
    assert.equal(error.cause, cause);
    assert.equal(error.launch.startup_cleanup, "passed");
    return true;
  });
  assert.deepEqual(attempted, ["inspect", "abort", "exit", "release", "directory"]);
});

test("owned browser termination and directory failures preserve the primary launch cause", async () => {
  const cause = new Error("Target.setDiscoverTargets timed out");
  const cleanup = new Error("synthetic-private process details");
  const dependencies = launchDependencies(async () => { throw cause; });
  dependencies.observe = () => ({
    read: async () => ({}),
    waitForExit: async () => { throw cleanup; },
    stop: () => {},
  });
  await assert.rejects(withCleanup(() => launchBrowser("chrome", {
    directory: TEST_ROOT, proxyPort: 12345, protocolTimeout: 15000,
  }, dependencies), () => cleanupActions([
    ["fixture-directory", async () => { throw new Error("synthetic-private directory"); }],
  ])), (error) => {
    assert.equal(error.cause, cause);
    assert.equal(error.cleanupCause, cleanup);
    assert.equal(error.launch.startup_cleanup, "failed");
    assert.deepEqual(error.cleanup, ["browser-process", "fixture-directory"]);
    assert.equal(JSON.stringify(error.launch).includes("synthetic-private"), false);
    return true;
  });
});

test("a real driver pipe timeout exits its owned synthetic process before directory cleanup", {
  skip: process.platform !== "win32", timeout: 40000,
}, async (context) => {
  await mkdir(TEST_ROOT, { recursive: true });
  const directory = await mkdtemp(join(TEST_ROOT, "chrome-"));
  let child;
  context.after(async () => {
    if (child && child.exitCode === null && child.signalCode === null) {
      const exited = once(child, "exit", { signal: AbortSignal.timeout(5000) });
      child.kill();
      await exited;
    }
    await rm(directory, { recursive: true, force: true });
  });
  const dependencies = launchDependencies((options) => puppeteer.launch({
    ...options, ignoreDefaultArgs: true,
    // Node receives Chrome-only flags as data after "--".
    args: ["--input-type=module", "--eval", "setInterval(() => {}, 1000)", "--",
      `--user-data-dir=${options.userDataDir}`],
  }));
  dependencies.findExecutable = async () => process.execPath;
  dependencies.observe = (path, directory) => observeStartup(path, directory, async (owned) => {
    child = owned;
    return { status: "read" };
  });
  await assert.rejects(withCleanup(() => launchBrowser("chrome", {
    directory, proxyPort: 12345, protocolTimeout: 15000,
  }, dependencies), () => rm(directory, { recursive: true })), (error) => {
    assert.equal(error.launch.cause_kind, "ProtocolError");
    assert.equal(error.launch.cause_reason, "timed-out");
    assert.equal(error.launch.protocol_messages_received, 0);
    assert.equal(error.launch.startup_cleanup, "passed");
    assert.ok(child && (child.exitCode !== null || child.signalCode !== null));
    return true;
  });
  await assert.rejects(stat(directory), { code: "ENOENT" });
});

/** Checks owned process fields, bounded real stacks, and target response for either probe mode. */
async function checkOwnedStartup(context, keepRunning = false) {
  await mkdir(TEST_ROOT, { recursive: true });
  const directory = await mkdtemp(join(TEST_ROOT, "chrome-"));
  const observer = observeStartup(process.execPath, directory);
  let child;
  context.after(async () => {
    observer.stop();
    if (child && child.exitCode === null && child.signalCode === null) {
      child.kill();
      await once(child, "exit");
    }
    await rm(directory, { recursive: true, force: true });
  });
  await mkdir(join(directory, "profile", "Crashpad"), { recursive: true });
  await writeFile(join(directory, "profile", "Local State"), "{}");
  child = spawn(process.execPath, [
    "--input-type=module", "--eval",
    "console.log('ready'); process.stdin.once('data', () => console.log('resumed')); setInterval(() => {}, 1000)",
    "--", `--user-data-dir=${join(directory, "profile")}`,
  ], { stdio: ["pipe", "pipe", "pipe"] });
  const [ready] = await Promise.race([
    once(child.stdout, "data"),
    once(child, "error").then(([error]) => { throw error; }),
  ]);
  assert.equal(ready.toString().trim(), "ready");
  const result = await observer.read({ keepRunning });
  assert.equal(result.startup_process.status, "read");
  assert.equal(result.startup_process.chrome_dll_loaded, false);
  assert.equal(result.startup_process.chrome_elf_loaded, false);
  assert.deepEqual(result.startup_process.public_module_symbols, []);
  assert.equal(result.startup_process.profile_created, true);
  assert.equal(result.startup_process.crashpad_created, true);
  assert.equal(result.startup_process.local_state_created, true);
  assert.deepEqual(result.startup_process.child_kinds, []);
  assert.ok(result.startup_process.thread_count > 0);
  assert.ok(result.startup_process.cpu_ms >= 0);
  assert.ok(result.startup_process.wait_reasons.every(Number.isInteger));
  assert.ok(result.startup_process.thread_states.length > 0 &&
    result.startup_process.thread_states.length <= 5);
  for (const [state, reason] of result.startup_process.thread_states) {
    assert.ok(Number.isInteger(state));
    assert.ok(reason === null || Number.isInteger(reason));
  }
  assert.equal(JSON.stringify(result).includes(directory), false);
  assert.equal(JSON.stringify(result).includes(process.execPath), false);
  if (process.env.BEYOND_GATE_STARTUP_STACKS === "1") {
    context.diagnostic(JSON.stringify(result.startup_stack));
    assert.equal(result.startup_stack.status, "read", JSON.stringify(result.startup_stack));
    assert.equal(result.startup_stack.capture_mode, keepRunning ? "non-suspending" : "suspended");
    assert.ok(result.startup_stack.frame_count > 0 && result.startup_stack.frame_count <= 32);
    assert.ok(result.startup_stack.modules.includes("ntdll"));
    assert.equal(result.startup_stack.os_symbols_loaded, true);
    assert.equal(result.startup_stack.symbol_match_policy, "codeview-required");
    const frames = [
      ...result.startup_stack.frame_hints,
      ...result.startup_stack.secondary_threads.flatMap((thread) => thread.frame_hints),
    ];
    assert.ok(frames.some(([module, symbol]) =>
      module === "kernel32" && symbol === "BaseThreadInitThunk"),
    "Windows export fallback must remain available without a kernel32 PDB");
    assert.ok(result.startup_stack.secondary_threads.length > 0 &&
      result.startup_stack.secondary_threads.length <= 4);
    for (const thread of result.startup_stack.secondary_threads) {
      assert.equal(thread.status, "read");
      assert.ok(thread.frame_hints.length > 0 && thread.frame_hints.length <= 8);
    }
  } else {
    assert.deepEqual(result.startup_stack, { status: "not-requested" });
  }
  const resumed = once(child.stdout, "data", { signal: AbortSignal.timeout(3000) });
  child.stdin.write("continue");
  assert.equal((await resumed)[0].toString().trim(), "resumed");
}

test("Windows startup inspection reports only fixed state for an owned synthetic process", {
  skip: process.platform !== "win32", timeout: 20000,
}, (context) => checkOwnedStartup(context));

test("Windows non-suspending startup capture keeps its owned target usable", {
  skip: process.platform !== "win32" || process.env.BEYOND_GATE_STARTUP_STACKS !== "1", timeout: 20000,
}, (context) => checkOwnedStartup(context, true));

test("launch failure reads at most 16 KiB of the owned log without printing its payload", async (context) => {
  await mkdir(TEST_ROOT, { recursive: true });
  const directory = await mkdtemp(join(TEST_ROOT, "chrome-"));
  context.after(() => rm(directory, { recursive: true, force: true }));
  await writeFile(join(directory, "startup.log"),
    "[1:2:time:ERROR:devtools_pipe_handler.cc(12)] " +
    "Remote debugging pipe file descriptors are not open.\n" +
    "synthetic-secret".repeat(2000) + "\nAccess is denied.\n");
  await assert.rejects(launchBrowser("chrome", {
    directory, proxyPort: 12345, protocolTimeout: 15000,
  }, launchDependencies(async () => { throw new Error("Target.setDiscoverTargets timed out"); })),
  (error) => {
    assert.equal(error.launch.startup_log, "read");
    assert.equal(error.launch.startup_log_bytes_read, 16384);
    assert.equal(error.launch.startup_log_limit_reached, true);
    assert.deepEqual(error.launch.startup_log_sources, ["devtools_pipe_handler.cc"]);
    assert.deepEqual(error.launch.startup_log_reasons, ["debugging-pipe-unavailable"]);
    assert.equal(JSON.stringify(error.launch).includes("synthetic-secret"), false);
    return true;
  });
});

test("missing executable and version failures never attempt browser launch", async () => {
  for (const stage of ["executable", "version-metadata"]) {
    const records = [];
    const dependencies = launchDependencies(() => assert.fail("must not launch"), records);
    dependencies[stage === "executable" ? "findExecutable" : "inspect"] = async () => {
      throw Object.assign(new Error("synthetic-private path"), { code: "ENOENT" });
    };
    await assert.rejects(launchBrowser("chrome", {}, dependencies), (error) => {
      assert.equal(error.launch.launch_stage, stage);
      assert.equal(error.launch.cause_reason, "executable-missing");
      assert.equal(error.launch.process_started, false);
      return true;
    });
    assert.deepEqual(records, []);
  }
});

test("known startup errors are classified without copying arbitrary causes or identifiers", () => {
  for (const [error, reason, method, code] of [
    [Object.assign(new Error("Waiting for target failed"), { name: "TimeoutError" }), "timed-out"],
    [Object.assign(new Error("Connection closed."), { name: "ConnectionClosedError" }), "connection-closed"],
    [new Error("Target.setAutoAttach timed out. private diagnostic"), "timed-out", "Target.setAutoAttach"],
    [new Error("The browser is already running for C:\\private"), "profile-in-use"],
    [new Error("The browser cannot write to C:\\private"), "profile-not-writable"],
    [Object.assign(new Error("private diagnostic"), { code: "EPERM" }), "access-denied"],
    [new Error("Failed to launch the browser process:  Code: 21\nstderr: private"), "process-launch-failed", undefined, 21],
    [Object.assign(new Error("Protocol error (Private.secret): private"), { name: "ProtocolError" }), "protocol-error"],
    [new Error("Protocol error (Target.setAutoAttach): private"), "protocol-error", "Target.setAutoAttach"],
    [new Error("synthetic-secret".repeat(10000)), "unclassified"],
    [null, "unclassified"],
  ]) {
    const result = safeLaunchCause(error);
    assert.equal(result.cause_reason, reason);
    assert.equal(result.protocol_method, method);
    assert.equal(result.process_exit_code, code);
    assert.ok(JSON.stringify(result).length < 256);
  }
});

test("failed launch still runs every owned cleanup action and preserves both failures", async () => {
  const cause = new Error("primary launch failure");
  const attempted = [];
  await assert.rejects(withCleanup(async () => { throw cause; }, () => cleanupActions([
    ["native-host", async () => { attempted.push("host"); throw new Error("private host path"); }],
    ["native-registration", async () => { attempted.push("registration"); }],
    ["browser", async () => { attempted.push("browser"); throw new Error("private profile"); }],
    ["proxy", async () => { attempted.push("proxy"); }],
    ["fixture-directory", async () => { attempted.push("directory"); }],
  ])), (error) => {
    assert.equal(error, cause);
    assert.deepEqual(error.cleanup, ["native-host", "browser"]);
    assert.equal(JSON.stringify(error.cleanup).includes("private"), false);
    return true;
  });
  assert.deepEqual(attempted, ["host", "registration", "browser", "proxy", "directory"]);
});

test("cleanup-only failure rejects success and successful cleanup preserves operation result", async () => {
  assert.equal(await withCleanup(async () => 42, async () => {}), 42);
  await assert.rejects(withCleanup(async () => 42, () => cleanupActions([
    ["fixture-directory", async () => { throw new Error("synthetic-private"); }],
  ])), (error) => {
    assert.equal(error.message, "owned fixture cleanup failed");
    assert.deepEqual(error.cleanup, ["fixture-directory"]);
    return true;
  });
});

test("browser and proxy cleanup failures do not prevent owned directory removal", async (context) => {
  await mkdir(TEST_ROOT, { recursive: true });
  const directory = await mkdtemp(join(TEST_ROOT, "chrome-"));
  context.after(() => rm(directory, { recursive: true, force: true }));
  let connectionsClosed = false;
  await assert.rejects(cleanupProbe("chrome", {
    close: async () => { throw new Error("synthetic-private browser"); },
  }, {
    close: (callback) => callback(new Error("synthetic-private proxy")),
    closeAllConnections: () => { connectionsClosed = true; },
  }, directory), (error) => {
    assert.deepEqual(error.cleanup, ["browser", "proxy"]);
    assert.equal(error.message, "owned fixture cleanup failed");
    return true;
  });
  assert.equal(connectionsClosed, true);
  await assert.rejects(stat(directory), { code: "ENOENT" });
});
