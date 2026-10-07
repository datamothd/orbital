"use strict";

const invoke = window.__TAURI__?.core.invoke;
const byId = (id) => document.getElementById(id);
let settings = null;
let busy = false;

function setStatus(message, error = false) {
  byId("status").textContent = message;
  byId("status").classList.toggle("error", error);
}

function updateButtons() {
  byId("refresh").disabled = busy || !invoke;
  byId("taskbar").disabled = busy || !settings;
  byId("compact-toggle").disabled = busy || !settings;
  byId("context-menu-toggle").disabled = busy || !settings?.classic_context_menu_supported;
}

function setPreferenceValue(id, value) {
  const element = byId(id);
  element.textContent = value;
  element.dataset.state = value === "Enabled" ? "enabled" : value === "Disabled" ? "disabled" : "other";
}

async function loadSettings() {
  settings = null;
  setPreferenceValue("alignment", "Unavailable");
  setPreferenceValue("compact", "Unavailable");
  setPreferenceValue("context-menu", "Unavailable");
  settings = await invoke("explorer_settings");
  setPreferenceValue("alignment", settings.taskbar_centered ? "Centered" : "Left");
  setPreferenceValue("compact", settings.compact_mode ? "Enabled" : "Disabled");
  setPreferenceValue("context-menu", settings.classic_context_menu_supported
    ? (settings.classic_context_menu ? "Enabled" : "Disabled")
    : "Requires Windows 11");
}

async function loadSystem() {
  const info = await invoke("system_info");
  const windowsVersion = info.windows_version.trim().toUpperCase();
  const testedOs = /^Windows 11 (Pro|Home)$/i.test(info.os.trim());
  const testedVersion = testedOs && windowsVersion === "25H2";
  const release = /^(\d{2})H([12])$/.exec(windowsVersion);
  const olderWindows11 = /^Windows 11(?: |$)/i.test(info.os.trim())
    && release !== null
    && (Number(release[1]) < 25 || (Number(release[1]) === 25 && Number(release[2]) < 2));
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
    const greenCheck = (label === "OS" && testedOs) || (label === "Windows" && testedVersion);
    const yellowDash = label === "Windows" && olderWindows11;
    if (greenCheck || yellowDash) {
      const check = document.createElement("span");
      check.className = greenCheck ? "tested-system-check" : "older-system-dash";
      check.textContent = greenCheck ? "✓" : "−";
      check.title = label === "OS"
        ? "Matches the tested OS edition: Windows 11 Pro"
        : greenCheck
          ? "Matches the tested Windows version: 25H2"
          : "Older Windows 11 release! Features are tested on version 25H2";
      check.setAttribute("role", "img");
      check.setAttribute("aria-label", check.title);
      description.append(check);
    }
    return [term, description];
  }));
}

async function refresh() {
  if (busy) return;
  busy = true;
  updateButtons();
  setStatus("Reading your system…");
  const results = await Promise.allSettled([loadSystem(), loadSettings()]);
  const errors = results.filter((result) => result.status === "rejected");
  if (results[0].status === "rejected") {
    byId("system").textContent = "System information unavailable.";
  }
  setStatus(errors.length ? errors.map((result) => String(result.reason)).join(" · ") : "Up to date.", errors.length > 0);
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
    setStatus("Applying preference and restarting Explorer…");
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
    setStatus(error || "Preference saved. Explorer restarted.", Boolean(error));
  } catch (reason) {
    setStatus(String(reason), true);
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

byId("context-menu-toggle").addEventListener("click", (event) => toggle(
  "toggle_classic_context_menu", `${settings?.classic_context_menu ? "Disable" : "Enable"} the Windows 10-style right-click menu?`,
  event.shiftKey,
));

if (invoke) {
  refresh();
} else {
  byId("system").textContent = "Open Orbital with npm run dev to read system information.";
  setPreferenceValue("alignment", "Unavailable");
  setPreferenceValue("compact", "Unavailable");
  setPreferenceValue("context-menu", "Unavailable");
  setStatus("The Rust backend is available inside the desktop app.", true);
}
