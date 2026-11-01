/** Offline product logic checks. Every cookie, key, and native claim is invented. */
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { createHash, createPublicKey } from "node:crypto";
import { test } from "node:test";
import {
  beginHandoff, selectedCookie, COOKIE_QUERY, EXTENSION_ID, HOST_NAME, PROFILE,
} from "../flow.mjs";

const fixture = JSON.parse(await readFile(new URL("../../docs/fixtures/windows-login-v1.json", import.meta.url), "utf8"));
const messages = fixture.messages;

/** Owns mock listeners without a real browser or native process. */
function event() {
  const listeners = new Set();
  return {
    addListener: (listener) => listeners.add(listener),
    removeListener: (listener) => listeners.delete(listener),
    emit: (message) => Promise.all([...listeners].map((listener) => listener(message))),
    size: () => listeners.size,
  };
}

/** Records only invented frames and query ordering from the actual product controller. */
function nativeApi() {
  const sent = [];
  const queries = [];
  let closed = 0;
  let connected = 0;
  const port = {
    onMessage: event(), onDisconnect: event(),
    postMessage(message) { sent.push(message); },
    disconnect() { closed += 1; },
  };
  return {
    sent, queries, port, closed: () => closed, connected: () => connected,
    runtime: {
      id: EXTENSION_ID,
      connectNative(host) { assert.equal(host, HOST_NAME); connected += 1; return port; },
    },
    extension: { inIncognitoContext: false },
    cookies: { getAll: async (query) => { queries.push(query); return [{ ...fixture.cookie }]; } },
  };
}

/** Uses the exact shared invented key and explicit consent, not a real session. */
function start(api, browser = "edge", changes = {}, notify = () => {}) {
  return beginHandoff(api, { browser, consent: true, key: messages.pair.key, ...changes }, notify);
}

/** Builds one fixed native error with no dependency or account text. */
function error(code, transaction = messages.pending.transaction) {
  return { protocol: "bg-browser-handoff", version: 1, type: "error", transaction, code };
}

test("public package identity and permissions match the native contract", async () => {
  const manifest = JSON.parse(await readFile(new URL("../manifest.json", import.meta.url), "utf8"));
  const publicKey = createPublicKey({ key: Buffer.from(manifest.key, "base64"), format: "der", type: "spki" });
  assert.equal(publicKey.type, "public");
  const hash = createHash("sha256").update(Buffer.from(manifest.key, "base64")).digest("hex").slice(0, 32);
  const id = hash.replace(/[0-9a-f]/g, (digit) => String.fromCharCode(97 + parseInt(digit, 16)));
  assert.equal(id, EXTENSION_ID);
  assert.equal(id, fixture.extension_id);
  assert.equal(HOST_NAME, fixture.host_name);
  assert.equal(PROFILE, fixture.profile);
  assert.deepEqual(COOKIE_QUERY, fixture.query);
  assert.deepEqual(manifest.permissions, ["cookies", "nativeMessaging"]);
  assert.deepEqual(manifest.host_permissions, ["https://dndbeyond.com/*", "https://www.dndbeyond.com/*"]);
  assert.equal(manifest.incognito, "not_allowed");
  assert.equal(manifest.content_scripts, undefined);
  assert.equal(manifest.action.default_popup, undefined);
});

test("toolbar action opens a separate owned form without native or cookie access", async () => {
  let clicked;
  const windows = [];
  const badges = [];
  globalThis.chrome = {
    runtime: { getURL: (name) => `chrome-extension://${EXTENSION_ID}/${name}` },
    windows: { create: async (options) => { windows.push(options); } },
    action: {
      onClicked: { addListener: (listener) => { clicked = listener; } },
      setBadgeText: async (badge) => badges.push(badge),
      setTitle: async () => {},
    },
  };
  try {
    await import("../launch.mjs");
    assert.deepEqual(windows, []);
    await clicked();
    assert.deepEqual(windows, [{
      url: `chrome-extension://${EXTENSION_ID}/login.html`,
      type: "popup", width: 640, height: 780, focused: true,
    }]);
    assert.deepEqual(badges, [{ text: "" }]);
  } finally {
    delete globalThis.chrome;
  }
});

test("cookie fields and exact native byte limit are enforced independently", () => {
  assert.equal(selectedCookie([fixture.cookie], 1700000000), fixture.cookie);
  assert.doesNotThrow(() => selectedCookie([{ ...fixture.cookie, value: "x".repeat(2552) }]));
  for (const candidates of [
    [], [fixture.cookie, fixture.cookie],
    ...[
      { name: "Other" }, { domain: "example.invalid" }, { path: "/other" },
      { hostOnly: true }, { secure: false }, { httpOnly: false }, { sameSite: "none" },
      { session: true }, { expirationDate: undefined }, { expirationDate: Infinity },
      { expirationDate: 0 }, { partitionKey: undefined }, { value: "" },
      { value: "x".repeat(2553) }, { value: "bad;value" }, { value: "\u00e9" },
      { value: "synthetic-session\n" }, { value: "synthetic-session\r" },
    ].map((change) => [{ ...fixture.cookie, ...change }]),
  ]) assert.throws(() => selectedCookie(candidates));
});

test("consent, identity, regular context, and canonical key gate all API operations", () => {
  for (const mode of ["consent", "identity", "incognito", "key", "encoding"]) {
    const api = nativeApi();
    if (mode === "identity") api.runtime.id = "a".repeat(32);
    if (mode === "incognito") api.extension.inIncognitoContext = true;
    const changes = mode === "consent" ? { consent: false } :
      mode === "key" ? { key: "synthetic-invalid" } :
      mode === "encoding" ? { key: `${"A".repeat(42)}B` } : {};
    assert.throws(() => start(api, "edge", changes));
    assert.equal(api.connected(), 0);
    assert.equal(api.queries.length, 0);
  }
});

test("both declared browsers use shared recipes and require final acknowledged success", async () => {
  for (const browser of ["edge", "chrome"]) {
    const api = nativeApi();
    const states = [];
    const flow = start(api, browser, {}, (state) => states.push(state));
    let done = false;
    flow.completion.then(() => { done = true; });
    assert.deepEqual(api.sent, [{ ...messages.pair, browser }]);
    assert.equal(api.queries.length, 0);
    await api.port.onMessage.emit(messages.pending);
    assert.deepEqual(api.queries, [fixture.query]);
    assert.deepEqual(api.sent[1], messages.submit);
    await api.port.onMessage.emit(messages.result);
    assert.deepEqual(api.sent[2], messages.ack);
    assert.equal(done, false);
    await api.port.onMessage.emit(messages.ack);
    assert.deepEqual(await flow.completion, { code: "saved", validation: "service_validated", expiry: "unknown" });
    assert.deepEqual(states, ["connecting", "cookie-query", "awaiting-result", "acknowledging", "closed"]);
    assert.equal(api.closed(), 1);
    assert.equal(api.port.onMessage.size(), 0);
    assert.equal(api.port.onDisconnect.size(), 0);
  }
});

test("no pending login and malformed early frames never permit cookie access", async () => {
  for (const message of [
    error("no_pending_login", null), null, messages.result, messages.ack,
    { ...messages.pending, profile: "synthetic-v1" },
    { ...messages.pending, version: 2 }, { ...messages.pending, expires_in_ms: 0 },
    { ...messages.pending, expires_in_ms: 300001 }, { ...messages.pending, extra: true },
    { ...messages.pending, transaction: `${messages.pending.transaction}\n` },
  ]) {
    const api = nativeApi();
    const flow = start(api);
    await api.port.onMessage.emit(message);
    assert.notEqual((await flow.completion).code, "saved");
    assert.equal(api.queries.length, 0);
  }
});

test("failed selection sends one bound safe failure and never submits", async () => {
  for (const rejects of [false, true]) {
    const api = nativeApi();
    api.cookies.getAll = async (query) => {
      api.queries.push(query);
      if (rejects) throw new Error("synthetic-private-error");
      return [];
    };
    const flow = start(api);
    await api.port.onMessage.emit(messages.pending);
    assert.deepEqual(api.sent[1], error("cookie_unavailable"));
    await api.port.onMessage.emit(error("cookie_unavailable"));
    assert.deepEqual(await flow.completion, { code: "cookie_unavailable" });
    assert.equal(api.queries.length, 1);
    assert.equal(api.sent.some((message) => message.type === "submit"), false);
  }
});

test("duplicate pending cannot query or submit a second time", async () => {
  const api = nativeApi();
  const flow = start(api);
  await api.port.onMessage.emit(messages.pending);
  await api.port.onMessage.emit(messages.pending);
  assert.deepEqual(await flow.completion, { code: "save_indeterminate" });
  assert.equal(api.queries.length, 1);
  assert.equal(api.sent.filter((message) => message.type === "submit").length, 1);
});

test("cancellation while cookie query is pending prevents its later submit", async () => {
  const api = nativeApi();
  let release;
  api.cookies.getAll = () => new Promise((done) => { release = done; });
  const flow = start(api);
  const pending = api.port.onMessage.emit(messages.pending);
  flow.cancel();
  assert.equal(api.sent[1].type, "cancel");
  await api.port.onMessage.emit(error("cancelled"));
  release([fixture.cookie]);
  await pending;
  assert.deepEqual(await flow.completion, { code: "cancelled" });
  assert.equal(api.sent.some((message) => message.type === "submit"), false);
});

test("post-submit cancellation waits for confirmed outcome and never trusts a late result", async () => {
  for (const response of [error("cancelled"), error("save_indeterminate"), messages.result]) {
    const api = nativeApi();
    const flow = start(api);
    await api.port.onMessage.emit(messages.pending);
    flow.cancel();
    assert.equal(api.closed(), 0);
    assert.equal(api.sent[2].type, "cancel");
    await api.port.onMessage.emit(response);
    assert.equal((await flow.completion).code,
      response.code === "cancelled" ? "cancelled" : "save_indeterminate");
  }
});

test("disconnect, wrong result, and wrong acknowledgement cannot report saved", async () => {
  for (const mode of ["disconnect", "result", "ack"]) {
    const api = nativeApi();
    const flow = start(api);
    await api.port.onMessage.emit(messages.pending);
    if (mode === "disconnect") await api.port.onDisconnect.emit();
    if (mode === "result") await api.port.onMessage.emit({ ...messages.result, validation: "unverified" });
    if (mode === "ack") {
      await api.port.onMessage.emit(messages.result);
      await api.port.onMessage.emit({ ...messages.ack, transaction: "b".repeat(32) });
    }
    assert.deepEqual(await flow.completion, { code: "save_indeterminate" });
  }
});

test("result and acknowledgement waits have exact fixed caps", async (context) => {
  context.mock.timers.enable({ apis: ["setTimeout"] });
  let now = 0;
  context.mock.method(performance, "now", () => now);
  for (const [phase, bound] of [["pair", 10000], ["result", 20000], ["ack", 5000]]) {
    const api = nativeApi();
    const flow = start(api);
    if (phase !== "pair") await api.port.onMessage.emit(messages.pending);
    if (phase === "ack") await api.port.onMessage.emit(messages.result);
    now += bound;
    context.mock.timers.tick(bound);
    assert.deepEqual(await flow.completion, { code: phase === "pair" ? "timed_out" : "save_indeterminate" });
  }
});
