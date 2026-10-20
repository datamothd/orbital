"use strict";

const invoke = window.__TAURI__?.core.invoke;
const byId = (id) => document.getElementById(id);
let settings = null;
let busy = false;
let skipPrompts = false;
try {
  skipPrompts = localStorage.getItem("orbital.skipPrompts") === "true";
} catch {
}

function renderSkipPrompts() {
  byId("skip-prompts-toggle").setAttribute("aria-checked", String(skipPrompts));
  setPreferenceValue("skip-prompts-value", skipPrompts ? "Enabled" : "Disabled");
}

byId("skip-prompts-toggle").addEventListener("click", () => {
  skipPrompts = !skipPrompts;
  renderSkipPrompts();
  try {
    localStorage.setItem("orbital.skipPrompts", String(skipPrompts));
  } catch {
  }
});
renderSkipPrompts();

const explorerPreferences = [
  { id: "taskbar-centering", field: "disable_taskbar_centering", enabledLabel: "Left", disabledLabel: "Centered", apply: "Move taskbar icons to the left?", restore: "Center taskbar icons?" },
  { id: "explorer-spacing", field: "disable_explorer_spacing", apply: "Use compact spacing in File Explorer?", restore: "Restore extra spacing in File Explorer?" },
  { id: "modern-context-menu", field: "disable_modern_context_menu", apply: "Use the classic Windows 10-style right-click menu?", restore: "Restore the Windows 11 right-click menu?", windows11: true },
  { id: "recent-files", field: "hide_recent_files", apply: "Hide recent files and recommendations in Quick Access?", restore: "Show recent files and recommendations in Quick Access?" },
  { id: "frequent-folders", field: "hide_frequent_folders", apply: "Hide frequently used folders in Quick Access?", restore: "Show frequently used folders in Quick Access?" },
  { id: "office-files", field: "hide_office_files", apply: "Hide Office.com cloud files in Quick Access?", restore: "Show Office.com cloud files in Quick Access?", windows11: true },
  { id: "home-folder", field: "hide_home_folder", apply: "Hide Home in the navigation pane?", restore: "Show Home in the navigation pane?", windows11: true, requiresAdmin: true },
  { id: "gallery", field: "hide_gallery", apply: "Hide Gallery in the navigation pane?", restore: "Show Gallery in the navigation pane?", windows11: true, requiresAdmin: true },
  { id: "shortcut-arrow", field: "hide_shortcut_arrow", apply: "Hide the arrow overlay on desktop shortcuts?", restore: "Show the arrow overlay on desktop shortcuts?", requiresAdmin: true },
];

const folderViews = [
  { id: "folder-sort", picker: folderSorting, field: "folder_sort", command: "set_folder_sort", argument: "sort", label: "sorting" },
  { id: "folder-grouping", picker: folderGrouping, field: "folder_grouping", command: "set_folder_grouping", argument: "grouping", label: "grouping" },
];

function setStatus(message, error = false) {
  const administrator = error && message.includes("requires administrator access");
  const notice = byId("administrator-notice");
  if (administrator) {
    byId("administrator-description").textContent = message;
    if (!notice.open) notice.showModal();
    return;
  }
  if (notice.open) notice.close();
  const status = byId("status");
  status.textContent = message;
  status.hidden = !message;
  status.classList.toggle("error", error);
}

function setBusy(value) {
  busy = value;
  byId("refresh").disabled = busy || !invoke;
  byId("apply-recommended").disabled = busy || !settings;
  byId("restore-original").disabled = busy || !settings;
  for (const view of folderViews) view.picker.setDisabled(busy || !settings);
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

function renderSettings() {
  for (const view of folderViews) view.picker.load(settings?.[view.field] ?? null, Boolean(settings));
  for (const preference of explorerPreferences) {
    setPreferenceValue(preference.id, !settings ? "Unavailable"
      : preference.windows11 && !settings.windows11_supported
      ? "Requires Windows 11"
      : settings[preference.field] === null ? "Custom or missing overlay icon"
      : settings[preference.field] ? (preference.enabledLabel || "Enabled") : (preference.disabledLabel || "Disabled"));
  }
}

async function loadSettings() {
  settings = null;
  renderSettings();
  settings = await invoke("explorer_settings");
  renderSettings();
}

async function loadSystem() {
  const info = await invoke("system_info");
  const windowsVersion = info.windows_version.trim().toUpperCase();
  const windows11 = /^Windows 11(?: |$)/i.test(info.os.trim());
  const testedOs = /^Windows 11 Pro$/i.test(info.os.trim());
  const testedVersion = windows11 && windowsVersion === "25H2";
  const release = /^(\d{2})H([12])$/.exec(windowsVersion);
  const olderWindows11 = windows11
    && release !== null
    && (Number(release[1]) < 25 || (Number(release[1]) === 25 && Number(release[2]) < 2));
  const newerWindows11 = windows11 && release !== null && Number(release[1]) > 25;
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
    const yellowDash = (label === "OS" && windows11 && !testedOs)
      || (label === "Windows" && (olderWindows11 || newerWindows11));
    const redCross = label === "OS" && !windows11;
    if (greenCheck || yellowDash || redCross) {
      const check = document.createElement("span");
      check.className = greenCheck ? "tested-system-check" : yellowDash ? "older-system-dash" : "unsupported-system-cross";
      check.textContent = greenCheck ? "✓" : yellowDash ? "−" : "✕";
      check.title = label === "OS"
        ? testedOs
          ? "Matches the tested OS edition: Windows 11 Pro"
          : windows11
            ? "Different Windows 11 edition! Features are tested on Windows 11 Pro"
            : "Different Windows operating system! Features are tested on Windows 11 Pro"
        : greenCheck
          ? "Matches the tested Windows version: 25H2"
          : newerWindows11
            ? "Windows version is newer than recommended. Features are tested on version 25H2"
            : "Windows version is older than recommended. Features are tested on version 25H2";
      check.setAttribute("role", "img");
      check.setAttribute("aria-label", check.title);
      description.append(check);
    }
    return [term, description];
  }));
}

async function refresh() {
  if (busy) return;
  setBusy(true);
  setStatus("Reading your system…");
  const results = await Promise.allSettled([loadSystem(), loadSettings()]);
  const errors = results.filter((result) => result.status === "rejected");
  if (results[0].status === "rejected") {
    byId("system").textContent = "System information unavailable.";
  }
  setStatus(errors.map((result) => String(result.reason)).join(" · "), errors.length > 0);
  setBusy(false);
}

function confirmChange(description, title = "Change preference?") {
  const dialog = byId("confirmation");
  byId("confirm-title").textContent = title;
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

async function applyChange(command, args, description, skipConfirmation = false, success = "Preference saved. Explorer restarted.") {
  if (busy || !settings) return;
  const recommended = command === "apply_recommended_settings";
  const preference = explorerPreferences.find(preference => preference.field === args.preference);
  if (preference?.requiresAdmin && !settings.is_administrator) {
    setStatus("This preference requires administrator access. Close Orbital and run it as administrator.", true);
    return;
  }
  setBusy(true);
  try {
    if (recommended) {
      setStatus("");
      const count = await invoke(command, { preview: true });
      if (count === 0) {
        showApplyResult(0);
        return;
      }
      description = `Apply ${count} recommended ${count === 1 ? "change" : "changes"}?`;
    }
    if (!skipPrompts && !skipConfirmation && !await confirmChange(description, recommended ? "Apply recommended settings?" : undefined)) return;
    setStatus("Saving preferences and restarting Explorer…");
    let error = null;
    let changed;
    try {
      changed = await invoke(command, args);
    } catch (reason) {
      error = String(reason);
    }
    try {
      await loadSettings();
    } catch (reason) {
      error = [error, `Could not refresh preferences: ${reason}`].filter(Boolean).join(" · ");
    }
    setStatus(error || (changed === false || changed === 0 ? "" : success), Boolean(error));
    if (recommended) showApplyResult(changed, error);
  } catch (reason) {
    setStatus(String(reason), true);
  } finally {
    setBusy(false);
  }
}

byId("refresh").addEventListener("click", refresh);
byId("apply-recommended").addEventListener("click", (event) => applyChange(
  "apply_recommended_settings", {}, "", event.shiftKey, "Recommended settings enabled. Explorer restarted.",
));
byId("restore-original").addEventListener("click", () => applyChange(
  "restore_original_settings", {}, "", true, "Original settings restored. Explorer restarted.",
));
for (const view of folderViews) {
  byId(`${view.id}-apply`).addEventListener("click", (event) => {
    const selected = view.picker.selected;
    if (selected !== null) applyChange(
      view.command,
      { [view.argument]: selected },
      `Use ${view.picker.label.toLowerCase()} for folder ${view.label} across all folder types? Saved folder views will be reset.`,
      event.shiftKey,
      `Default folder ${view.label} saved. Explorer restarted.`,
    );
  });
}
for (const preference of explorerPreferences) {
  byId(`${preference.id}-toggle`).addEventListener("click", (event) => applyChange(
    "toggle_preference",
    { preference: preference.field },
    settings?.[preference.field] ? preference.restore : preference.apply,
    event.shiftKey,
  ));
}

if (invoke) {
  refresh();
} else {
  renderSettings();
  byId("system").textContent = "Open Orbital with npm run dev to read system information.";
  setStatus("The Rust backend is available inside the desktop app.", true);
}
