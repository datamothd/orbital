"use strict";

const invoke = window.__TAURI__?.core.invoke;
const byId = (id) => document.getElementById(id);
let settings = null;
let busy = false;

const explorerPreferences = [
  { id: "taskbar-centering", field: "disable_taskbar_centering", enabledLabel: "Left", disabledLabel: "Centered", apply: "Move taskbar icons to the left?", restore: "Center taskbar icons?" },
  { id: "explorer-spacing", field: "disable_explorer_spacing", apply: "Use compact spacing in File Explorer?", restore: "Restore extra spacing in File Explorer?" },
  { id: "modern-context-menu", field: "disable_modern_context_menu", apply: "Use the classic Windows 10-style right-click menu?", restore: "Restore the Windows 11 right-click menu?", windows11: true },
  { id: "recent-files", field: "hide_recent_files", apply: "Hide recent files and recommendations in Quick Access?", restore: "Show recent files and recommendations in Quick Access?" },
  { id: "frequent-folders", field: "hide_frequent_folders", apply: "Hide frequently used folders in Quick Access?", restore: "Show frequently used folders in Quick Access?" },
  { id: "office-files", field: "hide_office_files", apply: "Hide Office.com cloud files in Quick Access?", restore: "Show Office.com cloud files in Quick Access?", windows11: true },
  { id: "home-folder", field: "hide_home_folder", apply: "Hide Home in the navigation pane?", restore: "Show Home in the navigation pane?", windows11: true },
  { id: "gallery", field: "hide_gallery", apply: "Hide Gallery in the navigation pane?", restore: "Show Gallery in the navigation pane?", windows11: true },
  { id: "shortcut-arrow", field: "hide_shortcut_arrow", apply: "Hide the arrow overlay on desktop shortcuts?", restore: "Show the arrow overlay on desktop shortcuts?" },
];

function setStatus(message, error = false) {
  const administrator = error && message.includes("requires administrator access");
  const notice = byId("administrator-status");
  const status = byId("status");
  notice.hidden = !administrator;
  notice.textContent = administrator ? message : "";
  status.textContent = administrator ? "" : message;
  status.hidden = administrator || !message;
  status.classList.toggle("error", error && !administrator);
}

function updateButtons() {
  byId("refresh").disabled = busy || !invoke;
  byId("apply-recommended").disabled = busy || !settings;
  byId("restore-original").disabled = busy || !settings;
  for (const preference of explorerPreferences) {
    byId(`${preference.id}-toggle`).disabled = busy || !settings
      || (preference.windows11 && !settings.windows11_supported);
  }
}

function setPreferenceValue(id, value) {
  const element = byId(id);
  element.textContent = value;
  element.dataset.state = value === "Enabled" ? "enabled" : value === "Disabled" ? "disabled" : "other";
}

async function loadSettings() {
  settings = null;
  for (const preference of explorerPreferences) {
    setPreferenceValue(preference.id, "Unavailable");
  }
  settings = await invoke("explorer_settings");
  for (const preference of explorerPreferences) {
    setPreferenceValue(preference.id, preference.windows11 && !settings.windows11_supported
      ? "Requires Windows 11"
      : settings[preference.field] === null ? "Custom or missing overlay icon"
      : settings[preference.field] ? (preference.enabledLabel || "Enabled") : (preference.disabledLabel || "Disabled"));
  }
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
  setStatus(errors.map((result) => String(result.reason)).join(" · "), errors.length > 0);
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

function showApplyResult(count, error) {
  const completed = typeof count === "number";
  const errorNotice = byId("apply-result-error");
  byId("apply-result-title").textContent = completed
    ? "Recommended settings applied" : "Could not complete recommendations";
  byId("apply-result-description").textContent = completed
    ? `${count} ${count === 1 ? "change" : "changes"} made.${count ? " Explorer restarted." : " All recommended settings are already enabled."}`
    : "The number of changes could not be confirmed.";
  errorNotice.hidden = !error;
  errorNotice.textContent = error || "";
  byId("apply-result").showModal();
}

async function toggle(command, description, skipConfirmation = false, success = "Preference saved. Explorer restarted.") {
  if (busy || !settings) return;
  busy = true;
  updateButtons();
  try {
    if (!skipConfirmation && !await confirmChange(description)) return;
    setStatus("Saving preferences and restarting Explorer…");
    let error = null;
    let changed;
    try {
      changed = await invoke(command);
    } catch (reason) {
      error = String(reason);
    }
    try {
      await loadSettings();
    } catch (reason) {
      error = [error, `Could not refresh preferences: ${reason}`].filter(Boolean).join(" · ");
    }
    setStatus(error || (changed === false || changed === 0 ? "" : success), Boolean(error));
    if (command === "apply_recommended_settings") showApplyResult(changed, error);
  } catch (reason) {
    setStatus(String(reason), true);
  } finally {
    busy = false;
    updateButtons();
  }
}

byId("refresh").addEventListener("click", refresh);
byId("apply-recommended").addEventListener("click", () => toggle(
  "apply_recommended_settings", "", true, "Recommended settings enabled. Explorer restarted.",
));
byId("restore-original").addEventListener("click", () => toggle(
  "restore_original_settings", "", true, "Original settings restored. Explorer restarted.",
));
for (const preference of explorerPreferences) {
  byId(`${preference.id}-toggle`).addEventListener("click", (event) => toggle(
    `toggle_${preference.field}`,
    settings?.[preference.field] ? preference.restore : preference.apply,
    event.shiftKey,
  ));
}

if (invoke) {
  refresh();
} else {
  byId("system").textContent = "Open Orbital with npm run dev to read system information.";
  for (const preference of explorerPreferences) {
    setPreferenceValue(preference.id, "Unavailable");
  }
  setStatus("The Rust backend is available inside the desktop app.", true);
}
