"use strict";

const pendingPreferences = new Map();
const pendingBar = byId("pending-bar");
const pendingContinue = byId("pending-continue");
const preferenceMatches = (preference, value, current) => preference.picker?.matches(value, current) ?? (value === current);

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
  byId("pending-count").textContent = `${count} ${count === 1 ? "change" : "changes"} selected`;
  byId("pending-review").disabled = busy || !settings;
  byId("pending-ignore").disabled = busy;
  pendingContinue.disabled = busy || !settings || !count;
}

function stagePreference(preference, value) {
  if (busy || !settings) return;
  pendingPreferences.set(preference, { value, selected: true });
  renderSettings();
}

byId("pending-review").addEventListener("click", async () => {
  byId("pending-options").open = true;
  byId("pending-list").replaceChildren(...[...pendingPreferences].map(([preference, change]) => {
    const value = preference.picker ? preference.picker.labelFor(change.value) : preferenceValue(change.value);
    return preferenceChangeRow(preference, `${preference.label} · ${value}`, change.selected);
  }));
  if (!await confirmDialog(byId("pending-dialog"))) return;
  const selected = new Set(selectedValues("pending-list"));
  for (const [preference, change] of pendingPreferences) change.selected = selected.has(preference.field);
  renderSettings();
});

byId("pending-ignore").addEventListener("click", () => {
  pendingPreferences.clear();
  renderSettings();
});

pendingContinue.addEventListener("click", async event => {
  if (busy || !settings) return;
  const selected = [...pendingPreferences].filter(([, change]) => change.selected);
  if (!selected.length) return;
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
      if (preference.shell) (profile.shell_preferences ??= {})[preference.field] = Number(value);
      else if (preference.picker) profile[preference.field] = value;
      else profile[value ? "tweaks" : "disabled_tweaks"].push(preference.field);
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
