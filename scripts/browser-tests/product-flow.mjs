/** Real regular browsers and product controllers with no real cookie or native storage access. */
import assert from "node:assert/strict";
import { execFile, spawn } from "node:child_process";
import { randomBytes } from "node:crypto";
import { copyFile, mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { release } from "node:os";
import { createInterface } from "node:readline";
import { promisify } from "node:util";
import { fileURLToPath, pathToFileURL } from "node:url";
import {
  launchBrowser, reportPublicBrowserSymbols, cleanupActions, withCleanup, safeLaunchCause,
} from "./browser-launch.mjs";
import { rejectingProxy, registration, cleanupProbe, TEST_ROOT, REPO } from "./run.mjs";

const HERE = dirname(fileURLToPath(import.meta.url));
const execute = promisify(execFile);
const CONTEXT = "bg-windows-product-fixture-v1";

/** Owns one bounded fixture CLI process and reads public JSON metadata only. */
export async function startCli(mode, launch = spawn) {
  const child = launch(join(REPO, "target", "debug", "examples", "windows-login-fixture.exe"),
    ["fixture-cli", mode], { cwd: REPO, stdio: ["ignore", "pipe", "pipe"] });
  let stderr = "";
  let result;
  let readyCount = 0;
  let resultCount = 0;
  let invalidMetadata = false;
  let rejectReady;
  const ready = new Promise((resolve, reject) => {
    rejectReady = reject;
    const lines = createInterface({ input: child.stdout });
    lines.on("line", (line) => {
      try {
        const message = JSON.parse(line);
        assert.equal(message.fixture, CONTEXT);
        if (message.ready === true) {
          assert.equal(++readyCount, 1);
          assert.equal(resultCount, 0);
          assert.deepEqual(Object.keys(message).sort(), ["fixture", "namespace", "ready"]);
          assert.match(message.namespace, /^[0-9a-f]{32}$/);
          resolve(message);
        } else {
          assert.equal(readyCount, 1);
          assert.equal(++resultCount, 1);
          assert.deepEqual(Object.keys(message).sort(), [
            "cli_outcome", "fixture", "in_memory_saved", "live_validated",
            "native_saved", "service_requests",
          ]);
          assert.equal(typeof message.cli_outcome, "string");
          assert.equal(typeof message.in_memory_saved, "boolean");
          assert.ok(Number.isInteger(message.service_requests) && message.service_requests >= 0);
          assert.equal(message.live_validated, false);
          assert.equal(message.native_saved, false);
          result = message;
        }
      } catch {
        invalidMetadata = true;
        reject(new Error("fixture CLI returned invalid public metadata"));
      }
    });
  });
  child.stderr.on("data", (chunk) => { stderr += chunk; });
  child.once("error", () => rejectReady(new Error("fixture CLI could not start")));
  const exited = new Promise((resolve) => child.once("close", (code) => {
    rejectReady(new Error("fixture CLI stopped before readiness"));
    resolve(code);
  }));
  const timer = setTimeout(() => child.kill(), 35000);
  const stop = async () => {
    if (child.exitCode === null && child.signalCode === null) child.kill();
    await exited;
    clearTimeout(timer);
  };
  try {
    const metadata = await ready;
    return {
      metadata, stop,
      async finish() {
        const code = await exited;
        clearTimeout(timer);
        const failure = /^Windows login fixture failed: ([a-z_]+)\.\r?\n$/.exec(stderr);
        assert.equal(code, 0, `fixture CLI failed (${failure?.[1] ?? "no fixed cause"})`);
        assert.equal(stderr, "", "fixture CLI emitted diagnostics");
        assert.equal(invalidMetadata, false, "fixture CLI returned invalid public metadata");
        assert.equal(readyCount, 1, "exactly one readiness record is required");
        assert.equal(resultCount, 1, "exactly one completion record is required");
        assert.ok(result, "fixture CLI completion metadata is required");
        return result;
      },
    };
  } catch (error) {
    await stop();
    throw error;
  }
}

/** Runs full pairing, mock validation/save, cancellation, and acknowledgement-loss cases. */
async function cases(page, name, directory, host, recipe) {
  const results = [];
  for (const mode of ["saved", "rejected", "cancelled", "lost-ack", "storage-unavailable"]) {
    const cli = await startCli(mode);
    try {
      await rm(join(directory, "host-report.json"), { force: true });
      await writeFile(join(directory, "control.json"), JSON.stringify({
        context: CONTEXT, namespace: cli.metadata.namespace,
      }));
      const browser = await page.evaluate(
        (host, name, recipe, mode) => globalThis.runProductFixture(host, name, recipe, mode),
        host, name, recipe, mode,
      );
      const result = await cli.finish();
      const expected = { saved: "saved", rejected: "session_rejected",
        cancelled: "cancelled", "lost-ack": "save_indeterminate",
        "storage-unavailable": "storage_unavailable" }[mode];
      assert.equal(browser.code, expected, `${mode}: browser result`);
      assert.equal(result.cli_outcome, expected, `${mode}: CLI result`);
      assert.equal(browser.cookie_queries, 1);
      assert.equal(browser.real_cookie_api_called, false);
      assert.equal(result.in_memory_saved, mode === "saved" || mode === "lost-ack");
      assert.ok(result.service_requests <= 1, "no retry or extra service request");
      if (mode === "saved" || mode === "rejected") assert.equal(result.service_requests, 1);
      assert.equal(result.live_validated, false);
      assert.equal(result.native_saved, false);
      const cleanup = await registration("StopHost", name, host, directory);
      assert.ok(["exited", "stopped"].includes(cleanup.host_cleanup));
      results.push({ case: mode, browser_outcome: browser.code, ...result,
        real_cookie_api_called: false, host_cleanup: cleanup.host_cleanup });
    } catch (cause) {
      throw new Error(`${mode}: ${cause.message}`, { cause });
    } finally {
      await cli.stop();
      await registration("StopHost", name, host, directory);
    }
  }
  return results;
}

/** Loads the pinned production package in one new regular profile with outbound traffic blocked. */
async function probe(name, recipe) {
  await mkdir(TEST_ROOT, { recursive: true });
  const directory = await mkdtemp(join(TEST_ROOT, `${name}-`));
  const host = `com.beyondgate.synthetic.${randomBytes(16).toString("hex")}`;
  let browser;
  let proxy;
  let registered = false;
  let stage = "setup";
  return await withCleanup(async () => {
    try {
      await copyFile(join(REPO, "target", "debug", "examples", "windows-login-fixture.exe"),
        join(directory, "host.exe"));
      const extension = join(directory, "extension");
      await mkdir(extension);
      for (const file of ["manifest.json", "launch.mjs", "flow.mjs", "login.html", "login.css", "login.mjs"]) {
        await copyFile(join(REPO, "beyond-gate-browser-extension", file), join(extension, file));
      }
      for (const file of ["product-fixture.html", "product-fixture.mjs"]) {
        await copyFile(join(HERE, file), join(extension, file));
      }
      await registration("CheckAbsent", name, host, directory);
      await writeFile(join(directory, "native-host.json"), JSON.stringify({
        name: host, description: "Beyond Gate isolated product-flow fixture",
        path: join(directory, "host.exe"), type: "stdio",
        allowed_origins: [`chrome-extension://${recipe.extension_id}/`],
      }));
      await registration("Register", name, host, directory);
      registered = true;
      proxy = await rejectingProxy();
      stage = "browser-start";
      browser = await launchBrowser(name, {
        directory, proxyPort: proxy.address().port, protocolTimeout: 35000,
      });
      stage = "extension-load";
      const id = await browser.installExtension(extension);
      assert.equal(id, recipe.extension_id, "actual package identity must match the native origin");
      const page = await browser.newPage();
      await page.goto(`chrome-extension://${id}/product-fixture.html`);
      await page.waitForFunction(() => typeof globalThis.runProductFixture === "function", { timeout: 5000 });
      stage = "product-flow";
      const results = await cases(page, name, directory, host, recipe);
      await reportPublicBrowserSymbols(name, browser, directory);
      const driver = JSON.parse(await readFile(
        join(HERE, "node_modules", "puppeteer-core", "package.json"), "utf8",
      ));
      return { browser: name, browser_version: await browser.version(),
        browser_flavor: "regular-installed", os_version: release(),
        node_version: process.version, driver_version: driver.version,
        native_registration_scope: `HKCU ${name} NativeMessagingHosts; unique test key`,
        extension_load_method: "DevTools Extensions.loadUnpacked over pipe",
        headless: true, synthetic_only: true,
        product_controller: "passed", native_relay: "passed", core_completion: "passed",
        real_cookie_api_called: false, live_validated: false, native_saved: false,
        production_cli_terminal: "not-tested", cases: results, cleanup: "passed" };
    } catch (cause) {
      const error = new Error(`${name}: ${stage} failed`, { cause });
      error.launch = cause.launch;
      error.cleanup = cause.cleanup;
      throw error;
    }
  }, () => cleanupActions([
    ["native-host", async () => {
      if (registered) await registration("StopHost", name, host, directory);
    }],
    ["native-registration", async () => {
      if (registered) await registration("Remove", name, host, directory);
    }],
    ["browser-resources", () => cleanupProbe(name, browser, proxy, directory)],
  ]));
}

/** Builds the isolated product fixture through the repository's VS 2022 wrapper. */
async function buildHost() {
  await execute("pwsh", ["-NoProfile", "-File", join(REPO, "scripts", "cargo.ps1"),
    "build", "-p", "beyond-gate-core", "--example", "windows-login-fixture",
    "--features", "windows-login-fixture", "--locked", "--quiet"],
  { cwd: REPO, timeout: 600000, maxBuffer: 1024 * 1024 });
}

/** Builds with VS 2022 and reports Chrome and Edge evidence separately. */
export async function main({ build = buildHost, run = probe } = {}) {
  assert.equal(process.platform, "win32", "Windows product-flow fixture only");
  const args = process.argv.slice(2);
  assert.ok(args.length === 0 || (args.length === 2 && args[0] === "--browser"));
  const names = args.length === 0 ? ["chrome", "edge"] : [args[1]];
  assert.ok(names.every((name) => ["chrome", "edge"].includes(name)));
  const recipe = JSON.parse(await readFile(join(REPO, "docs", "fixtures", "windows-login-v1.json"), "utf8"));
  assert.equal(recipe.synthetic_only, true);
  await build();
  for (const name of names) {
    try {
      console.log(JSON.stringify({ status: "passed", ...await run(name, recipe) }));
    } catch (error) {
      console.error(JSON.stringify({ browser: name, status: "failed", error: error.message,
        launch: error.launch, cleanup_failures: error.cleanup,
        fixture_detail: safeLaunchCause(error.cause) }));
      process.exitCode = 1;
    }
  }
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  try {
    await main();
  } catch {
    console.error(JSON.stringify({ status: "failed", error: "Windows product fixture setup failed" }));
    process.exitCode = 1;
  }
}
