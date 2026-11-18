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
  { id: "transfer-details", field: "expand_transfer_details" },
  { id: "network", field: "show_network" },
  { id: "taskbar-flashing", field: "show_taskbar_flashing", supported: "windows11_supported" },
  { id: "restore-folder-windows", field: "restore_folder_windows" },
  { id: "taskbar-centering", field: "center_taskbar_icons", supported: "windows11_supported" },
  { id: "task-view", field: "show_task_view" },
  { id: "widgets", field: "allow_widgets", supported: "windows11_supported", requiresAdmin: true },
  { id: "taskbar-end-task", field: "enable_taskbar_end_task", supported: "taskbar_end_task_supported" },
  { id: "explorer-spacing", personal: false, field: "use_compact_mode", supported: "windows11_supported" },
  { id: "modern-context-menu", personal: false, field: "use_classic_context_menu", supported: "windows11_supported" },
  { id: "recent-files", field: "show_recent_files" },
  { id: "recommendations", field: "show_recommendations", supported: "windows11_supported" },
  { id: "frequent-folders", field: "show_frequent_folders" },
  { id: "office-files", field: "show_cloud_files", supported: "windows11_supported" },
  { id: "home-folder", personal: true, field: "show_home_folder", supported: "home_supported", requiresAdmin: true },
  { id: "gallery", personal: true, field: "show_gallery", supported: "gallery_supported", requiresAdmin: true },
  { id: "file-extensions", field: "hide_file_extensions" },
  { id: "hidden-files", personal: false, field: "show_hidden_files" },
  { id: "item-checkboxes", field: "show_item_checkboxes" },
  { id: "libraries", field: "show_libraries" },
  { id: "full-path", field: "show_full_path" },
  { id: "single-click", field: "double_click_open" },
  { id: "empty-drives", field: "hide_empty_drives" },
  { id: "confirm-delete", field: "confirm_file_delete", supported: "confirm_file_delete_supported" },
  { id: "icons-only", field: "icons_only" },
  { id: "info-tips", field: "show_info_tips" },
  { id: "system-files", field: "show_system_files" },
  { id: "shortcut-arrow", field: "hide_shortcut_arrow", requiresAdmin: true },
];

const explorerChoices = [
  { id: "folder-view", picker: folderView, field: "folder_view", personal: true, recommendedValue: "details", recommendation: "Set default folder view to Details" },
  { id: "explorer-launch", picker: explorerLaunch, field: "open_explorer_to", personal: true, recommendedValue: "downloads", recommendation: "Open Explorer to Downloads" },
  { id: "folder-sort", picker: folderSorting, field: "folder_sort", personal: true, recommendedValue: { column: "type", descending: false }, recommendation: "Sort folders by Type · A to Z" },
  { id: "folder-grouping", picker: folderGrouping, field: "folder_grouping", personal: true, recommendedValue: "none", recommendation: "Set folder grouping to None" },
];

const combineButtons = ["Always, hide labels", "When taskbar is full", "Never"];
for (const [group, preferences] of Object.entries({
  "explorer-behavior": [
    ["browse_folders_in_new_window", "Open folders in", "Choose whether opening a folder uses the current window or creates one.", ["Same window", "New window"]],
    ["separate_folder_process", "Launch folder windows in a separate process", "Keeps folder windows separate from the desktop and taskbar process."],
    ["sharing_wizard", "Use the Sharing Wizard", "Uses the guided file and folder sharing interface."],
    ["hide_folder_merge_conflicts", "Hide folder merge conflicts", "Skips the folder merge confirmation when destination folders share the same name."],
    ["explorer_typing", "When typing in a folder", "Choose how File Explorer handles typing while the file list has focus.", ["Select the typed item", "Type into the search box"]],
  ],
  "explorer-visibility": [
    ["drive_letters", "Drive letters", "Choose where drive letters appear relative to drive names.", ["After drive names", "Before network drive names", "Hidden", null, "Before all drive names"]],
  ],
  "explorer-appearance": [
    ["folder_size_tips", "Show folder size in hover descriptions", "Includes folder size when hover descriptions are enabled."],
    ["highlight_compressed_files", "Color encrypted and compressed files", "Uses distinct colors for NTFS-encrypted and NTFS-compressed items."],
    ["file_type_overlay", "Show file icons on thumbnails", "Overlays a file type icon on thumbnails."],
    ["preview_handlers", "Show previews in the preview pane", "Lets File Explorer display previews for supported file types."],
    ["explorer_status_bar", "Show status bar", "Shows item counts and selection details at the bottom of File Explorer."],
  ],
  "explorer-navigation": [
    ["expand_navigation_to_folder", "Expand to the open folder", "Expands the navigation tree to the folder you are viewing."],
    ["all_navigation_folders", "Show all folders in the navigation pane", "Shows the full desktop folder tree in the navigation pane."],
    ["navigation_cloud_status", "Show cloud availability status", "Shows availability indicators for synced folders in the navigation pane."],
  ],
  "explorer-privacy": [
    ["sync_provider_notifications", "Show sync provider notifications", "Shows storage and sync provider messages in File Explorer."],
  ],
  "start-layout": [
    ["start_pinned_section", "Show Pinned section", "Shows pinned apps in Start."],
    ["start_recent_section", "Show Recent section", "Shows the Recent section when it has items to display."],
    ["start_all_apps_section", "Show All apps section", "Shows the installed apps section in Start."],
    ["start_hide_profile", "Hide name and profile picture", "Hides your name and profile picture in the Start menu."],
  ],
  "start-recent": [
    ["start_recently_added_apps", "Show recently added apps", "Shows newly installed apps in Start."],
    ["start_most_used_apps", "Show most used apps", "Shows frequently used apps when app launch tracking is enabled."],
    ["start_recent_items", "Show recently opened items", "Controls recent items in Start, Jump Lists and File Explorer."],
    ["start_tips_and_suggestions", "Show tips and app suggestions", "Shows suggested apps, shortcuts and tips in Start."],
    ["start_account_notifications", "Show account-related notifications", "Shows account reminders on the Start profile menu."],
    ["track_app_launches", "Track app launches", "Lets Windows personalize Start and Search based on the apps you open."],
  ],
  "start-search": [
    ["search_highlights", "Show search highlights", "Shows featured moments and other highlights in Windows Search."],
    ["search_web_suggestions", "Show web suggestions", "Includes suggestions from the web in Windows Search."],
    ["search_store_suggestions", "Show Microsoft Store suggestions", "Includes app suggestions from Microsoft Store in Windows Search."],
  ],
  "taskbar-items": [
    ["taskbar_search", "Search on the taskbar", "Choose how Windows Search appears. Win+S always opens Search.", ["Hidden", "Search icon", "Search box", "Search icon and label"]],
    ["taskbar_widgets_button", "Show Widgets button", "Shows the taskbar entry point when Widgets are allowed."],
  ],
  "taskbar-behavior": [
    ["taskbar_auto_hide", "Automatically hide the taskbar", "Shows the taskbar when you move the pointer to its edge of the screen."],
    ["taskbar_badges", "Show badges on taskbar apps", "Shows unread counts and other app status indicators."],
    ["taskbar_share_window", "Share windows from the taskbar", "Shows a window sharing shortcut during supported calls."],
    ["taskbar_show_desktop", "Use the far corner to show the desktop", "Minimizes open windows when you click the far edge of the taskbar."],
    ["taskbar_combine", "Combine taskbar buttons", "Choose when buttons and labels are combined on the primary taskbar.", combineButtons],
  ],
  "taskbar-tray": [
    ["taskbar_show_clock", "Show time and date in the system tray", "Shows the calendar and clock on the taskbar."],
    ["taskbar_clock_seconds", "Show seconds in the system tray clock", "Adds seconds to the clock display."],
    ["taskbar_abbreviated_clock", "Use abbreviated time and date", "Uses a compact clock and calendar format."],
    ["taskbar_notification_bell", "Show notification bell", "Shows the notification center icon in the system tray."],
    ["notification_center_clock", "Show clock in Notification Center", "Shows a clock above the calendar and notifications."],
  ],
  "desktop-icon-settings": [
    ["desktop_icons", "Hide desktop icons", "Hides icons on the desktop without removing them."],
  ],
})) {
  const heading = byId(group);
  let nextGroup = heading.nextElementSibling;
  while (nextGroup && !nextGroup.classList.contains("preference-group")) nextGroup = nextGroup.nextElementSibling;
  for (const [field, label, description, options] of preferences) {
    const id = field.replaceAll("_", "-");
    const row = element("div", "preference");
    const content = element("div");
    const title = element("h3", "", label);
    content.append(title);
    if (description) content.append(element("p", "preference-description", description));
    row.append(content);
    heading.parentElement.insertBefore(row, nextGroup);
    const preference = { id, field, shell: true };
    if (options) {
      preference.picker = addFolderPicker(row, title, id, options);
      preference.personal = true;
      explorerChoices.push(preference);
    } else {
      row.dataset.preference = id;
      explorerPreferences.push(preference);
    }
  }
}

function currentPreferenceValue(preference) {
  if (!preference.shell) return settings?.[preference.field];
  const value = settings?.shell_preferences?.[preference.field]?.value;
  return preference.picker || value === undefined ? value : value === 1;
}

function preferenceRecommendation(preference) {
  if (preference.shell) {
    const value = settings?.shell_preferences?.[preference.field]?.recommended ?? undefined;
    return preference.picker || value === undefined ? value : value === 1;
  }
  if (preference.picker) return preference.recommendedValue;
  return settings?.recommended_preferences?.find(([field]) => field === preference.field)?.[1];
}

function preferenceRecommendationLabel(preference) {
  const value = preferenceRecommendation(preference);
  if (preference.recommendation) return preference.recommendation;
  return `${preference.label} · ${preference.picker ? preference.picker.labelFor(value) : preferenceValue(value)}`;
}

function preferenceRecommended(preference) {
  return preferenceRecommendation(preference) !== undefined;
}

function preferencePersonal(preference) {
  return preference.personal ?? !preferenceRecommended(preference);
}

function preferenceAvailable(preference) {
  return Boolean(settings) && (preference.shell ? currentPreferenceValue(preference) !== undefined
    : !preference.supported || settings[preference.supported]);
}

function preferenceRestriction(preference, unavailable = "Unavailable on this system") {
  if (!preferenceAvailable(preference)) return unavailable;
  return preference.requiresAdmin && !settings.is_administrator ? "Requires administrator access" : "";
}

function preferenceValue(enabled) {
  return enabled === null ? "Custom or missing overlay icon" : enabled ? "Enabled" : "Disabled";
}

function setStatus(message, error = false) {
  writeTweaksOutput(message || "Ready.", { error });
  const notice = byId("administrator-notice");
  if (error && message.includes("requires administrator access")) {
    byId("administrator-description").textContent = message;
    if (!notice.open) notice.showModal();
    return;
  }
  if (notice.open) notice.close();
}

function setBusy(value) {
  busy = value;
  byId("refresh").disabled = busy || !invoke;
  byId("desktop-layout-safe-mode-toggle").disabled = busy || !settings;
  for (const id of ["apply-recommended", "restore-original", "import-tweaks", "export-tweaks"]) {
    byId(id).disabled = busy || !settings;
  }
  for (const view of explorerChoices) view.picker.setDisabled(busy || !preferenceAvailable(view));
  for (const preference of explorerPreferences) {
    byId(`${preference.id}-toggle`).disabled = busy || !preferenceAvailable(preference);
  }
  renderPendingChanges();
}

function setPreferenceValue(id, value, visual = false) {
  const element = byId(id);
  element.textContent = value;
  element.dataset.state = visual ? "visual" : value === "Enabled" ? "enabled" : value === "Disabled" ? "disabled" : "other";
}

function renderBohr() {
  if (!settings) return;
  const available = [...explorerPreferences, ...explorerChoices]
    .filter(preference => preferenceAvailable(preference) && preferenceRecommended(preference)
      && (!preference.picker || preference.picker.choices.some(option => preference.picker.matches(option.value, preferenceRecommendation(preference)))))
    .map(preference => ({ ...preference, label: preferenceRecommendationLabel(preference),
      enabled: preferenceMatches(preference, preferenceRecommendation(preference), currentPreferenceValue(preference)) }));
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
  const orbitScale = (29 / 60 * 100) / (13 + (shellCount - 1) * 8 + 2.5);
  diagram.style.setProperty("--electron-size", `${5 * orbitScale}%`);
  const shells = [];
  for (let index = 0, offset = 0; index < shellCount; index++) {
    const electrons = changes.slice(offset, offset + (index === 0 ? 2 : 8));
    const radius = (13 + index * 8) * orbitScale;
    const shell = element("div", "bohr-shell");
    shell.style.setProperty("--orbit-size", `${radius * 2 * 12 / 11.36}%`);
    electrons.forEach((preference, position) => {
      const angle = position * Math.PI * 2 / electrons.length - Math.PI / 2;
      const electron = element("span", "bohr-electron");
      electron.style.left = `${50 + Math.cos(angle) * radius}%`;
      electron.style.top = `${50 + Math.sin(angle) * radius}%`;
      electron.title = preference.label;
      shell.append(electron);
    });
    shells.push(shell);
    offset += electrons.length;
  }
  diagram.replaceChildren(...shells, diagram.querySelector(".bohr-nucleus"));
}

function renderSettings() {
  renderPendingChanges();
  const safeMode = Boolean(settings?.desktop_layout_safe_mode);
  byId("desktop-layout-safe-mode-toggle").setAttribute("aria-checked", String(safeMode));
  byId("desktop-layout-safe-mode-toggle").disabled = busy || !settings;
  setPreferenceValue("desktop-layout-safe-mode-value", settings ? safeMode ? "Enabled" : "Disabled" : "Loading…");
  document.querySelectorAll(".admin-warning").forEach(adminWarning);
  document.querySelectorAll(".admin-requirement").forEach(note => {
    note.dataset.state = settings?.is_administrator === true ? "granted" : settings?.is_administrator === false ? "missing" : "unknown";
  });
  for (const view of explorerChoices) {
    const selected = pendingValue(view);
    const available = preferenceAvailable(view);
    const recommended = preferenceRecommendation(view);
    view.picker.load(selected ?? null, available);
    const value = byId(`${view.id}-value`);
    if (available && pendingPreferences.get(view)?.selected) value.textContent += " · Pending";
    value.dataset.state = !available ? "other" : view.personal || recommended === undefined ? "visual"
      : view.picker.matches(recommended, selected) ? "enabled" : "disabled";
  }
  for (const preference of explorerPreferences) {
    const available = preferenceAvailable(preference);
    const selected = pendingValue(preference);
    setPreferenceValue(preference.id, available ? preferenceValue(selected) : "Unavailable", preferencePersonal(preference));
    if (available && pendingPreferences.get(preference)?.selected) {
      byId(preference.id).textContent += " · Pending";
    }
    if (available && preferenceRecommended(preference) && !preferencePersonal(preference) && selected !== null) {
      byId(preference.id).dataset.state = selected === preferenceRecommendation(preference) ? "enabled" : "disabled";
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
      check.classList.add("icon", greenCheck ? "icon-check" : yellowDash ? "icon-minus" : "icon-close");
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
  warning.replaceChildren(element("span", "icon icon-warning"));
  if (administrator != null) {
    warning.append(element("span", administrator ? "icon icon-check" : "icon icon-close"));
  }
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
  byId("confirm-description").hidden = !description;
  byId("confirm-description").textContent = description;
  byId("confirm-note").textContent = note;
  byId("recommended-options").hidden = !recommendations.length;
  byId("recommended-options").open = true;
  byId("recommendation-list").replaceChildren(...recommendations.map((id) => {
    const preference = [...explorerPreferences, ...explorerChoices].find(preference => preference.field === id);
    const { row, text } = createChangeRow(id, preferenceRecommendationLabel(preference),
      preference.description, preferencePersonal(preference));
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
    setStatus(error, Boolean(error));
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
  const title = byId(`${view.id}-label`);
  view.label = title.textContent.trim();
  view.description = title.parentElement.querySelector(".preference-description").textContent.trim();
}
for (const preference of explorerPreferences) {
  const row = document.querySelector(`[data-preference="${preference.id}"]`);
  const title = row.querySelector("h3");
  preference.description = row.querySelector(".preference-description")?.textContent.trim() || "";
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
  toggle.addEventListener("click", () => {
    if (busy || !preferenceAvailable(preference)) return;
    if (preference.requiresAdmin && !settings.is_administrator) {
      setStatus("This preference requires administrator access. Close Orbital and run it as administrator.", true);
      return;
    }
    const current = currentPreferenceValue(preference);
    stagePreference(preference, pendingPreferences.get(preference)?.selected ? current : !current);
  });
}

if (invoke) {
  loadAppVersion();
  refresh();
} else {
  renderSettings();
  byId("system").textContent = "Open Orbital with npm run dev to read system information.";
  setStatus("The Rust backend is available inside the desktop app.", true);
}
