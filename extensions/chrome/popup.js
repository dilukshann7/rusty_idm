import { api as chrome } from "./browser-api.js";
const toggle = document.querySelector("#capture");
const status = document.querySelector("#status");
const origins = ["http://*/*", "https://*/*"];
async function refresh() {
  try {
    const result = await chrome.runtime.sendMessage({ type: "status" });
    toggle.checked = result.captureEnabled;
    status.textContent = result.hostAvailable
      ? result.guiRunning
        ? "Rusty is connected."
        : "Host ready. Rusty opens when needed."
      : "Native host unavailable. Follow setup instructions.";
    document.querySelector("#pending").textContent = result.attention
      ? `${result.attention} handoff(s) need attention. Check your browser downloads.`
      : result.pending
        ? `${result.pending} handoff(s) pending.`
        : "No pending handoffs.";
  } catch {
    status.textContent = "Connection unavailable. Reopen this popup.";
  }
}
toggle.addEventListener("change", async () => {
  const enabled = toggle.checked;
  toggle.disabled = true;
  try {
    // This call stays directly in the user gesture; page access is optional.
    const granted = !enabled || (await chrome.permissions.request({ origins }));
    await chrome.runtime.sendMessage({
      type: "capture-preference",
      enabled: enabled && granted,
    });
    if (!granted)
      status.textContent = "Access was declined. Automatic capture is off.";
  } catch {
    status.textContent = "Capture preference could not be saved.";
  } finally {
    toggle.disabled = false;
    await refresh();
  }
});
document.querySelector("#recover").addEventListener("click", async () => {
  await chrome.runtime.sendMessage({ type: "recover" });
  await refresh();
});
void refresh();
