/** Offline checks for the development-only extension's narrow synthetic scope. */
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { test } from "node:test";
import { checkSyntheticCookie, syntheticHandoff, SYNTHETIC } from "./extension/service_worker.js";

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
