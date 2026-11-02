/** Offline checks for the development-only extension's narrow synthetic scope. */
import assert from "node:assert/strict";
import { mkdir, mkdtemp, readFile, rm, stat, writeFile } from "node:fs/promises";
import { test } from "node:test";
import { channel } from "node:diagnostics_channel";
import { EventEmitter, once } from "node:events";
import { spawn } from "node:child_process";
import { PassThrough } from "node:stream";
import { checkSyntheticCookie, syntheticHandoff, SYNTHETIC } from "./extension/service_worker.js";
import { launchBrowser, safeLaunchCause, safeBrowserLog, observeStartup, cleanupActions, withCleanup } from "./browser-launch.mjs";
import { join } from "node:path";
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
  const observer = observeStartup("owned.exe", directory, async (child) => {
    inspected.push(child);
    return { status: "read", chrome_dll_loaded: false };
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
    const result = await observer.read();
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

test("Windows startup inspection reports only fixed state for an owned synthetic process", {
  skip: process.platform !== "win32", timeout: 15000,
}, async (context) => {
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
    "--input-type=module", "--eval", "console.log('ready'); setInterval(() => {}, 1000)",
    "--", `--user-data-dir=${join(directory, "profile")}`,
  ], { stdio: ["ignore", "pipe", "pipe"] });
  const [ready] = await Promise.race([
    once(child.stdout, "data"),
    once(child, "error").then(([error]) => { throw error; }),
  ]);
  assert.equal(ready.toString().trim(), "ready");
  const result = await observer.read();
  assert.equal(result.startup_process.status, "read");
  assert.equal(result.startup_process.chrome_dll_loaded, false);
  assert.equal(result.startup_process.chrome_elf_loaded, false);
  assert.equal(result.startup_process.profile_created, true);
  assert.equal(result.startup_process.crashpad_created, true);
  assert.equal(result.startup_process.local_state_created, true);
  assert.deepEqual(result.startup_process.child_kinds, []);
  assert.ok(result.startup_process.thread_count > 0);
  assert.ok(result.startup_process.cpu_ms >= 0);
  assert.ok(result.startup_process.wait_reasons.every(Number.isInteger));
  assert.equal(JSON.stringify(result).includes(directory), false);
  assert.equal(JSON.stringify(result).includes(process.execPath), false);
});

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
