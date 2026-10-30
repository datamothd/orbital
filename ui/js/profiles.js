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
      current: preferenceValue(preference, settings[field]),
      imported: preferenceValue(preference, enabled),
      matching, blocked,
    };
  });
  for (const { field, picker, label } of folderViews) {
    const value = profile[field];
    if (value == null) continue;
    const current = settings[field];
    rows.push({
      field,
      label: `Default folder ${label}`,
      current: picker.labelFor(current),
      imported: picker.labelFor(value),
      matching: picker.matches(value, current),
      blocked: "",
    });
  }
  const priority = row => row.matching ? 2 : row.blocked ? 1 : 0;
  return rows.sort((a, b) => priority(a) - priority(b));
}

function updateImportSelection() {
  const list = byId("import-list");
  const selected = selectedValues("import-list").length;
  const matching = list.querySelectorAll('[data-state="matching"]').length;
  const blocked = list.querySelectorAll('[data-state="blocked"]').length;
  byId("import-apply").disabled = selected === 0;
  byId("import-summary").textContent = `${selected} selected · ${matching} already match${blocked ? ` · ${blocked} unavailable` : ""}`;
}

function previewImport(profile) {
  const dialog = byId("import-preview");
  const rows = importedRows(profile);
  byId("import-description").textContent = rows.length
    ? "Compare the imported tweaks with your current settings. Uncheck any changes you want to skip."
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
      : row.blocked || "Will change";
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
    if (await invoke("export_tweaks")) showResult("Tweaks exported", "Your enabled and disabled tweaks and current folder sorting and grouping were saved as JSON.");
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
    byId("explorer-preferences").open = true;
    const profile = await invoke("import_tweaks");
    if (!profile) return;
    await loadSettings();
    if (!await previewImport(profile)) return;
    const selected = new Set(selectedValues("import-list"));
    if (!selected.size) return;
    const selection = {
      ...profile,
      tweaks: profile.tweaks.filter(field => selected.has(field)),
      disabled_tweaks: (profile.disabled_tweaks || []).filter(field => selected.has(field)),
      folder_sort: selected.has("folder_sort") ? profile.folder_sort : null,
      folder_grouping: selected.has("folder_grouping") ? profile.folder_grouping : null,
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
