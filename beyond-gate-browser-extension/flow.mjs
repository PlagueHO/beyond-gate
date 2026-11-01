/**
 * One-use browser handoff. No credential storage, logging, background cookie
 * access, content scripts, service fetches, or account navigation occurs here.
 * JavaScript/browser-owned copies cannot be guaranteed to erase from memory.
 */

export const HOST_NAME = "io.github.plagueho.beyond_gate";
export const EXTENSION_ID = "gnhdconfgjfdbbgkemadegjcjnnodefh";
export const PROFILE = "cobalt-session-v1";
export const COOKIE_QUERY = Object.freeze({
  url: "https://www.dndbeyond.com/", name: "CobaltSession", path: "/",
});

const CODES = new Set([
  "consent_required", "unauthorized_peer", "origin_rejected", "no_pending_login",
  "busy", "version_mismatch", "unsupported_profile", "protocol_error",
  "transaction_mismatch", "replay", "cookie_unavailable", "invalid_material",
  "cancelled", "timed_out", "disconnected", "session_rejected", "session_expired",
  "validation_unavailable", "unsupported_contract", "storage_unavailable",
  "storage_access", "storage_failure", "save_indeterminate",
]);

/** Reports only a fixed local category, never native input or API error text. */
export class FlowError extends Error {
  constructor(code) {
    super(code);
    this.code = code;
  }
}

/** Rejects missing, ambiguous, partitioned, or out-of-contract cookie candidates. */
export function selectedCookie(candidates, now = Date.now() / 1000) {
  if (!Array.isArray(candidates) || candidates.length !== 1) {
    throw new FlowError("cookie_unavailable");
  }
  const cookie = candidates[0];
  if (!cookie || cookie.name !== "CobaltSession" || cookie.domain !== ".dndbeyond.com" ||
      cookie.path !== "/" || cookie.hostOnly !== false || cookie.secure !== true ||
      cookie.httpOnly !== true || cookie.sameSite !== "unspecified" ||
      cookie.session !== false || Object.hasOwn(cookie, "partitionKey") ||
      !Number.isFinite(cookie.expirationDate) || cookie.expirationDate <= now) {
    throw new FlowError("cookie_unavailable");
  }
  if (typeof cookie.value !== "string" || cookie.value.length < 1 ||
      cookie.value.length > 2552 ||
      /[^\x21\x23-\x2b\x2d-\x3a\x3c-\x5b\x5d-\x7e]/.test(cookie.value)) {
    throw new FlowError("invalid_material");
  }
  return cookie;
}

/** Requires only the known JSON fields and fixed protocol version. */
function exact(message, fields) {
  if (!message || typeof message !== "object" || Array.isArray(message) ||
      message.protocol !== "bg-browser-handoff" || message.version !== 1 ||
      Object.keys(message).sort().join(",") !== fields.toSorted().join(",")) {
    throw new FlowError("protocol_error");
  }
}

/** Starts one consented pairing; the returned handle owns cancellation and completion. */
export function beginHandoff(api, { browser, consent, key }, notify) {
  if (consent !== true) throw new FlowError("consent_required");
  if (!["chrome", "edge"].includes(browser) || api.runtime.id !== EXTENSION_ID) {
    throw new FlowError("origin_rejected");
  }
  if (api.extension.inIncognitoContext !== false) throw new FlowError("cookie_unavailable");
  if (typeof key !== "string" || key.length !== 43 ||
      !/^[A-Za-z0-9_-]{42}[AEIMQUYcgkosw048]$/.test(key)) {
    throw new FlowError("invalid_material");
  }
  let state = "connecting";
  let transaction = null;
  let submitted = false;
  let pendingDeadline = Infinity;
  let deadline;
  let timer;
  let port;
  let resolve;
  const completion = new Promise((done) => { resolve = done; });

  /** Changes only fixed UI state; no credential reaches the notification callback. */
  function transition(next) {
    state = next;
    notify(next);
  }

  /** Closes one port and resolves once; only acknowledged completion reports evidence. */
  function finish(code) {
    if (state === "closed") return;
    transition("closed");
    clearTimeout(timer);
    if (port) {
      port.onMessage.removeListener(onMessage);
      port.onDisconnect.removeListener(onDisconnect);
      port.disconnect();
    }
    resolve(code === "saved"
      ? { code, validation: "service_validated", expiry: "unknown" }
      : { code });
  }

  /** Uses one monotonic phase bound within the original received pending lifetime. */
  function arm(milliseconds) {
    clearTimeout(timer);
    deadline = Math.min(pendingDeadline, performance.now() + milliseconds);
    timer = setTimeout(() => finish(submitted ? "save_indeterminate" : "timed_out"),
      Math.max(0, deadline - performance.now()));
  }

  /** Stops before submit immediately; afterwards waits for the native cancellation outcome. */
  function cancel() {
    if (state === "closed" || state === "cancelling") return;
    if (transaction === null) {
      finish("cancelled");
      return;
    }
    transition("cancelling");
    try {
      port.postMessage({
        protocol: "bg-browser-handoff", version: 1, type: "cancel", transaction,
      });
      if (!submitted) arm(5000);
    } catch {
      finish(submitted ? "save_indeterminate" : "cancelled");
    }
  }

  /** Handles bound pending/result/ack frames without exposing their input to the UI. */
  async function onMessage(message) {
    try {
      if (state === "closed") return;
      if (performance.now() >= deadline) {
        finish(submitted ? "save_indeterminate" : "timed_out");
        return;
      }
      if (message.type === "error") {
        exact(message, ["protocol", "version", "type", "transaction", "code"]);
        if (message.transaction !== transaction || !CODES.has(message.code)) {
          throw new FlowError("protocol_error");
        }
        finish(message.code);
        return;
      }
      if (message.type === "pending" && state === "connecting") {
        exact(message, ["protocol", "version", "type", "transaction", "profile", "expires_in_ms"]);
        if (message.profile !== PROFILE || typeof message.transaction !== "string" ||
            message.transaction.length !== 32 || !/^[a-f0-9]{32}$/.test(message.transaction) ||
            !Number.isInteger(message.expires_in_ms) ||
            message.expires_in_ms < 1 || message.expires_in_ms > 300000) {
          throw new FlowError("protocol_error");
        }
        transaction = message.transaction;
        pendingDeadline = performance.now() + message.expires_in_ms;
        transition("cookie-query");
        arm(15000);
        let candidates;
        try {
          candidates = await api.cookies.getAll({ ...COOKIE_QUERY });
        } catch {
          throw new FlowError("cookie_unavailable");
        }
        if (state !== "cookie-query") return;
        const cookie = selectedCookie(candidates);
        if (performance.now() >= deadline) {
          finish("timed_out");
          return;
        }
        submitted = true;
        transition("awaiting-result");
        arm(20000);
        port.postMessage({
          protocol: "bg-browser-handoff", version: 1, type: "submit",
          transaction, profile: PROFILE, value: cookie.value,
        });
        candidates = undefined;
        return;
      }
      if (message.type === "result" && state === "awaiting-result") {
        exact(message, ["protocol", "version", "type", "transaction", "outcome", "validation", "expiry"]);
        if (message.transaction !== transaction || message.outcome !== "saved" ||
            message.validation !== "service-validated" || message.expiry !== "unknown") {
          throw new FlowError("protocol_error");
        }
        transition("acknowledging");
        arm(5000);
        port.postMessage({
          protocol: "bg-browser-handoff", version: 1, type: "ack", transaction,
        });
        return;
      }
      if (message.type === "ack" && state === "acknowledging") {
        exact(message, ["protocol", "version", "type", "transaction"]);
        if (message.transaction !== transaction) throw new FlowError("protocol_error");
        finish("saved");
        return;
      }
      throw new FlowError("protocol_error");
    } catch (error) {
      const code = error instanceof FlowError ? error.code : "protocol_error";
      if (!submitted && state === "cookie-query" &&
          ["cookie_unavailable", "invalid_material"].includes(code)) {
        transition("stopping");
        arm(5000);
        try {
          port.postMessage({
            protocol: "bg-browser-handoff", version: 1, type: "error", transaction, code,
          });
        } catch {
          finish(code);
        }
      } else {
        finish(submitted ? "save_indeterminate" : code);
      }
    }
  }

  /** Consumes browser error state but never prints it or treats disconnect as success. */
  function onDisconnect() {
    void api.runtime.lastError;
    finish(submitted ? "save_indeterminate" : "disconnected");
  }

  try {
    port = api.runtime.connectNative(HOST_NAME);
    port.onMessage.addListener(onMessage);
    port.onDisconnect.addListener(onDisconnect);
    notify(state);
    arm(10000);
    port.postMessage({
      protocol: "bg-browser-handoff", version: 1, type: "pair",
      browser, profile: PROFILE, consent: true, key,
    });
    key = undefined;
  } catch {
    key = undefined;
    finish("disconnected");
  }
  return { cancel, completion };
}
