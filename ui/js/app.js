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

function renderToggle(id, enabled) {
  byId(`${id}-toggle`).setAttribute("aria-checked", String(enabled));
  setPreferenceValue(`${id}-value`, enabled ? "Enabled" : "Disabled");
}

byId("skip-prompts-toggle").addEventListener("click", () => {
  skipPrompts = !skipPrompts;
  renderToggle("skip-prompts", skipPrompts);
  try {
    localStorage.setItem("orbital.skipPrompts", String(skipPrompts));
  } catch {
  }
});
renderToggle("skip-prompts", skipPrompts);

const explorerPreferences = [
  { id: "taskbar-centering", field: "disable_taskbar_centering", enabledLabel: "Left", disabledLabel: "Centered", apply: "Move taskbar icons to the left?", restore: "Center taskbar icons?", visual: true },
  { id: "explorer-spacing", field: "disable_explorer_spacing", apply: "Use compact spacing in File Explorer?", restore: "Restore extra spacing in File Explorer?" },
  { id: "modern-context-menu", field: "disable_modern_context_menu", apply: "Use the classic Windows 10-style right-click menu?", restore: "Restore the Windows 11 right-click menu?", windows11: true },
  { id: "recent-files", field: "hide_recent_files", apply: "Hide recent files and recommendations in Quick Access?", restore: "Show recent files and recommendations in Quick Access?" },
  { id: "frequent-folders", field: "hide_frequent_folders", apply: "Hide frequently used folders in Quick Access?", restore: "Show frequently used folders in Quick Access?" },
  { id: "office-files", field: "hide_office_files", apply: "Hide Office.com cloud files in Quick Access?", restore: "Show Office.com cloud files in Quick Access?", windows11: true },
  { id: "home-folder", field: "hide_home_folder", apply: "Hide Home in the navigation pane?", restore: "Show Home in the navigation pane?", windows11: true, requiresAdmin: true, visual: true },
  { id: "gallery", field: "hide_gallery", apply: "Hide Gallery in the navigation pane?", restore: "Show Gallery in the navigation pane?", windows11: true, requiresAdmin: true, visual: true },
  { id: "shortcut-arrow", field: "hide_shortcut_arrow", apply: "Hide the arrow overlay on desktop shortcuts?", restore: "Show the arrow overlay on desktop shortcuts?", requiresAdmin: true, visual: true },
];

const folderViews = [
  { id: "folder-sort", picker: folderSorting, field: "folder_sort", command: "set_folder_sort", argument: "sort", label: "sorting" },
  { id: "folder-grouping", picker: folderGrouping, field: "folder_grouping", command: "set_folder_grouping", argument: "grouping", label: "grouping" },
];

function setStatus(message, error = false) {
  const notice = byId("administrator-notice");
  if (error && message.includes("requires administrator access")) {
    byId("administrator-description").textContent = message;
    if (!notice.open) notice.showModal();
    return;
  }
  if (notice.open) notice.close();
  const status = byId("status");
  status.textContent = message || "Ready.";
  status.classList.toggle("error", error);
}

function setBusy(value) {
  busy = value;
  byId("refresh").disabled = busy || !invoke;
  for (const id of ["apply-recommended", "restore-original", "import-tweaks", "export-tweaks"]) {
    byId(id).disabled = busy || !settings;
  }
  for (const view of folderViews) view.picker.setDisabled(busy || !settings);
  for (const preference of explorerPreferences) {
    byId(`${preference.id}-toggle`).disabled = busy || !settings
      || (preference.windows11 && !settings.windows11_supported);
  }
}

function setPreferenceValue(id, value, visual = false) {
  const element = byId(id);
  element.textContent = value;
  element.dataset.state = visual ? "visual" : value === "Enabled" ? "enabled" : value === "Disabled" ? "disabled" : "other";
}

function renderSettings() {
  document.querySelectorAll(".admin-warning").forEach(adminWarning);
  for (const view of folderViews) view.picker.load(settings?.[view.field] ?? null, Boolean(settings));
  for (const preference of explorerPreferences) {
    setPreferenceValue(preference.id, !settings ? "Unavailable"
      : preference.windows11 && !settings.windows11_supported
      ? "Requires Windows 11"
      : settings[preference.field] === null ? "Custom or missing overlay icon"
      : settings[preference.field] ? (preference.enabledLabel || "Enabled") : (preference.disabledLabel || "Disabled"),
      preference.visual);
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

function adminWarning(warning = document.createElement("span")) {
  const administrator = settings?.is_administrator;
  warning.className = "admin-warning";
  warning.dataset.state = administrator === true ? "granted" : administrator === false ? "missing" : "unknown";
  warning.textContent = `⚠${administrator === true ? " ✓" : administrator === false ? " ✕" : ""}`;
  warning.title = `Requires administrator access${administrator === true ? " · Access found" : administrator === false ? " · Access not found" : ""}`;
  warning.setAttribute("role", "img");
  warning.setAttribute("aria-label", warning.title);
  return warning;
}

function missingAdminCount(ids) {
  if (settings?.is_administrator) return 0;
  return ids.filter(id => explorerPreferences.some(preference => preference.field === id && preference.requiresAdmin)).length;
}

function updateRecommendedSelection() {
  const selected = [...byId("recommendation-list").querySelectorAll("input:checked")].map(input => input.value);
  const count = missingAdminCount(selected);
  const note = byId("confirm-admin-note");
  note.hidden = count === 0;
  note.textContent = count ? `Admin access is required for ${count} ${count === 1 ? "change" : "changes"}.` : "";
  byId("confirm-apply").disabled = !selected.length || count > 0;
}

function confirmDialog(dialog) {
  dialog.returnValue = "cancel";
  return new Promise((resolve) => {
    dialog.addEventListener("close", () => resolve(dialog.returnValue === "apply"), { once: true });
    dialog.showModal();
  });
}

function confirmChange(description, title = "Change preference?", recommendations = []) {
  const dialog = byId("confirmation");
  dialog.classList.toggle("changes-dialog", recommendations.length > 0);
  byId("confirm-title").textContent = title;
  byId("confirm-description").textContent = description;
  byId("recommended-options").hidden = !recommendations.length;
  byId("recommended-options").open = true;
  const list = byId("recommendation-list");
  list.replaceChildren(...recommendations.map((id) => {
    const label = document.createElement("label");
    const checkbox = document.createElement("input");
    checkbox.type = "checkbox";
    checkbox.value = id;
    checkbox.checked = true;
    const text = document.createElement("span");
    const preference = explorerPreferences.find(preference => preference.field === id);
    text.textContent = preference
      ? byId(`${preference.id}-toggle`).closest(".preference").querySelector("h3").firstChild.textContent.trim()
      : id === "folder_sort" ? "Sort folders by Type · A to Z" : "Set folder grouping to None";
    if (preference?.requiresAdmin) text.append(adminWarning());
    if (preference?.visual || folderViews.some(view => view.field === id)) {
      label.className = "visual-preference";
    }
    label.append(checkbox, text);
    return label;
  }).sort((a, b) => Number(b.classList.contains("visual-preference")) - Number(a.classList.contains("visual-preference"))));
  if (recommendations.length) {
    updateRecommendedSelection();
  } else {
    byId("confirm-admin-note").hidden = true;
    byId("confirm-admin-note").textContent = "";
    byId("confirm-apply").disabled = false;
  }
  return confirmDialog(dialog);
}

function showResult(title, description, error = "") {
  const errorNotice = byId("apply-result-error");
  byId("apply-result-title").textContent = title;
  byId("apply-result-description").textContent = description;
  errorNotice.hidden = !error;
  errorNotice.textContent = error;
  byId("apply-result").showModal();
}

function showApplyResult(count, error) {
  const completed = typeof count === "number";
  showResult(completed ? "Recommended settings applied" : "Could not complete recommendations", completed
    ? count === 0 ? "0 changes are made. All recommended settings are already enabled."
      : `${count} ${count === 1 ? "change" : "changes"} made. Explorer restarted.`
    : "The number of changes could not be confirmed.", error || "");
}

async function savePreferences(command, args) {
  let changed, error;
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
  return { changed, error };
}

async function applyChange(command, args, description, skipConfirmation = false, success = "Preference saved. Explorer restarted.") {
  if (busy || !settings) return;
  const recommended = command === "apply_recommended_settings";
  let skip = skipPrompts || skipConfirmation;
  const preference = explorerPreferences.find(preference => preference.field === args.preference);
  if (preference?.requiresAdmin && !settings.is_administrator) {
    setStatus("This preference requires administrator access. Close Orbital and run it as administrator.", true);
    return;
  }
  setBusy(true);
  let changed;
  let error = null;
  try {
    let recommendations = [];
    if (recommended) {
      setStatus("");
      recommendations = await invoke(command, { preview: true });
      if (!recommendations.length) {
        showApplyResult(0);
        return;
      }
      description = "Uncheck any settings you want to keep.";
      if (missingAdminCount(recommendations)) skip = false;
    }
    if (!skip && !await confirmChange(description, recommended ? "Recommended settings" : undefined, recommendations)) return;
    if (recommended) {
      args.selected = skip ? recommendations
        : [...byId("recommendation-list").querySelectorAll("input:checked")].map(input => input.value);
      if (!args.selected.length || missingAdminCount(args.selected)) return;
    }
    setStatus("Saving preferences and restarting Explorer…");
    ({ changed, error } = await savePreferences(command, args));
    if (recommended && changed) changed = changed.length;
  } catch (reason) {
    error = String(reason);
  } finally {
    setBusy(false);
  }
  if (recommended) {
    setStatus("");
    showApplyResult(changed, error);
  } else {
    setStatus(error || (changed === false || changed === 0 ? "" : success), Boolean(error));
  }
}

byId("recommendation-list").addEventListener("change", updateRecommendedSelection);
byId("refresh").addEventListener("click", refresh);
byId("apply-recommended").addEventListener("click", (event) => applyChange(
  "apply_recommended_settings", {}, "", event.shiftKey,
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
  if (preference.requiresAdmin) {
    byId(`${preference.id}-toggle`).closest(".preference").querySelector("h3").append(adminWarning());
  }
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
