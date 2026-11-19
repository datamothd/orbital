"use strict";

const pendingPreferences = new Map();
const pendingBar = byId("pending-bar");
const pendingContinue = byId("pending-continue");
const pendingExportMode = byId("pending-export-mode");
let exportMode = false;
const preferenceMatches = (preference, value, current) => preference.picker
  ? value == null ? current == null : preference.picker.matches(value, current)
  : value === current;

function pendingValue(preference) {
  const change = pendingPreferences.get(preference);
  return change?.selected ? change.value : currentPreferenceValue(preference);
}

function renderPendingChanges() {
  if (settings) {
    for (const [preference, { value }] of pendingPreferences) {
      if (!preferenceAvailable(preference) || preferenceMatches(preference, value, currentPreferenceValue(preference))) pendingPreferences.delete(preference);
    }
  }
  const count = [...pendingPreferences.values()].filter(change => change.selected).length;
  pendingBar.hidden = !pendingPreferences.size;
  if (pendingBar.hidden) exportMode = false;
  pendingExportMode.setAttribute("aria-pressed", String(exportMode));
  pendingExportMode.disabled = busy || !settings;
  pendingContinue.textContent = exportMode ? "Export" : "Continue";
  pendingContinue.title = exportMode ? "Save preferences as JSON" : "Hold Shift to skip confirmation";
  byId("pending-count").textContent = `${count} ${count === 1 ? "change" : "changes"} selected`;
  byId("pending-review").disabled = busy || !settings;
  byId("pending-ignore").disabled = busy;
  pendingContinue.disabled = busy || !settings || !count;
}

pendingExportMode.addEventListener("click", () => {
  exportMode = !exportMode;
  renderPendingChanges();
});

function stagePreference(preference, value) {
  if (busy || !settings) return;
  pendingPreferences.set(preference, { value, selected: true });
  renderSettings();
}

function reviewPreferences(rows, exporting = false) {
  byId("pending-title").textContent = exporting ? "Export preferences" : "Pending changes";
  byId("pending-description").textContent = exporting
    ? "Checked preferences are enabled in the export. Use the dropdowns to change other values."
    : "Uncheck any changes you want to skip.";
  byId("pending-summary").textContent = exporting ? "List of preferences" : "List of changes";
  byId("pending-apply").textContent = exporting ? "Export" : "Done";
  byId("pending-list").setAttribute("aria-label", exporting ? "Preferences to export" : "Pending preferences");
  byId("pending-options").open = true;
  byId("pending-list").replaceChildren(...rows);
  return confirmDialog(byId("pending-dialog"));
}

function setProfilePreference(profile, preference, value) {
  const { field, shell, picker } = preference;
  if (shell) (profile.shell_preferences ??= {})[field] = Number(value);
  else if (picker) profile[field] = value;
  else {
    const previous = profile[value ? "disabled_tweaks" : "tweaks"];
    const index = previous.indexOf(field);
    if (index !== -1) previous.splice(index, 1);
    profile[value ? "tweaks" : "disabled_tweaks"].push(field);
  }
}

function preferencesInPageOrder() {
  const anchor = preference => byId(preference.picker ? `${preference.id}-label` : `${preference.id}-toggle`);
  return [...explorerPreferences, ...explorerChoices].sort((a, b) =>
    anchor(a).compareDocumentPosition(anchor(b)) & Node.DOCUMENT_POSITION_FOLLOWING ? -1 : 1);
}

function reviewChoice({ field, label, picker }, original, value) {
  const matches = (a, b) => preferenceMatches({ picker }, a, b);
  const originalLabel = picker.labelFor(original);
  const choices = [...picker.choices];
  if (!choices.some(choice => matches(choice.value, original))) {
    choices.unshift({ value: original, label: originalLabel });
  }
  const id = `review-${field}`;
  const elements = choicePickerElements(id, `${id}-options`, originalLabel);
  elements.picker.setAttribute("aria-label", label);
  elements.menu.setAttribute("aria-label", label);
  const control = createChoicePicker(elements.picker, elements.menu, elements.selection, choices, originalLabel, null, matches);
  control.load(choices.some(choice => matches(choice.value, value)) ? value : original);
  byId("pending-dialog").append(elements.menu);
  return { button: elements.picker, menu: elements.menu, control };
}

byId("pending-review").addEventListener("click", async () => {
  const fields = preferencesInPageOrder().filter(preference => pendingPreferences.has(preference)).map(preference => {
    const change = pendingPreferences.get(preference);
    if (!preference.picker) {
      return { preference, change, row: preferenceChangeRow(preference,
        `${preference.label} · ${preferenceValue(change.value)}`, change.selected) };
    }
    const choice = reviewChoice(preference, currentPreferenceValue(preference), change.value);
    const row = element("div", "export-choice");
    row.append(preferenceChangeRow(preference, preference.label, change.selected), choice.button);
    return { preference, change, row, choice };
  });
  try {
    if (!await reviewPreferences(fields.map(({ row }) => row))) return;
    const selected = new Set(selectedValues("pending-list"));
    for (const { preference, change, choice } of fields) {
      change.selected = selected.has(preference.field);
      if (choice) change.value = choice.control.selected;
    }
    renderSettings();
  } finally {
    closeChoicePicker();
    for (const { choice } of fields) choice?.menu.remove();
  }
});

byId("pending-ignore").addEventListener("click", () => {
  pendingPreferences.clear();
  renderSettings();
});

pendingContinue.addEventListener("click", async event => {
  if (busy || !settings) return;
  const selected = [...pendingPreferences].filter(([, change]) => change.selected);
  if (!selected.length) return;
  if (exportMode) return exportPreferences(selected);
  setBusy(true);
  try {
    const skip = skipPrompts.enabled || event.shiftKey;
    const count = selected.length;
    const label = `${count} preference ${count === 1 ? "change" : "changes"}`;
    const resetsViews = selected.some(([{ field }]) => folderViewFields.has(field));
    const note = `${resetsViews ? "Saved folder views will be reset. Desktop icon layouts are preserved. " : ""}Windows Explorer will restart once.`;
    if (!skip && !await confirmChange("", `Apply ${label}?`, [], note)) return;
    const profile = { format: "orbital-tweaks", version: 4, tweaks: [], disabled_tweaks: [] };
    for (const [preference, { value }] of selected) {
      setProfilePreference(profile, preference, value);
    }
    setStatus("Saving preferences and restarting Explorer…");
    const { changed, error, cancelled } = await savePreferences("apply_imported_tweaks", { profile });
    if (cancelled) return;
    if (!error) pendingPreferences.clear();
    renderSettings();
    setStatus(error || (changed ? `${changed} ${changed === 1 ? "change" : "changes"} applied. Explorer restarted.` : "Your preferences already match."), Boolean(error));
  } catch (error) {
    setStatus(String(error), true);
  } finally {
    setBusy(false);
    if (!pendingContinue.disabled && !document.querySelector("dialog:modal")) pendingContinue.focus({ preventScroll: true });
  }
});
