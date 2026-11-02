"use strict";

let settings = null;
let busy = false;

function createSettingToggle(id, key, defaultValue, onChange) {
  const setting = { enabled: defaultValue };
  const toggle = byId(`${id}-toggle`);
  try {
    const stored = localStorage.getItem(`orbital.${key}`);
    setting.enabled = defaultValue ? stored !== "false" : stored === "true";
  } catch {
  }
  function render() {
    toggle.setAttribute("aria-checked", String(setting.enabled));
    setPreferenceValue(`${id}-value`, setting.enabled ? "Enabled" : "Disabled");
  }
  toggle.addEventListener("click", () => {
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
const debugMode = createSettingToggle("debug-mode", "debugMode", false, () => document.dispatchEvent(new Event("orbital-debug-mode-change")));

const explorerPreferences = [
  { id: "transfer-details", field: "expand_transfer_details", apply: "Expand details by default during file transfers?", restore: "Use the compact file-transfer dialog by default?" },
  { id: "network", field: "show_network", apply: "Show Network in the navigation pane?", restore: "Hide Network from the navigation pane?" },
  { id: "taskbar-flashing", field: "show_taskbar_flashing", apply: "Allow taskbar app icons to flash when they need your attention?", restore: "Stop taskbar app icons from flashing for attention?", supported: "windows11_supported" },
  { id: "restore-folder-windows", field: "restore_folder_windows", apply: "Restore previous folder windows when you sign in?", restore: "Stop restoring folder windows at sign-in?" },
  { id: "taskbar-centering", field: "center_taskbar_icons", apply: "Center taskbar icons?", restore: "Align taskbar icons to the left?", supported: "windows11_supported" },
  { id: "task-view", field: "show_task_view", apply: "Show the Task View button?", restore: "Hide the Task View button? Win+Tab will still work." },
  { id: "widgets", field: "allow_widgets", apply: "Allow Widgets, including the widgets board and taskbar content?", restore: "Disable Widgets?", supported: "windows11_supported", requiresAdmin: true },
  { id: "taskbar-end-task", field: "enable_taskbar_end_task", apply: "Add End task to app menus on the taskbar? It force-closes apps and can discard unsaved work.", restore: "Remove End task from app menus on the taskbar?", supported: "taskbar_end_task_supported" },
  { id: "explorer-spacing", personal: false, field: "use_compact_mode", apply: "Use compact spacing in File Explorer?", restore: "Restore extra spacing in File Explorer?", supported: "windows11_supported" },
  { id: "modern-context-menu", personal: false, field: "use_classic_context_menu", apply: "Use the classic Windows 10-style right-click menu?", restore: "Restore the Windows 11 right-click menu?", supported: "windows11_supported" },
  { id: "recent-files", field: "show_recent_files", apply: "Show recently used files in File Explorer Home?", restore: "Hide recently used files in File Explorer Home?" },
  { id: "recommendations", field: "show_recommendations", apply: "Show the Recommended section in File Explorer Home?", restore: "Hide the Recommended section in File Explorer Home?", supported: "windows11_supported" },
  { id: "frequent-folders", field: "show_frequent_folders", apply: "Show frequently used folders in Quick access?", restore: "Hide frequently used folders in Quick access?" },
  { id: "office-files", field: "show_cloud_files", apply: "Allow cloud files and account-based recommendations in File Explorer?", restore: "Hide cloud files and account-based recommendations in File Explorer?", supported: "windows11_supported" },
  { id: "home-folder", personal: true, field: "show_home_folder", apply: "Show Home in the navigation pane?", restore: "Hide Home in the navigation pane?", supported: "home_supported", requiresAdmin: true },
  { id: "gallery", personal: true, field: "show_gallery", apply: "Show Gallery in the navigation pane?", restore: "Hide Gallery in the navigation pane?", supported: "gallery_supported", requiresAdmin: true },
  { id: "file-extensions", field: "hide_file_extensions", apply: "Hide known file extensions?", restore: "Show file extensions?" },
  { id: "hidden-files", personal: false, field: "show_hidden_files", apply: "Show hidden files, folders and drives?", restore: "Hide hidden files, folders and drives?" },
  { id: "item-checkboxes", field: "show_item_checkboxes", apply: "Show selection checkboxes?", restore: "Hide selection checkboxes?" },
  { id: "libraries", field: "show_libraries", apply: "Show Libraries in the navigation pane?", restore: "Hide Libraries in the navigation pane?" },
  { id: "full-path", field: "show_full_path", apply: "Show the full folder path in the title bar?", restore: "Hide the full folder path in the title bar?" },
  { id: "single-click", field: "double_click_open", apply: "Require a double-click to open files and folders?", restore: "Open files and folders with a single click?" },
  { id: "empty-drives", field: "hide_empty_drives", apply: "Hide drives with no media?", restore: "Show empty drives?" },
  { id: "confirm-delete", field: "confirm_file_delete", apply: "Confirm before moving items to the Recycle Bin?", restore: "Stop asking before moving items to the Recycle Bin?", supported: "confirm_file_delete_supported" },
  { id: "icons-only", field: "icons_only", apply: "Show icons instead of thumbnails?", restore: "Allow file thumbnails?" },
  { id: "info-tips", field: "show_info_tips", apply: "Show file and folder hover descriptions?", restore: "Hide file and folder hover descriptions?" },
  { id: "system-files", field: "show_system_files", apply: "Show protected operating system files? Show hidden files must also be enabled.", restore: "Hide protected operating system files?" },
  { id: "shortcut-arrow", field: "hide_shortcut_arrow", apply: "Hide the arrow overlay on desktop shortcuts?", restore: "Show the arrow overlay on desktop shortcuts?", requiresAdmin: true },
];

const explorerChoices = [
  { id: "folder-view", picker: folderView, field: "folder_view", command: "set_folder_view", argument: "view", label: "view", personal: true, recommendedValue: "details", recommendation: "Set default folder view to Details" },
  { id: "explorer-launch", picker: explorerLaunch, field: "open_explorer_to", command: "set_explorer_launch", argument: "target", label: "starting location", personal: true, recommendedValue: "downloads", recommendation: "Open Explorer to Downloads" },
  { id: "folder-sort", picker: folderSorting, field: "folder_sort", command: "set_folder_sort", argument: "sort", label: "sorting", personal: true, recommendedValue: { column: "type", descending: false }, recommendation: "Sort folders by Type · A to Z" },
  { id: "folder-grouping", picker: folderGrouping, field: "folder_grouping", command: "set_folder_grouping", argument: "grouping", label: "grouping", personal: true, recommendedValue: "none", recommendation: "Set folder grouping to None" },
];

function preferenceRecommendation(preference) {
  return settings?.recommended_preferences?.find(([field]) => field === preference.field)?.[1];
}

function preferenceRecommended(preference) {
  return preferenceRecommendation(preference) !== undefined;
}

function preferencePersonal(preference) {
  return preference.personal ?? !preferenceRecommended(preference);
}

function preferenceAvailable(preference) {
  return Boolean(settings) && (!preference.supported || settings[preference.supported]);
}

function preferenceRestriction(preference, unavailable = "Unavailable on this system") {
  if (!preferenceAvailable(preference)) return unavailable;
  return preference.requiresAdmin && !settings.is_administrator ? "Requires administrator access" : "";
}

function preferenceValue(enabled) {
  return enabled === null ? "Custom or missing overlay icon" : enabled ? "Enabled" : "Disabled";
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
  byId("desktop-layout-safe-mode-toggle").disabled = busy || !settings;
  for (const id of ["apply-recommended", "restore-original", "batch-changes", "import-tweaks", "export-tweaks"]) {
    byId(id).disabled = busy || !settings;
  }
  for (const view of explorerChoices) view.picker.setDisabled(busy || !settings);
  for (const preference of explorerPreferences) {
    byId(`${preference.id}-toggle`).disabled = busy || !preferenceAvailable(preference);
  }
}

function setPreferenceValue(id, value, visual = false) {
  const element = byId(id);
  element.textContent = value;
  element.dataset.state = visual ? "visual" : value === "Enabled" ? "enabled" : value === "Disabled" ? "disabled" : "other";
}

function bohrPixelCircle(radius) {
  const step = 0.5;
  const pixels = Math.max(1, Math.round(radius / step));
  const rows = [];
  for (let y = -pixels; y < pixels; y++) {
    const halfWidth = Math.floor(Math.sqrt(pixels ** 2 - (y + 0.5) ** 2) + 0.5);
    rows.push(`M${-halfWidth * step} ${y * step}h${halfWidth * step * 2}v${step}h${-halfWidth * step * 2}Z`);
  }
  return rows.join("");
}

function renderBohr() {
  if (!settings) return;
  const available = [
    ...explorerPreferences.filter(preference => preferenceRecommended(preference) && preferenceAvailable(preference))
      .map(preference => ({ ...preference, label: `${preference.label} · ${preferenceValue(preferenceRecommendation(preference))}`, enabled: settings[preference.field] === preferenceRecommendation(preference) })),
    ...explorerChoices.filter(choice => choice.recommendedValue !== undefined && choice.picker.choices.some(option => choice.picker.matches(option.value, choice.recommendedValue)))
      .map(choice => ({ ...choice, label: choice.recommendation, enabled: choice.picker.matches(choice.recommendedValue, settings[choice.field]) })),
  ];
  const changes = available.filter(preference => preference.enabled);
  const count = byId("preferences-made");
  if (count) {
    count.textContent = `${changes.length} / ${available.length}`;
    count.title = "Recommended preferences applied";
  }
  const diagram = byId("bohr");
  diagram.setAttribute("aria-label", `${changes.length} of ${available.length} recommended preferences applied`);
  const signature = changes.map(preference => preference.field).join("|");
  if (diagram.dataset.changes === signature) return;
  diagram.dataset.changes = signature;
  const shellCount = 1 + Math.ceil(Math.max(0, changes.length - 2) / 8);
  const center = 30;
  const orbitScale = 29 / (13 + (shellCount - 1) * 8 + 2.5);
  const electronRadius = Math.max(1, Math.round(2.5 * orbitScale * 2) / 2);
  const electronOutline = bohrPixelCircle(electronRadius);
  const electronCore = bohrPixelCircle(Math.min(1.7 * orbitScale, electronRadius - 0.5));
  const circle = (radius, className, x = center, y = center) => {
    const element = document.createElementNS(diagram.namespaceURI, "circle");
    for (const [name, value] of Object.entries({ cx: x, cy: y, r: radius, class: className })) {
      element.setAttribute(name, value);
    }
    return element;
  };
  const shells = [];
  for (let index = 0, offset = 0; index === 0 || offset < changes.length; index++) {
    const electrons = changes.slice(offset, offset + (index === 0 ? 2 : 8));
    const radius = (13 + index * 8) * orbitScale;
    const shell = document.createElementNS(diagram.namespaceURI, "g");
    shell.classList.add("bohr-shell");
    shell.append(circle(radius, "bohr-ring"));
    electrons.forEach((preference, position) => {
      const angle = position * Math.PI * 2 / electrons.length - Math.PI / 2;
      const placement = document.createElementNS(diagram.namespaceURI, "g");
      placement.setAttribute("transform", `translate(${center + Math.cos(angle) * radius} ${center + Math.sin(angle) * radius})`);
      const electron = document.createElementNS(diagram.namespaceURI, "g");
      electron.classList.add("bohr-electron-pixel");
      for (const [className, outline] of [
        ["bohr-electron-outline", electronOutline],
        ["bohr-electron-core", electronCore],
      ]) {
        const path = document.createElementNS(diagram.namespaceURI, "path");
        path.setAttribute("class", className);
        path.setAttribute("d", outline);
        electron.append(path);
      }
      const title = document.createElementNS(diagram.namespaceURI, "title");
      title.textContent = preference.label;
      electron.append(title);
      placement.append(electron);
      shell.append(placement);
    });
    shells.push(shell);
    offset += electrons.length;
  }
  const nucleus = document.createElementNS(diagram.namespaceURI, "path");
  nucleus.setAttribute("class", "bohr-nucleus");
  nucleus.setAttribute("d", bohrPixelCircle(5.5 * orbitScale));
  nucleus.setAttribute("transform", `translate(${center} ${center})`);
  diagram.replaceChildren(...shells, nucleus);
}

function renderSettings() {
  const safeMode = Boolean(settings?.desktop_layout_safe_mode);
  byId("desktop-layout-safe-mode-toggle").setAttribute("aria-checked", String(safeMode));
  byId("desktop-layout-safe-mode-toggle").disabled = busy || !settings;
  setPreferenceValue("desktop-layout-safe-mode-value", settings ? safeMode ? "Enabled" : "Disabled" : "Loading…");
  document.querySelectorAll(".admin-warning").forEach(adminWarning);
  document.querySelectorAll(".admin-requirement").forEach(note => {
    note.dataset.state = settings?.is_administrator === true ? "granted" : settings?.is_administrator === false ? "missing" : "unknown";
  });
  for (const view of explorerChoices) {
    view.picker.load(settings?.[view.field] ?? null, Boolean(settings));
    byId(`${view.id}-value`).dataset.state = view.personal || view.recommendedValue === undefined ? "visual" : !settings ? "other"
      : view.picker.matches(view.recommendedValue, settings[view.field]) ? "enabled" : "disabled";
  }
  for (const preference of explorerPreferences) {
    setPreferenceValue(preference.id, preferenceAvailable(preference)
      ? preferenceValue(settings[preference.field]) : "Unavailable",
      preferencePersonal(preference));
    if (preferenceAvailable(preference) && preferenceRecommended(preference) && !preferencePersonal(preference) && settings[preference.field] !== null) {
      byId(preference.id).dataset.state = settings[preference.field] === preferenceRecommendation(preference) ? "enabled" : "disabled";
    }
  }
  renderBohr();
}

async function loadSettings() {
  settings = null;
  renderSettings();
  settings = await invoke("explorer_settings");
  renderSettings();
}

byId("desktop-layout-safe-mode-toggle").addEventListener("click", async () => {
  if (busy || !settings) return;
  const enabled = !settings.desktop_layout_safe_mode;
  setBusy(true);
  try {
    await invoke("set_desktop_layout_safe_mode", { enabled });
    settings.desktop_layout_safe_mode = enabled;
    renderSettings();
    setStatus(`Desktop icon layout safe mode ${enabled ? "enabled" : "disabled"}.`);
  } catch (error) {
    setStatus(String(error), true);
  } finally {
    setBusy(false);
  }
});

async function loadAppVersion() {
  try {
    const version = await window.__TAURI__.app.getVersion();
    const label = byId("app-version");
    label.textContent = `Orbital version: v${version}`;
    label.hidden = false;
  } catch (error) {
    console.error("Could not read the app version:", error);
  }
}

async function loadSystem() {
  const info = await invoke("system_info");
  const windowsVersion = info.windows_version.trim().toUpperCase();
  const windows11 = /^Windows 11(?: |$)/i.test(info.os.trim());
  const supportedOs = /^Windows 11 Pro$/i.test(info.os.trim());
  const supportedVersions = ["25H2", "26H2"];
  const supportedVersion = windows11 && supportedVersions.includes(windowsVersion);
  const release = /^\d{2}H[12]$/.test(windowsVersion);
  const rows = [
    ["Host", info.host], ["OS", info.os], ["Windows", info.windows_version],
    ["CPU", info.cpu], ["Threads", info.threads],
    ["Memory", `${info.used_memory_gib.toFixed(1)} / ${info.total_memory_gib.toFixed(1)} GiB`],
    ["Recommended:", "…"],
  ];
  byId("system").replaceChildren(...rows.flatMap(([label, value]) => {
    const term = element("dt", "", label);
    const description = element("dd", "", value);
    if (label === "OS") {
      description.className = "system-os";
      const name = element("span", "system-os-name", value);
      name.title = value;
      description.replaceChildren(name);
    }
    if (label === "Recommended:") {
      description.className = "changes-count";
      description.replaceChildren(byId("changes-count").content.cloneNode(true));
    }
    const greenCheck = (label === "OS" && supportedOs) || (label === "Windows" && supportedVersion);
    const yellowDash = (label === "OS" && windows11 && !supportedOs)
      || (label === "Windows" && windows11 && release && !supportedVersion);
    const redCross = label === "OS" && !windows11;
    if (greenCheck || yellowDash || redCross) {
      const check = document.createElement("span");
      check.className = greenCheck ? "tested-system-check" : yellowDash ? "older-system-dash" : "unsupported-system-cross";
      check.textContent = greenCheck ? "✓" : yellowDash ? "−" : "✕";
      check.title = label === "OS"
        ? supportedOs
          ? "Supported OS edition: Windows 11 Pro"
          : windows11
            ? "Different Windows 11 edition. Orbital supports Windows 11 Pro."
            : "Unsupported Windows operating system. Orbital supports Windows 11 Pro."
        : greenCheck
          ? `Supported Windows version: ${windowsVersion}`
          : `Unsupported Windows version. Orbital supports ${supportedVersions.join(" and ")}.`;
      check.setAttribute("role", "img");
      check.setAttribute("aria-label", check.title);
      description.append(check);
    }
    return [term, description];
  }));
  renderBohr();
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
  const icon = (pathData, detail) => {
    const svg = document.createElementNS("http://www.w3.org/2000/svg", "svg");
    for (const [name, value] of Object.entries({ viewBox: "0 0 24 24", width: 16, height: 16, fill: "none", stroke: "currentColor", "stroke-width": 1.75, "stroke-linecap": "round", "stroke-linejoin": "round", "aria-hidden": "true", focusable: "false" })) {
      svg.setAttribute(name, value);
    }
    const path = document.createElementNS(svg.namespaceURI, "path");
    path.setAttribute("d", pathData);
    svg.append(path);
    if (detail) {
      const mark = document.createElementNS(svg.namespaceURI, "path");
      mark.setAttribute("d", detail);
      mark.setAttribute("fill", "currentColor");
      mark.setAttribute("stroke", "none");
      svg.append(mark);
    }
    return svg;
  };
  const icons = [icon("M12 3 22 21H2Z", "M10.5 8h3v6h-3Z M10.5 16.5h3v3h-3Z")];
  if (administrator != null) {
    icons.push(icon(administrator ? "M3 12l6 9L21 3" : "M3 3l18 18 M21 3 3 21"));
  }
  warning.replaceChildren(...icons);
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

function confirmChange(description, title = "Change preference?", recommendations = [], note = "Windows Explorer will restart to apply this change.") {
  const dialog = byId("confirmation");
  dialog.classList.toggle("changes-dialog", recommendations.length > 0);
  byId("confirm-title").textContent = title;
  byId("confirm-description").textContent = description;
  byId("confirm-note").textContent = note;
  byId("recommended-options").hidden = !recommendations.length;
  byId("recommended-options").open = true;
  byId("recommendation-list").replaceChildren(...recommendations.map((id) => {
    const preference = explorerPreferences.find(preference => preference.field === id);
    const view = explorerChoices.find(view => view.field === id);
    const { row, text } = createChangeRow(id, preference ? `${preference.label} · ${preferenceValue(preferenceRecommendation(preference))}` : view.recommendation,
      (preference || view).description, preference ? preferencePersonal(preference) : Boolean(view.personal));
    if (preference?.requiresAdmin) text.append(adminWarning());
    return row;
  }));
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
  showResult(completed ? "Recommended preferences applied" : "Could not complete recommendations", completed
    ? count === 0 ? "Your settings already match all recommended values."
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
      if (!await showResult("Shortcut icon already exists", `${reason.blank_icon_conflict} already exists and is not Orbital's transparent icon. Replace it to hide shortcut arrows?`, "", true)) return { cancelled: true };
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
  const reverting = command === "restore_original_settings";
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
    if (!skip && !await confirmChange(description, recommended ? "Recommended preferences" : reverting ? "Revert preferences?" : undefined, recommendations,
      reverting ? "If folder views were changed, this also restores their saved layout snapshot and replaces folder layout changes made since then. Desktop icon layouts are preserved. Windows Explorer will restart to apply this change."
        : recommendations.some(field => field === "folder_view" || field === "folder_sort" || field === "folder_grouping")
          ? "Changing the default folder view, sorting or grouping resets saved folder views. Desktop icon layouts are preserved. Windows Explorer will restart to apply these changes." : undefined)) return;
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
  "restore_original_settings", {}, "Revert your original settings?", false, "Original settings restored. Explorer restarted.",
));
for (const view of explorerChoices) {
  view.row = byId(`${view.id}-label`).closest(".preference");
  view.title = byId(`${view.id}-label`).textContent.trim();
  view.description = view.row.querySelector(".preference-description").textContent.trim();
  byId(`${view.id}-apply`).addEventListener("click", (event) => {
    const selected = view.picker.selected;
    if (selected !== null) applyChange(
      view.command,
      { [view.argument]: selected },
      view.field === "open_explorer_to" ? `Open new File Explorer windows to ${view.picker.label}?`
        : `Use ${view.picker.label.toLowerCase()} for folder ${view.label} across all folder types? Saved folder views will be reset. Desktop icon layouts are preserved.`,
      event.shiftKey,
      view.field === "open_explorer_to" ? "Explorer starting location saved. Explorer restarted."
        : `${view.title} saved. Explorer restarted.`,
    );
  });
}
for (const preference of explorerPreferences) {
  const row = document.querySelector(`[data-preference="${preference.id}"]`);
  const title = row.querySelector("h3");
  preference.row = row;
  preference.description = row.querySelector(".preference-description").textContent.trim();
  preference.label = title.textContent.trim();
  if (preference.requiresAdmin) title.append(adminWarning());
  const value = document.createElement("p");
  value.id = preference.id;
  value.className = "preference-value";
  title.parentElement.append(value);
  const toggle = document.createElement("button");
  toggle.id = `${preference.id}-toggle`;
  toggle.textContent = "Toggle";
  toggle.disabled = true;
  row.append(toggle);
  toggle.addEventListener("click", (event) => applyChange(
    "toggle_preference",
    { preference: preference.field },
    settings?.[preference.field] ? preference.restore : preference.apply,
    event.shiftKey,
  ));
}

if (invoke) {
  loadAppVersion();
  refresh();
} else {
  renderSettings();
  byId("system").textContent = "Open Orbital with npm run dev to read system information.";
  setStatus("The Rust backend is available inside the desktop app.", true);
}
