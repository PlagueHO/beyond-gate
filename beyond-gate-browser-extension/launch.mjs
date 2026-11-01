/** Opens only the extension's own form after an explicit toolbar action. */
chrome.action.onClicked.addListener(async () => {
  try {
    await chrome.windows.create({
      url: chrome.runtime.getURL("login.html"),
      type: "popup",
      width: 640,
      height: 780,
      focused: true,
    });
    await chrome.action.setBadgeText({ text: "" });
    await chrome.action.setTitle({ title: "Open Beyond Gate login" });
  } catch {
    await chrome.action.setBadgeText({ text: "!" });
    await chrome.action.setTitle({ title: "Login window could not open. Try again." });
  }
});
