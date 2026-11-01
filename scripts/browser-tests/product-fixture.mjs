/** Actual product controller with explicit invented cookie and fixture-only host routing. */
import { beginHandoff, COOKIE_QUERY, HOST_NAME } from "./flow.mjs";

/** Uses the real browser port, but never invokes the real browser cookie API. */
globalThis.runProductFixture = async function (host, browser, recipe, mode) {
  if (!/^com\.beyondgate\.synthetic\.[a-f0-9]{32}$/.test(host) ||
      recipe.synthetic_only !== true || recipe.cookie.value !== "synthetic-session") {
    throw new Error("invalid synthetic product fixture");
  }
  let reads = 0;
  let flow;
  const api = {
    extension: { inIncognitoContext: chrome.extension.inIncognitoContext },
    cookies: {
      async getAll(query) {
        if (JSON.stringify(query) !== JSON.stringify(COOKIE_QUERY)) {
          throw new Error("unexpected fixture cookie query");
        }
        reads++;
        return [{ ...recipe.cookie }];
      },
    },
    runtime: {
      id: chrome.runtime.id,
      get lastError() { return chrome.runtime.lastError; },
      connectNative(name) {
        if (name !== HOST_NAME) throw new Error("unexpected product host");
        const port = chrome.runtime.connectNative(host);
        return {
          onMessage: port.onMessage,
          onDisconnect: port.onDisconnect,
          disconnect: () => port.disconnect(),
          postMessage(message) {
            if (mode === "lost-ack" && message.type === "ack") {
              port.disconnect();
            } else {
              port.postMessage(message);
            }
          },
        };
      },
    },
  };
  flow = beginHandoff(api, {
    browser, consent: true, key: recipe.messages.pair.key,
  }, (state) => {
    if (mode === "cancelled" && state === "awaiting-result") {
      setTimeout(() => flow.cancel(), 0);
    }
  });
  const result = await flow.completion;
  return { code: result.code, cookie_queries: reads,
    real_cookie_api_called: false, live_validated: false, native_saved: false };
};
