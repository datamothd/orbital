"use strict";

const invoke = window.__TAURI__?.core.invoke;
const byId = (id) => document.getElementById(id);
let settings = null;
let busy = false;

function createSettingToggle(id, key, defaultValue, onChange) {
  const setting = { enabled: defaultValue };
  try {
    const stored = localStorage.getItem(`orbital.${key}`);
    setting.enabled = defaultValue ? stored !== "false" : stored === "true";
  } catch {
  }
  function render() {
    byId(`${id}-toggle`).setAttribute("aria-checked", String(setting.enabled));
    setPreferenceValue(`${id}-value`, setting.enabled ? "Enabled" : "Disabled");
  }
  byId(`${id}-toggle`).addEventListener("click", () => {
    setting.enabled = !setting.enabled;
    onChange?.();
    render();
    try {
      localStorage.setItem(`orbital.${key}`, String(setting.enabled));
    } catch {
    }
  });
  render();
  return setting;
}

const skipPrompts = createSettingToggle("skip-prompts", "skipPrompts", false);

const explorerPreferences = [
  { id: "taskbar-centering", field: "disable_taskbar_centering", enabledLabel: "Left", disabledLabel: "Centered", apply: "Move taskbar icons to the left?", restore: "Center taskbar icons?", supported: "windows11_supported", visual: true },
  { id: "task-view", field: "hide_task_view", apply: "Hide the Task View button? Win+Tab will still work.", restore: "Show the Task View button?", visual: true },
  { id: "widgets", field: "disable_widgets", apply: "Disable Widgets, including the widgets board and taskbar content?", restore: "Allow Widgets again?", supported: "windows11_supported", requiresAdmin: true },
  { id: "taskbar-end-task", field: "enable_taskbar_end_task", apply: "Add End task to app menus on the taskbar? It force-closes apps and can discard unsaved work.", restore: "Remove End task from app menus on the taskbar?", supported: "taskbar_end_task_supported" },
  { id: "explorer-spacing", field: "disable_explorer_spacing", apply: "Use compact spacing in File Explorer?", restore: "Restore extra spacing in File Explorer?", supported: "windows11_supported" },
  { id: "modern-context-menu", field: "disable_modern_context_menu", apply: "Use the classic Windows 10-style right-click menu?", restore: "Restore the Windows 11 right-click menu?", supported: "windows11_supported" },
  { id: "recent-files", field: "hide_recent_files", apply: "Hide recently used files in File Explorer Home?", restore: "Show recently used files in File Explorer Home?" },
  { id: "recommendations", field: "hide_recommendations", apply: "Hide the Recommended section in File Explorer Home?", restore: "Show the Recommended section in File Explorer Home?", supported: "windows11_supported" },
  { id: "frequent-folders", field: "hide_frequent_folders", apply: "Hide frequently used folders in Quick Access?", restore: "Show frequently used folders in Quick Access?" },
  { id: "office-files", field: "hide_office_files", apply: "Hide cloud files and account-based recommendations in File Explorer?", restore: "Allow cloud files and account-based recommendations in File Explorer?", supported: "windows11_supported" },
  { id: "home-folder", field: "hide_home_folder", apply: "Hide Home in the navigation pane?", restore: "Show Home in the navigation pane?", supported: "home_supported", requiresAdmin: true, visual: true },
  { id: "gallery", field: "hide_gallery", apply: "Hide Gallery in the navigation pane?", restore: "Show Gallery in the navigation pane?", supported: "gallery_supported", requiresAdmin: true, visual: true },
  { id: "shortcut-arrow", field: "hide_shortcut_arrow", apply: "Hide the arrow overlay on desktop shortcuts?", restore: "Show the arrow overlay on desktop shortcuts?", requiresAdmin: true, visual: true },
];

const folderViews = [
  { id: "folder-sort", picker: folderSorting, field: "folder_sort", command: "set_folder_sort", argument: "sort", label: "sorting" },
  { id: "folder-grouping", picker: folderGrouping, field: "folder_grouping", command: "set_folder_grouping", argument: "grouping", label: "grouping" },
];

function preferenceAvailable(preference) {
  return Boolean(settings) && (!preference.supported || settings[preference.supported]);
}

function preferenceValue(preference, enabled) {
  if (enabled === null) return "Custom or missing overlay icon";
  return enabled ? (preference.enabledLabel || "Enabled") : (preference.disabledLabel || "Disabled");
}

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
    byId(`${preference.id}-toggle`).disabled = busy || !preferenceAvailable(preference);
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
    setPreferenceValue(preference.id, preferenceAvailable(preference)
      ? preferenceValue(preference, settings[preference.field]) : "Unavailable",
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
      || (label === "Windows" && windows11 && release !== null && !testedVersion);
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

function selectedValues(id) {
  return [...byId(id).querySelectorAll("input:checked:enabled")].map(input => input.value);
}

function updateRecommendedSelection() {
  const selected = selectedValues("recommendation-list");
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
      ? preference.label
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

function showResult(title, description, error = "", replaceIcon = false) {
  const errorNotice = byId("apply-result-error");
  byId("apply-result-title").textContent = title;
  byId("apply-result-description").textContent = description;
  errorNotice.hidden = !error;
  errorNotice.textContent = error;
  byId("replace-icon-apply").hidden = !replaceIcon;
  if (replaceIcon) return confirmDialog(byId("apply-result"));
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
    if (!args.replaceBlankIcon && typeof reason?.blank_icon_conflict === "string") {
      setStatus("");
      if (!await showResult("blank.ico already exists", `${reason.blank_icon_conflict} already exists and is not Orbital's transparent icon. Replace it to hide shortcut arrows?`, "", true)) return { cancelled: true };
      setStatus("Saving preferences and restarting Explorer…");
      return savePreferences(command, { ...args, replaceBlankIcon: true });
    }
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
  let skip = skipPrompts.enabled || skipConfirmation;
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
      args.selected = skip ? recommendations : selectedValues("recommendation-list");
      if (!args.selected.length || missingAdminCount(args.selected)) return;
    }
    setStatus("Saving preferences and restarting Explorer…");
    const result = await savePreferences(command, args);
    if (result.cancelled) return;
    ({ changed, error } = result);
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
  "restore_original_settings", {}, "Restore your original settings? If folder views were changed, this also restores their saved layout snapshot and replaces layout changes made since then.", false, "Original settings restored. Explorer restarted.",
));
for (const view of folderViews) {
  byId(`${view.id}-apply`).addEventListener("click", (event) => {
    const selected = view.picker.selected;
    if (selected !== null) applyChange(
      view.command,
      { [view.argument]: selected },
      `Use ${view.picker.label.toLowerCase()} for folder ${view.label} across all folder types? Saved views, including individual folder layouts, will be reset.`,
      event.shiftKey,
      `Default folder ${view.label} saved. Explorer restarted.`,
    );
  });
}
for (const preference of explorerPreferences) {
  const title = byId(`${preference.id}-toggle`).closest(".preference").querySelector("h3");
  preference.label = title.textContent.trim();
  if (preference.requiresAdmin) title.append(adminWarning());
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
