"use strict";

function batchSelection() {
  const profile = {
    format: "orbital-tweaks", version: 3,
    tweaks: [], disabled_tweaks: [], folder_sort: null, folder_grouping: null,
  };
  for (const checkbox of byId("batch-list").querySelectorAll("input:enabled")) {
    if (checkbox.indeterminate || checkbox.checked === settings[checkbox.value]) continue;
    profile[checkbox.checked ? "tweaks" : "disabled_tweaks"].push(checkbox.value);
  }
  for (const view of folderViews) {
    const select = byId(`batch-${view.id}`);
    if (select.value === "") continue;
    const value = view.picker.choices[Number(select.value)].value;
    if (!view.picker.matches(value, settings[view.field])) profile[view.field] = value;
  }
  return profile;
}

function updateBatchSelection() {
  const count = batchChangeCount(batchSelection());
  byId("batch-apply").disabled = count === 0;
  byId("batch-summary").hidden = count === 0;
  byId("batch-summary").textContent = count
    ? `${count} ${count === 1 ? "change" : "changes"} pending.` : "No changes pending.";
}

function batchChangeCount(profile) {
  return profile.tweaks.length + profile.disabled_tweaks.length
    + Number(profile.folder_sort !== null) + Number(profile.folder_grouping !== null);
}

function prepareBatchDialog() {
  byId("batch-options").open = true;
  const rows = new Map();
  for (const preference of explorerPreferences) {
    const { row, checkbox, text } = createChangeRow(preference.field,
      preference.id === "taskbar-centering" ? "Left-align taskbar icons" : preference.label,
      preference.description, preference.visual);
    checkbox.checked = settings[preference.field] === true;
    checkbox.indeterminate = settings[preference.field] === null;
    const blocked = preferenceRestriction(preference);
    checkbox.disabled = Boolean(blocked);
    if (preference.requiresAdmin) text.append(adminWarning());
    if (blocked || checkbox.indeterminate) {
      const note = blocked || "Custom or missing overlay icon · unchanged until selected";
      row.title = [preference.description, note].filter(Boolean).join(" · ");
      checkbox.setAttribute("aria-description", row.title);
    }
    rows.set(preference.row, row);
  }
  for (const view of folderViews) {
    const row = element("div", "batch-folder-view visual-preference");
    const label = element("span", "", `Default folder ${view.label}`);
    label.id = `batch-${view.id}-label`;
    const select = document.createElement("select");
    select.id = `batch-${view.id}`;
    select.setAttribute("aria-labelledby", label.id);
    const current = settings[view.field];
    const index = view.picker.choices.findIndex(choice => view.picker.matches(choice.value, current));
    if (index < 0) select.add(new Option(`Keep current · ${view.picker.labelFor(current)}`, ""));
    view.picker.choices.forEach((choice, index) => select.add(new Option(choice.label, String(index))));
    select.value = index < 0 ? "" : String(index);
    row.append(label, select);
    rows.set(view.row, row);
  }
  byId("batch-list").replaceChildren(...[...document.querySelectorAll("#explorer-preferences .preference")]
    .map(source => rows.get(source)).filter(Boolean));
  updateBatchSelection();
}

byId("batch-dialog").addEventListener("change", updateBatchSelection);
byId("batch-changes").addEventListener("click", async () => {
  if (busy || !settings) return;
  setBusy(true);
  try {
    await loadSettings();
    prepareBatchDialog();
    if (!await confirmDialog(byId("batch-dialog"))) return;
    const profile = batchSelection();
    if (!batchChangeCount(profile)) return;
    setStatus("Saving preferences and restarting Explorer…");
    const { changed, error, cancelled } = await savePreferences("apply_imported_tweaks", { profile });
    if (cancelled) return;
    setStatus("");
    showResult(error ? "Could not complete batch changes" : "Batch changes applied", error
      ? "Some changes may already have been applied. Review your current settings before trying again."
      : changed === 0 ? "Your preferences already match. No restart was needed."
      : `${changed} ${changed === 1 ? "change" : "changes"} applied. Explorer restarted once.`, error || "");
  } catch (error) {
    setStatus(String(error), true);
  } finally {
    setBusy(false);
  }
});
