/** Isolated regular-browser feasibility; no account, personal profile, or live requests. */
import assert from "node:assert/strict";
import { createServer } from "node:http";
import { execFile } from "node:child_process";
import { randomBytes } from "node:crypto";
import { copyFile, mkdir, mkdtemp, readFile, realpath, rm, writeFile } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { release } from "node:os";
import { fileURLToPath } from "node:url";
import { promisify } from "node:util";
import {
  executable, launchBrowser, reportPublicBrowserSymbols, cleanupActions, withCleanup,
} from "./browser-launch.mjs";

const HERE = dirname(fileURLToPath(import.meta.url));
const TEST_ROOT = join(HERE, "..", "..", "target", "browser-feasibility");
const execute = promisify(execFile);
const REPO = join(HERE, "..", "..");

/** Builds the development-only Rust executable with the repository's VS 2022 wrapper. */
async function buildHost() {
  await execute("pwsh", [
    "-NoProfile", "-File", join(HERE, "..", "cargo.ps1"),
    "build", "-p", "beyond-gate-core", "--example", "browser-handoff-fixture",
    "--features", "browser-fixture", "--locked", "--quiet",
  ], { cwd: REPO, timeout: 600000, maxBuffer: 1024 * 1024 });
}

/** Changes only this run's unique registry key or verified fixture process. */
async function registration(action, name, host, directory) {
  const { stdout } = await execute("pwsh", [
    "-NoProfile", "-File", join(HERE, "registration.ps1"),
    "-Action", action, "-Browser", name, "-HostName", host,
    "-RunDirectory", directory,
  ], { timeout: 15000, maxBuffer: 65536 });
  return JSON.parse(stdout);
}

/** Reads expected test metadata absence explicitly, without suppressing other failures. */
async function hostReport(directory) {
  try {
    return JSON.parse(await readFile(join(directory, "host-report.json"), "utf8"));
  } catch (error) {
    if (error.code === "ENOENT") return null;
    throw error;
  }
}

/** Tests consent, absent pending authorization, and real native receipt separately. */
async function nativeCases(worker, name, id, directory) {
  const host = `com.beyondgate.synthetic.${randomBytes(16).toString("hex")}`;
  const origin = `chrome-extension://${id}/`;
  await copyFile(
    join(REPO, "target", "debug", "examples", "browser-handoff-fixture.exe"),
    join(directory, "host.exe"),
  );
  await copyFile(
    join(REPO, "docs", "fixtures", "browser-handoff-v1.json"),
    join(directory, "fixture.json"),
  );
  let registered = false;
  let stage = "registration";
  let outcome = "not-received";
  const results = [];
  try {
    await registration("CheckAbsent", name, host, directory);
    for (const scenario of [
      { name: "missing-registration", mode: "receive", consent: true,
        registration: "absent", code: "disconnected", reads: 0, started: false, received: false },
      { name: "wrong-origin", mode: "receive", consent: true,
        registration: "wrong-origin", code: "disconnected", reads: 0, started: false, received: false },
      { name: "no-consent", mode: "receive", consent: false,
        code: "consent_required", reads: 0, started: false, received: false },
      { name: "no-pending", mode: "no-pending", consent: true,
        code: "no_pending_login", reads: 0, started: true, received: false },
      { name: "native-receipt", mode: "receive", consent: true,
        code: "unsupported_contract", reads: 1, started: true, received: true },
    ]) {
      stage = scenario.name;
      const differentId = id === "a".repeat(32) ? "b".repeat(32) : "a".repeat(32);
      await writeFile(join(directory, "native-host.json"), JSON.stringify({
        name: host, description: "Beyond Gate development-only synthetic host",
        path: join(directory, "host.exe"), type: "stdio",
        allowed_origins: [
          scenario.registration === "wrong-origin" ? `chrome-extension://${differentId}/` : origin,
        ],
      }));
      if (scenario.registration !== "absent" && !registered) {
        stage = "registration";
        await registration("Register", name, host, directory);
        registered = true;
        stage = scenario.name;
      }
      await rm(join(directory, "host-report.json"), { force: true });
      await writeFile(join(directory, "control.json"), JSON.stringify({
        context: "beyond-gate-synthetic-v1", browser: name, origin, mode: scenario.mode,
      }));
      const reply = await worker.evaluate(
        (host, browser, consent) => globalThis.runSyntheticHandoff(host, browser, consent),
        host, name, scenario.consent,
      );
      outcome = [
        "consent_required", "no_pending_login", "unsupported_contract", "protocol_error",
        "cookie_unavailable", "disconnected", "timed_out", "save_indeterminate",
      ].includes(reply.code) ? reply.code : "unexpected";
      assert.equal(reply.code, scenario.code);
      assert.equal(reply.cookie_reads, scenario.reads);
      assert.equal(reply.received, scenario.received);
      assert.equal(reply.validated, false);
      assert.equal(reply.saved, false);
      const observed = await hostReport(directory);
      assert.equal(observed !== null, scenario.started);
      if (observed) {
        assert.equal(observed.origin_matches, true);
        assert.equal(observed.hello_received, true);
        assert.equal(observed.submit_received, scenario.received);
        assert.equal(observed.validation_performed, false);
        assert.equal(observed.storage_used, false);
      }
      const cleanup = await registration("StopHost", name, host, directory);
      results.push({
        case: scenario.name, code: reply.code, cookie_reads: reply.cookie_reads,
        native_started: observed !== null, native_received: reply.received,
        host_cleanup: cleanup.host_cleanup, validated: false, saved: false,
      });
    }
  } catch (cause) {
    const error = new Error("synthetic native fixture failed", { cause });
    error.stage = stage;
    error.outcome = outcome;
    throw error;
  } finally {
    try {
      await registration("StopHost", name, host, directory);
    } finally {
      if (registered) await registration("Remove", name, host, directory);
    }
  }
  return results;
}

/** Rejects HTTP and CONNECT traffic locally; never forwards a browser request. */
async function rejectingProxy() {
  const server = createServer((_request, response) => {
    response.writeHead(403);
    response.end();
  });
  server.on("connect", (_request, socket) => socket.destroy());
  await new Promise((resolve, reject) => {
    server.once("error", reject);
    server.listen(0, "127.0.0.1", resolve);
  });
  return server;
}

/** Deletes only the unique owned test directory after verifying its parent. */
async function removeOwnedDirectory(path) {
  assert.equal(dirname(await realpath(path)), await realpath(TEST_ROOT));
  await rm(path, { recursive: true, force: false, maxRetries: 3, retryDelay: 500 });
}

/** Attempts all owned cleanup and reports failures without exposing filesystem details. */
async function cleanupProbe(name, browser, proxy, directory) {
  await cleanupActions([
    ["browser", async () => { if (browser) await browser.close(); }],
    ["proxy", async () => {
      if (proxy) await new Promise((resolve, reject) => {
        proxy.close((error) => error ? reject(error) : resolve());
        proxy.closeAllConnections();
      });
    }],
    ["fixture-directory", () => removeOwnedDirectory(directory)],
  ]);
}

/** Loads the synthetic fixture through the supported DevTools pipe extension API. */
async function probe(name) {
  await mkdir(TEST_ROOT, { recursive: true });
  const directory = await mkdtemp(join(TEST_ROOT, `${name}-`));
  let proxy;
  let browser;
  let phase = "proxy-start";
  return await withCleanup(async () => {
    try {
      proxy = await rejectingProxy();
      const address = proxy.address();
      phase = "browser-start";
      browser = await launchBrowser(name, {
        directory, proxyPort: address.port, protocolTimeout: 15000,
      });
      phase = "extension-load";
      const id = await browser.installExtension(join(HERE, "extension"));
      phase = "extension-worker";
      const target = await browser.waitForTarget(
        (candidate) => candidate.type() === "service_worker" &&
          candidate.url() === `chrome-extension://${id}/service_worker.js`,
        { timeout: 15000 },
      );
      const worker = await target.worker();
      assert.ok(worker, "extension service worker must exist");
      phase = "cookie-api";
      const result = await worker.evaluate(() => globalThis.inspectSyntheticCookie());
      assert.equal(result.cookie_api, "passed");
      phase = "native-messaging";
      const cases = await nativeCases(worker, name, id, directory);
      await reportPublicBrowserSymbols(name, browser, directory);
      const driver = JSON.parse(await readFile(
        join(HERE, "node_modules", "puppeteer-core", "package.json"), "utf8",
      ));
      return {
        browser: name,
        browser_version: await browser.version(),
        browser_flavor: "regular-installed",
        os_version: release(),
        node_version: process.version,
        driver_version: driver.version,
        native_registration_scope: `HKCU ${name} NativeMessagingHosts; unique test key`,
        extension_load_method: "DevTools Extensions.loadUnpacked over pipe",
        headless: true,
        synthetic_only: true,
        cookie_api: result.cookie_api,
        native_messaging: "passed",
        native_cases: cases,
        application_peer_authentication: "not-tested",
        account_validation: "not-tested",
        protected_save: "not-tested",
        cleanup: "passed",
      };
    } catch (cause) {
      const error = new Error(`${name}: ${phase} failed`, { cause });
      error.launch = cause.launch;
      error.cleanup = cause.cleanup;
      throw error;
    }
  }, () => cleanupProbe(name, browser, proxy, directory));
}

/** Reports each required browser separately and fails rather than silently skipping it. */
export async function main({ build = buildHost, run = probe } = {}) {
  assert.equal(process.platform, "win32", "this feasibility test supports Windows only");
  const args = process.argv.slice(2);
  assert.ok(args.length === 0 || (args.length === 2 && args[0] === "--browser"));
  const names = args.length === 0 ? ["chrome", "edge"] : [args[1]];
  assert.ok(names.every((name) => name === "chrome" || name === "edge"));
  await build();
  for (const name of names) {
    try {
      console.log(JSON.stringify({ status: "passed", ...await run(name) }));
    } catch (error) {
      console.error(JSON.stringify({
        browser: name, status: "failed", error: error.message,
        launch: error.launch, cleanup_failures: error.cleanup,
        native_stage: error.cause?.stage, native_outcome: error.cause?.outcome,
      }));
      process.exitCode = 1;
    }
  }
}

export { executable, rejectingProxy, registration, cleanupProbe, TEST_ROOT, REPO };

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    await main();
  } catch {
    console.error(JSON.stringify({
      status: "failed", error: "browser fixture setup failed",
    }));
    process.exitCode = 1;
  }
}
