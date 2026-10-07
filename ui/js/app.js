"use strict";

const invoke = window.__TAURI__?.core.invoke;
const byId = (id) => document.getElementById(id);
let settings = null;
let busy = false;

function status(message, error = false) {
  byId("status").textContent = message;
  byId("status").classList.toggle("error", error);
}

function updateButtons() {
  byId("refresh").disabled = busy || !invoke;
  byId("taskbar").disabled = busy || !settings;
  byId("compact-toggle").disabled = busy || !settings;
}

async function loadSettings() {
  settings = null;
  byId("alignment").textContent = "Unavailable";
  byId("compact").textContent = "Unavailable";
  settings = await invoke("explorer_settings");
  byId("alignment").textContent = settings.taskbar_centered ? "Centered" : "Left";
  byId("compact").textContent = settings.compact_mode ? "Enabled" : "Disabled";
}

async function loadSystem() {
  const info = await invoke("system_info");
  const rows = [
    ["Host", info.host], ["OS", info.os], ["Windows", info.windows_version],
    ["CPU", info.cpu], ["Threads", info.threads],
    ["Memory", `${info.used_memory_gib.toFixed(1)} / ${info.total_memory_gib.toFixed(1)} GiB`],
  ];
  byId("system").replaceChildren(...rows.flatMap(([label, value]) => {
    const term = document.createElement("dt");
    const description = document.createElement("dd");
    term.textContent = label;
    description.textContent = value;
    return [term, description];
  }));
}

async function refresh() {
  if (busy) return;
  busy = true;
  updateButtons();
  status("Reading your system…");
  const results = await Promise.allSettled([loadSystem(), loadSettings()]);
  const errors = results.filter((result) => result.status === "rejected");
  if (results[0].status === "rejected") {
    byId("system").textContent = "System information unavailable.";
  }
  status(errors.length ? errors.map((result) => String(result.reason)).join(" · ") : "Up to date.", errors.length > 0);
  busy = false;
  updateButtons();
}

function confirmChange(description) {
  const dialog = byId("confirmation");
  byId("confirm-description").textContent = description;
  dialog.returnValue = "cancel";
  return new Promise((resolve) => {
    dialog.addEventListener("close", () => resolve(dialog.returnValue === "apply"), { once: true });
    dialog.showModal();
  });
}

async function toggle(command, description, skipConfirmation = false) {
  if (busy || !settings) return;
  busy = true;
  updateButtons();
  try {
    if (!skipConfirmation && !await confirmChange(description)) return;
    status("Applying preference and restarting Explorer…");
    let error = null;
    try {
      await invoke(command);
    } catch (reason) {
      error = String(reason);
    }
    try {
      await loadSettings();
    } catch (reason) {
      error = [error, `Could not refresh preferences: ${reason}`].filter(Boolean).join(" · ");
    }
    status(error || "Preference saved. Explorer restarted.", Boolean(error));
  } catch (reason) {
    status(String(reason), true);
  } finally {
    busy = false;
    updateButtons();
  }
}

byId("refresh").addEventListener("click", refresh);
byId("taskbar").addEventListener("click", (event) => toggle(
  "toggle_taskbar_alignment", `Move taskbar icons to the ${settings?.taskbar_centered ? "left" : "center"}?`,
  event.shiftKey,
));
byId("compact-toggle").addEventListener("click", (event) => toggle(
  "toggle_explorer_compact_mode", `${settings?.compact_mode ? "Disable" : "Enable"} compact spacing in File Explorer?`,
  event.shiftKey,
));

if (invoke) {
  refresh();
} else {
  byId("system").textContent = "Open Orbital with npm run dev to read system information.";
  byId("alignment").textContent = "Unavailable";
  byId("compact").textContent = "Unavailable";
  status("The Rust backend is available inside the desktop app.", true);
}
