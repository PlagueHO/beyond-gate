/** Checks fixture stdout contracts without a browser, native process, or credential access. */
import assert from "node:assert/strict";
import { EventEmitter } from "node:events";
import { PassThrough } from "node:stream";
import { test } from "node:test";
import { execFile } from "node:child_process";
import { promisify } from "node:util";
import { readFile } from "node:fs/promises";
import { startCli } from "./product-flow.mjs";

const CONTEXT = "bg-windows-product-fixture-v1";
const READY = { fixture: CONTEXT, ready: true, namespace: "a".repeat(32) };
const RESULT = { fixture: CONTEXT, cli_outcome: "saved", service_requests: 1,
  in_memory_saved: true, live_validated: false, native_saved: false };

/** Provides only synthetic stdout and bounded process-close events. */
function child() {
  const process = new EventEmitter();
  process.stdout = new PassThrough();
  process.stderr = new PassThrough();
  process.exitCode = null;
  process.signalCode = null;
  process.kill = () => {
    process.signalCode = "SIGTERM";
    process.emit("close", null);
  };
  process.complete = (lines) => {
    process.stdout.end(lines.join("\n") + "\n");
    process.exitCode = 0;
    setImmediate(() => process.emit("close", 0));
  };
  return process;
}

test("exactly one readiness and completion record passes", async () => {
  const process = child();
  const pending = startCli("saved", () => process);
  process.stdout.write(JSON.stringify(READY) + "\n");
  const cli = await pending;
  process.complete([JSON.stringify(RESULT)]);
  assert.deepEqual(await cli.finish(), RESULT);
  await cli.stop();
});

test("invalid or duplicate output after readiness cannot pass completion", async () => {
  for (const lines of [
    [JSON.stringify(RESULT), "not-json"],
    [JSON.stringify(READY), JSON.stringify(RESULT)],
    [JSON.stringify(RESULT), JSON.stringify(RESULT)],
    [JSON.stringify({ ...RESULT, extra: true })],
    [JSON.stringify({ ...RESULT, service_requests: -1 })],
    [JSON.stringify({ ...RESULT, native_saved: true })],
    [],
  ]) {
    const process = child();
    const pending = startCli("saved", () => process);
    process.stdout.write(JSON.stringify(READY) + "\n");
    const cli = await pending;
    process.complete(lines);
    await assert.rejects(cli.finish());
    await cli.stop();
  }
});

test("completion before readiness and malformed namespaces fail safely", async () => {
  for (const message of [RESULT, { ...READY, namespace: "invalid" }]) {
    const process = child();
    const pending = startCli("saved", () => process);
    process.stdout.write(JSON.stringify(message) + "\n");
    await assert.rejects(pending, /invalid public metadata/);
  }
});

test("both runner entry points preserve failure exit codes and safe launch diagnostics", async () => {
  for (const script of ["run.mjs", "product-flow.mjs"]) {
    const url = new URL(script, import.meta.url).href;
    await assert.rejects(promisify(execFile)(process.execPath, ["--input-type=module", "-e", `
      import { main } from ${JSON.stringify(url)};
      process.argv = [process.execPath, "synthetic-runner", "--browser", "chrome"];
      await main({
        build: async () => {},
        run: async () => {
          const error = new Error("chrome: browser-start failed", {
            cause: new Error("synthetic-secret C:\\\\synthetic-private https://private.invalid"),
          });

          error.launch = { cause_reason: "timed-out", protocol_method: "Target.setAutoAttach" };
          error.cleanup = ["fixture-directory"];
          throw error;
        },
      });
    `], { timeout: 10000, maxBuffer: 4096 }), (error) => {
      assert.equal(error.code, 1);
      assert.equal(error.stdout, "");
      const record = JSON.parse(error.stderr);
      assert.equal(record.browser, "chrome");
      assert.equal(record.status, "failed");
      assert.equal(record.error, "chrome: browser-start failed");
      assert.deepEqual(record.launch, {
        cause_reason: "timed-out", protocol_method: "Target.setAutoAttach",
      });
      assert.deepEqual(record.cleanup_failures, ["fixture-directory"]);
      for (const privateText of ["synthetic-secret", "synthetic-private", "private.invalid"]) {
        assert.equal(error.stderr.includes(privateText), false);
      }
      return true;
    });
  }
});

test("mandatory CI cold launches all run and any failure remains a failed step", async () => {
  const workflow = await readFile(new URL("../../.github/workflows/ci.yml", import.meta.url), "utf8");
  const match = /- name: Exercise regular browser and real native host[\s\S]*?run: \|\r?\n((?: {10}[^\n]*\n)+)/.exec(workflow);
  assert.ok(match, "the required cold-launch step must exist");
  assert.equal(workflow.includes("continue-on-error"), false);
  const diagnosticCondition = "if: success() || (inputs.startup-stacks && !cancelled() && steps.browser-dependencies.outcome == 'success')";
  assert.equal(workflow.split(diagnosticCondition).length - 1, 4);
  const commands = match[1].replace(/^ {10}/gm, "");
  const invocation = "npm run feasibility --prefix .\\scripts\\browser-tests -- --browser $env:BEYOND_GATE_TEST_BROWSER";
  assert.ok(commands.includes(invocation));
  for (const failedCase of [0, 1, 2, 3]) {
    const script = commands.replace(invocation,
      `& '${process.execPath.replaceAll("'", "''")}' -e "process.exit($case === ${failedCase} ? 1 : 0)"`);
    try {
      const { stdout } = await promisify(execFile)("pwsh", ["-NoProfile", "-Command", script],
        { timeout: 10000, maxBuffer: 4096 });
      assert.equal(failedCase, 0, "a failed cold launch must not become success");
      assert.equal(stdout.trim().split(/\r?\n/).length, 3);
    } catch (error) {
      assert.notEqual(failedCase, 0);
      assert.equal(error.code, 1);
      assert.equal(error.stdout.trim().split(/\r?\n/).length, 3);
    }
  }
});
