/** Binds the explicit form to one port; no cookie or account operation runs on load. */
import { beginHandoff, FlowError } from "./flow.mjs";

const form = document.querySelector("#pairing-form");
const fields = document.querySelector("#pairing-fields");
const key = document.querySelector("#key");
const browser = document.querySelector("#browser");
const consent = document.querySelector("#consent");
const cancel = document.querySelector("#cancel");
const status = document.querySelector("#status");
const heading = document.querySelector("#status-heading");
const text = document.querySelector("#status-text");
let active;
browser.value = navigator.userAgent.includes("Edg/") ? "edge" : "chrome";

const STATES = {
  connecting: ["Pairing with the terminal", "Keep both windows open. No cookie has been read."],
  "cookie-query": ["Selecting the approved cookie", "One narrow cookie query is in progress."],
  "awaiting-result": ["Validating and saving", "The terminal must confirm service validation and protected storage."],
  acknowledging: ["Confirming the saved session", "Waiting for final receipt confirmation. Do not close this window yet."],
  cancelling: ["Cancelling login", "Waiting for the terminal to report whether a write started."],
  stopping: ["Stopping the handoff", "The cookie selection failed. Waiting for the terminal to close this login."],
};
const ERRORS = {
  consent_required: "Approve the cookie read and protected save before pairing.",
  origin_rejected: "The installed extension identity does not match the native host.",
  invalid_material: "The pairing key or selected cookie does not match the required format.",
  cookie_unavailable: "No single approved cookie is available. Sign in in this regular browser profile.",
  no_pending_login: "Start a new interactive beyond-gate auth login command.",
  unauthorized_peer: "Pairing failed. Start a new login and use its new key.",
  busy: "Another login owns the pending slot. Finish or cancel that login first.",
  session_rejected: "D&D Beyond rejected the session. Sign in again before starting a new login.",
  session_expired: "The session expired. Sign in again before starting a new login.",
  validation_unavailable: "Service validation could not complete. Start a new login later.",
  storage_unavailable: "Protected storage is unavailable. Resolve the OS storage problem first.",
  storage_access: "Protected storage access was denied. Resolve the OS access problem first.",
  storage_failure: "Protected storage setup failed. Resolve the OS storage problem first.",
  save_indeterminate: "The save is unconfirmed. Do not assume storage is unchanged. Local auth status cannot prove service usability.",
  cancelled: "Login was cancelled before saving.",
  timed_out: "The login deadline expired before saving. Start a new login.",
  disconnected: "The native connection closed. Check host registration and start a new login.",
};

/** Shows only authored copy and fixed categories, never native or cookie data. */
function show(title, detail, error = false, success = false) {
  heading.textContent = title;
  text.textContent = detail;
  status.dataset.error = String(error);
  status.dataset.success = String(success);
}

/** Clears the form's key before starting any native operation. */
form.addEventListener("submit", async (event) => {
  event.preventDefault();
  if (active) return;
  const input = { browser: browser.value, consent: consent.checked, key: key.value };
  key.value = "";
  try {
    active = beginHandoff(chrome, input, (state) => {
      if (STATES[state]) show(...STATES[state]);
    });
  } catch (error) {
    show("Login did not start", ERRORS[error instanceof FlowError ? error.code : "protocol_error"] ??
      "The login setup is invalid. Check the installed component.", true);
    input.key = undefined;
    return;
  }
  input.key = undefined;
  fields.disabled = true;
  cancel.hidden = false;
  const result = await active.completion;
  active = undefined;
  cancel.hidden = true;
  if (result.code === "saved") {
    show("Session saved", "Service validation and protected storage are confirmed. Cookie expiry remains unknown.", false, true);
  } else {
    show("Login did not complete", ERRORS[result.code] ??
      "The protocol or installed component is unsupported. Start a new login after checking setup.", true);
  }
  status.focus();
});

/** Requests cancellation without silently treating an unconfirmed write as unchanged. */
cancel.addEventListener("click", () => {
  active?.cancel();
  cancel.disabled = true;
});

/** Closing the owned window stops the operation and its native port. */
window.addEventListener("pagehide", () => active?.cancel());
