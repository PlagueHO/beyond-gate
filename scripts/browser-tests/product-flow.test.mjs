/** Checks fixture stdout contracts without a browser, native process, or credential access. */
import assert from "node:assert/strict";
import { EventEmitter } from "node:events";
import { PassThrough } from "node:stream";
import { test } from "node:test";
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
