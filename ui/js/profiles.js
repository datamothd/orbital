"use strict";

function importedRows(profile) {
  const importedTweaks = [...profile.tweaks.map(field => [field, true]),
    ...(profile.disabled_tweaks || []).map(field => [field, false])];
  const rows = importedTweaks.map(([field, enabled]) => {
    const preference = explorerPreferences.find(preference => preference.field === field);
    const matching = settings[field] === enabled;
    const blocked = !matching && preferenceRestriction(preference, "Unavailable");
    return {
      field,
      label: preference.label,
      current: preferenceValue(settings[field]),
      imported: preferenceValue(enabled),
      matching, blocked,
    };
  });
  for (const { field, picker, title } of explorerChoices) {
    const value = profile[field];
    if (value == null) continue;
    const current = settings[field];
    rows.push({
      field,
      label: title,
      current: picker.labelFor(current),
      imported: picker.labelFor(value),
      matching: picker.matches(value, current),
      blocked: picker.choices.some(choice => picker.matches(choice.value, value)) ? "" : "Unavailable",
    });
  }
  for (const app of profile.apps_to_remove || []) {
    rows.push({ field: "app:" + app.kind + ":" + app.id, label: app.name,
      current: "Not applied", imported: "Save for later removal", matching: false, blocked: "", deferred: true });
  }
  const priority = row => row.matching ? 2 : row.blocked ? 1 : 0;
  return rows.sort((a, b) => priority(a) - priority(b));
}

function updateImportSelection() {
  const list = byId("import-list");
  const values = selectedValues("import-list");
  const selected = values.length;
  const matching = list.querySelectorAll('[data-state="matching"]').length;
  const blocked = list.querySelectorAll('[data-state="blocked"]').length;
  byId("import-apply").disabled = selected === 0;
  byId("import-apply").textContent = selected && values.every(value => value.startsWith("app:")) ? "Save selections" : "Apply & restart";
  byId("import-summary").textContent = `${selected} selected · ${matching} already match${blocked ? ` · ${blocked} unavailable` : ""}`;
}

function previewImport(profile) {
  const dialog = byId("import-preview");
  const rows = importedRows(profile);
  byId("import-description").textContent = rows.length
    ? "Compare the imported settings. App selections are saved for future exports only; importing does not uninstall apps. Uncheck any changes you want to skip."
    : "This file contains no tweaks or folder settings.";
  byId("import-options").open = true;
  byId("import-options").hidden = !rows.length;
  byId("import-list").replaceChildren(...rows.map((row) => {
    const { row: label, checkbox, text } = createChangeRow(row.field, row.label);
    label.className = "import-row";
    label.dataset.state = row.matching ? "matching" : row.blocked ? "blocked" : "change";
    checkbox.disabled = row.matching || Boolean(row.blocked);
    checkbox.checked = !checkbox.disabled;
    if (!row.matching) {
      text.append(element("span", "import-comparison", `Current: ${row.current} → Imported: ${row.imported}`));
    }
    const state = row.matching
      ? row.imported === "Enabled" ? "Already enabled" : row.imported === "Disabled" ? "Already disabled" : "Already matches"
      : row.blocked || (row.deferred ? "Save for future export" : "Will change");
    text.append(element("span", "import-state", state));
    return label;
  }));
  updateImportSelection();
  return confirmDialog(dialog);
}

byId("import-list").addEventListener("change", updateImportSelection);

byId("export-tweaks").addEventListener("click", async () => {
  if (busy || !settings) return;
  setBusy(true);
  try {
    if (await invoke("export_tweaks", { appsToRemove: window.orbitalAppSelections?.export() || [] })) showResult("Tweaks exported", "Your tweaks, folder settings and selected apps to remove were saved as JSON. App selections include apps not installed on this PC; automatic removal will be handled by future post-installation configuration.");
  } catch (error) {
    showResult("Could not export tweaks", String(error));
  } finally {
    setBusy(false);
  }
});

byId("import-tweaks").addEventListener("click", async () => {
  if (busy || !settings) return;
  setBusy(true);
  try {
    selectNavigationTab(byId("tweaks-tab"));
    byId("visual-settings").open = true;
    byId("explorer-preferences").open = true;
    const profile = await invoke("import_tweaks");
    if (!profile) return;
    await loadSettings();
    if (!await previewImport(profile)) return;
    const selected = new Set(selectedValues("import-list"));
    if (!selected.size) return;
    const appSelections = (profile.apps_to_remove || []).filter(app => selected.has("app:" + app.kind + ":" + app.id));
    window.orbitalAppSelections?.restore(appSelections);
    if (appSelections.length === selected.size) {
      showResult("App selections imported", "Saved for your next export. No apps were removed. Automatic removal will be handled by future post-installation configuration.");
      return;
    }
    const selection = {
      ...profile,
      apps_to_remove: [],
      tweaks: profile.tweaks.filter(field => selected.has(field)),
      disabled_tweaks: (profile.disabled_tweaks || []).filter(field => selected.has(field)),
      ...Object.fromEntries(explorerChoices.map(({ field }) => [field, selected.has(field) ? profile[field] : null])),
    };
    setStatus("Importing tweaks and restarting Explorer…");
    const { changed: count, error, cancelled } = await savePreferences("apply_imported_tweaks", { profile: selection });
    setStatus("");
    if (cancelled) return;
    if (error) {
      showResult("Could not complete import", `${error} Some changes may already have been applied. Review your current settings before trying again.`);
    } else {
      showResult("Tweaks imported", count === 0 ? "The selected tweaks already match. No changes were needed."
        : `${count} ${count === 1 ? "change" : "changes"} applied. Explorer restarted.`);
    }
  } catch (error) {
    showResult("Could not import tweaks", String(error));
  } finally {
    setBusy(false);
  }
});
