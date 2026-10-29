"use strict";

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
  byId("desktop-layout-safe-mode-toggle").disabled = busy || !settings;
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
  const available = explorerPreferences.filter(preference => !preference.visual && preferenceAvailable(preference));
  const changes = available
    .filter(preference => settings[preference.field] === true)
    .map(preference => preference.label);
  const count = byId("preferences-made");
  if (count) count.textContent = `${changes.length} / ${available.length}`;
  const diagram = byId("bohr");
  diagram.setAttribute("aria-label", `${changes.length} preferences differ from Windows defaults`);
  if (diagram.dataset.changes === changes.join("|")) return;
  diagram.dataset.changes = changes.join("|");
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
    electrons.forEach((label, position) => {
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
      title.textContent = label;
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
  for (const view of folderViews) view.picker.load(settings?.[view.field] ?? null, Boolean(settings));
  for (const preference of explorerPreferences) {
    setPreferenceValue(preference.id, preferenceAvailable(preference)
      ? preferenceValue(preference, settings[preference.field]) : "Unavailable",
      preference.visual);
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
  const testedOs = /^Windows 11 Pro$/i.test(info.os.trim());
  const testedVersion = windows11 && windowsVersion === "25H2";
  const release = /^(\d{2})H([12])$/.exec(windowsVersion);
  const newerWindows11 = windows11 && release !== null && Number(release[1]) > 25;
  const rows = [
    ["Host", info.host], ["OS", info.os], ["Windows", info.windows_version],
    ["CPU", info.cpu], ["Threads", info.threads],
    ["Memory", `${info.used_memory_gib.toFixed(1)} / ${info.total_memory_gib.toFixed(1)} GiB`],
    ["Changes:", "…"],
  ];
  byId("system").replaceChildren(...rows.flatMap(([label, value]) => {
    const term = document.createElement("dt");
    const description = document.createElement("dd");
    term.textContent = label;
    description.textContent = value;
    if (label === "Changes:") {
      description.className = "changes-count";
      description.replaceChildren(byId("changes-count").content.cloneNode(true));
    }
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
  if (administrator !== undefined && administrator !== null) {
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

function confirmChange(description, title = "Change preference?", recommendations = []) {
  const dialog = byId("confirmation");
  dialog.classList.toggle("changes-dialog", recommendations.length > 0);
  byId("confirm-title").textContent = title;
  byId("confirm-description").textContent = description;
  byId("recommended-options").hidden = !recommendations.length;
  byId("recommended-options").open = true;
  const list = byId("recommendation-list");
  list.replaceChildren(...recommendations.map((id) => {
    const preference = explorerPreferences.find(preference => preference.field === id);
    const { row, checkbox, text } = createChangeRow(id, preference?.label
      || (id === "folder_sort" ? "Sort folders by Type · A to Z" : "Set folder grouping to None"));
    const view = folderViews.find(view => view.field === id);
    const source = preference
      ? document.querySelector(`[data-preference="${preference.id}"]`)
      : view && byId(`${view.id}-label`).closest(".preference");
    const note = source?.querySelector(".preference-description")?.textContent.trim();
    if (note) {
      row.title = note;
      checkbox.setAttribute("aria-description", note);
    }
    if (preference?.requiresAdmin) text.append(adminWarning());
    if (preference?.visual || folderViews.some(view => view.field === id)) {
      row.className = "visual-preference";
    }
    return row;
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
  "restore_original_settings", {}, "Restore your original settings? If folder views were changed, this also restores their saved layout snapshot and replaces folder layout changes made since then. Desktop icon layouts are preserved.", false, "Original settings restored. Explorer restarted.",
));
for (const view of folderViews) {
  byId(`${view.id}-apply`).addEventListener("click", (event) => {
    const selected = view.picker.selected;
    if (selected !== null) applyChange(
      view.command,
      { [view.argument]: selected },
      `Use ${view.picker.label.toLowerCase()} for folder ${view.label} across all folder types? Saved folder views will be reset. Desktop icon layouts are preserved.`,
      event.shiftKey,
      `Default folder ${view.label} saved. Explorer restarted.`,
    );
  });
}
for (const preference of explorerPreferences) {
  const row = document.querySelector(`[data-preference="${preference.id}"]`);
  const title = row.querySelector("h3");
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
