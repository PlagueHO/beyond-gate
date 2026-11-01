/**
 * Development-only cookie fixture. Permissions and every query are confined to
 * the invented host. This is not the product extension or an authentication path.
 */

export const SYNTHETIC = Object.freeze({
  url: "https://login.beyond-gate.invalid/",
  domain: "login.beyond-gate.invalid",
  name: "BeyondGateSyntheticSession",
  value: "synthetic-session",
});

/** Requires exactly the invented cookie and its agreed synthetic metadata. */
export function checkSyntheticCookie(candidates) {
  if (!Array.isArray(candidates) || candidates.length !== 1) {
    throw new Error("synthetic cookie is missing or ambiguous");
  }
  const cookie = candidates[0];
  if (
    cookie.name !== SYNTHETIC.name ||
    cookie.value !== SYNTHETIC.value ||
    cookie.domain !== SYNTHETIC.domain ||
    cookie.path !== "/" ||
    cookie.hostOnly !== true ||
    cookie.secure !== true ||
    cookie.httpOnly !== true ||
    cookie.sameSite !== "lax" ||
    cookie.session !== true ||
    Object.hasOwn(cookie, "partitionKey")
  ) {
    throw new Error("synthetic cookie metadata does not match");
  }
  return cookie;
}

/** Exercises actual browser cookie APIs without navigating or reading account data. */
globalThis.inspectSyntheticCookie = async function inspectSyntheticCookie() {
  await chrome.cookies.set({
    url: SYNTHETIC.url,
    name: SYNTHETIC.name,
    value: SYNTHETIC.value,
    path: "/",
    secure: true,
    httpOnly: true,
    sameSite: "lax",
  });
  const found = await chrome.cookies.getAll({
    name: SYNTHETIC.name,
    domain: SYNTHETIC.domain,
  });
  checkSyntheticCookie(found);
  return { cookie_api: "passed", matches: found.length, synthetic_only: true };
};

/** Requires the exact proposed fields without reflecting a native peer's input. */
function exactFields(message, fields) {
  if (!message || typeof message !== "object" || Array.isArray(message) ||
    message.protocol !== "bg-browser-handoff" || message.version !== 1 ||
    Object.keys(message).sort().join(",") !== fields.toSorted().join(",")) {
    throw new Error("synthetic native message shape is invalid");
  }
}

/** Exercises native messaging only with invented material and explicit test consent. */
export function syntheticHandoff(api, host, browser, consent) {
  if (consent !== true) {
    return Promise.resolve({
      code: "consent_required", cookie_reads: 0, received: false, validated: false, saved: false,
    });
  }
  if (!/^com\.beyondgate\.synthetic\.[a-f0-9]{32}$/.test(host) ||
    !["chrome", "edge"].includes(browser)) {
    throw new Error("synthetic host setup is invalid");
  }
  return new Promise((resolve) => {
    let state = "connecting";
    let transaction = null;
    let cookieReads = 0;
    let pendingDeadline = Infinity;
    let deadline;
    let timer;
    const port = api.runtime.connectNative(host);

    /** Keeps each test phase within its budget and the received pending deadline. */
    function arm(milliseconds) {
      clearTimeout(timer);
      deadline = Math.min(pendingDeadline, performance.now() + milliseconds);
      timer = setTimeout(() => finish(
        state === "awaiting-result" ? "save_indeterminate" : "timed_out",
      ), Math.max(0, deadline - performance.now()));
    }

    /** Closes this test's port and reports metadata, never a value or login success. */
    function finish(code) {
      if (state === "closed") return;
      const received = code === "unsupported_contract" && state === "awaiting-result";
      state = "closed";
      clearTimeout(timer);
      port.onMessage.removeListener(onMessage);
      port.onDisconnect.removeListener(onDisconnect);
      port.disconnect();
      resolve({ code, cookie_reads: cookieReads, received, validated: false, saved: false });
    }

    /** Handles a real pending reply before the single narrow synthetic-cookie query. */
    async function onMessage(message) {
      try {
        if (state === "closed") return;
        if (performance.now() >= deadline) {
          finish(state === "awaiting-result" ? "save_indeterminate" : "timed_out");
          return;
        }
        if (message.type === "pending" && state === "connecting") {
          exactFields(message, [
            "protocol", "version", "type", "transaction", "profile", "expires_in_ms",
          ]);
          if (message.profile !== "synthetic-v1" ||
            typeof message.transaction !== "string" ||
            !/^[a-f0-9]{32}$/.test(message.transaction) ||
            !Number.isInteger(message.expires_in_ms) ||
            message.expires_in_ms < 1 || message.expires_in_ms > 300000) {
            finish("protocol_error");
            return;
          }
          transaction = message.transaction;
          pendingDeadline = performance.now() + message.expires_in_ms;
          state = "cookie-query";
          arm(15000);
          cookieReads += 1;
          let candidates;
          try {
            candidates = await api.cookies.getAll({
              name: SYNTHETIC.name,
              domain: SYNTHETIC.domain,
            });
            checkSyntheticCookie(candidates);
          } catch {
            finish("cookie_unavailable");
            return;
          }
          if (state === "closed") return;
          if (performance.now() >= deadline) {
            finish("timed_out");
            return;
          }
          state = "awaiting-result";
          arm(5000);
          port.postMessage({
            protocol: "bg-browser-handoff", version: 1, type: "submit",
            transaction, profile: "synthetic-v1", value: candidates[0].value,
          });
          candidates = undefined;
          return;
        }
        exactFields(message, ["protocol", "version", "type", "transaction", "code"]);
        if (message.type !== "error" || message.transaction !== transaction ||
          !["no_pending_login", "unsupported_contract"].includes(message.code)) {
          finish("protocol_error");
          return;
        }
        finish(message.code);
      } catch {
        finish("protocol_error");
      }
    }

    /** Consumes the browser error without printing it or treating disconnect as success. */
    function onDisconnect() {
      void api.runtime.lastError;
      finish(state === "awaiting-result" ? "save_indeterminate" : "disconnected");
    }

    port.onMessage.addListener(onMessage);
    port.onDisconnect.addListener(onDisconnect);
    arm(10000);
    port.postMessage({
      protocol: "bg-browser-handoff", version: 1, type: "hello",
      browser, profile: "synthetic-v1", consent: true,
    });
  });
}

/** Exposes only the test's compiled synthetic operation to the automation context. */
globalThis.runSyntheticHandoff = (host, browser, consent) =>
  syntheticHandoff(chrome, host, browser, consent);
