"use strict";

(() => {
  const dropdown = byId("uninstall-apps");
  const parent = byId("debloat-preferences");
  const category = byId("optimization-settings");
  const inventory = byId("uninstall-inventory");
  const tableScroll = inventory.querySelector(".uninstall-table-scroll");
  const rows = byId("uninstall-rows");
  const refreshButton = byId("uninstall-refresh");
  const refreshDots = byId("uninstall-refresh-dots");
  const uninstallButton = byId("uninstall-start");
  const permanentCheckbox = byId("uninstall-permanent");
  const selected = new Set();
  let apps = [];
  let installed = { headers: ["Name", "Id", "Version"], apps: [] };
  let view = "installed";
  const known = new Map();
  const selectionAllowed = () => view === "default" || inventoryValid;
  const selectionKey = app => {
    const entry = defaultWindowsApps.find(entry => defaultAppMatch(entry, app));
    return entry ? `${entry.kind}:${entry.id.toLowerCase()}` : `${app.kind}:${app.id.toLowerCase()}`;
  };
  const chosenInstalled = () => inventoryValid ? installed.apps.filter(app => app.selectable && (!app.requires_permanent || permanentRemoval) && selected.has(selectionKey(app))) : [];
  window.orbitalAppSelections = {
    export: () => [...selected].map(key => known.get(key)).filter(Boolean),
    restore: entries => {
      for (const imported of entries || []) {
        const entry = { ...imported, kind: imported.kind === "winget" ? "desktop" : imported.kind };
        const key = `${entry.kind}:${entry.id.toLowerCase()}`;
        known.set(key, entry);
        selected.add(key);
      }
      updateControls();
    },
  };
  for (const entry of defaultWindowsApps) known.set(`${entry.kind}:${entry.id.toLowerCase()}`, { name: entry.name, id: entry.id, kind: entry.kind });
  let operation = "";
  let inventoryValid = false;
  let permanentRemoval = false;
  let drag = null;
  let dragFrame = 0;
  let suppressClick = false;
  let suppressContextMenu = false;

  function sortApps(list) {
    const rank = app => app.protected ? 2 : app.requires_permanent && !permanentRemoval ? 1 : 0;
    list.sort((a, b) => rank(a) - rank(b)
      || (view === "default" ? Number(b.state === "Installed") - Number(a.state === "Installed") : 0)
      || a.name.localeCompare(b.name) || a.id.localeCompare(b.id));
  }

  function renderView() {
    byId("apps-installed").setAttribute("aria-pressed", String(view === "installed"));
    byId("apps-default").setAttribute("aria-pressed", String(view === "default"));
    const label = view === "default" ? "Default Windows apps" : "Installed apps";
    tableScroll.setAttribute("aria-label", label);
    byId("uninstall-table").querySelector("caption").textContent = `${label}. Select apps for removal or export.`;
    byId("uninstall-command-text").textContent = view === "default" ? "Windows app catalog" : "Read Windows apps and components";
    byId("uninstall-command-text").removeAttribute("title");
    const result = view === "installed" ? { ...installed, apps: installed.apps.map(app => ({ ...app, key: selectionKey(app),
      protected: !app.selectable,
      selectable: app.selectable && (!app.requires_permanent || permanentRemoval),
      blocked_reason: app.blocked_reason || (app.requires_permanent && !permanentRemoval ? "Enable permanent removal and acknowledge the warning to remove these component files." : ""),
    })) } : {
      headers: ["Name", "Package", "Status"],
      apps: defaultWindowsApps.map(entry => {
        const matches = inventoryValid ? installed.apps.filter(app => defaultAppMatch(entry, app)) : [];
        return {
          ...entry, key: `${entry.kind}:${entry.id.toLowerCase()}`, selectable: !matches.length || matches.some(app => app.selectable && (!app.requires_permanent || permanentRemoval)),
          requires_permanent: matches.some(app => app.selectable) && matches.filter(app => app.selectable).every(app => app.requires_permanent),
          protected: matches.length > 0 && matches.every(app => !app.selectable),
          blocked_reason: matches.find(app => !app.selectable)?.blocked_reason || "Enable permanent removal and acknowledge the warning to remove these component files.",
          version: matches[0]?.version || "—",
          state: matches.length ? "Installed" : inventoryValid ? "Not reported" : "Not checked",
          cells: [entry.name, entry.id, matches.length ? `Installed · ${matches.map(app => app.version).join(", ")}` : inventoryValid ? "Not reported by Windows" : "Installation status not checked"],
          details: [...matches.flatMap(app => app.details || []), ...(entry.caution ? [{ label: "Dependency", value: entry.caution }] : [])],
        };
      }),
    };
    sortApps(result.apps);
    render(result);
    updateControls();
  }

  function status(message, error = false, reveal = false) {
    writeTweaksOutput(message, { error, reveal });
  }

  function updateControls() {
    refreshButton.disabled = Boolean(operation) || !invoke;
    permanentCheckbox.disabled = Boolean(operation) || !invoke;
    permanentCheckbox.checked = permanentRemoval;
    uninstallButton.disabled = Boolean(operation) || !chosenInstalled().length;
    uninstallButton.textContent = operation === "uninstall" ? "Uninstalling…" : "Uninstall apps";
    inventory.setAttribute("aria-busy", String(Boolean(operation) && operation !== "permanent"));
    const selection = byId("uninstall-selection");
    selection.textContent = `${selected.size} ${selected.size === 1 ? "app" : "apps"} selected`;
    selection.dataset.selected = String(selected.size > 0);
    for (const checkbox of rows.querySelectorAll("input")) {
      checkbox.disabled = Boolean(operation) || !selectionAllowed() || checkbox.dataset.selectable !== "true";
      checkbox.checked = selected.has(checkbox.value);
    }
  }

  function updateAvailability() {
    const pairs = new Map();
    for (const row of rows.querySelectorAll(".uninstall-app-row")) {
      const app = apps[Number(row.dataset.index)];
      const details = byId(row.dataset.detailsId);
      pairs.set(app, [row, details]);
      app.selectable = !app.protected && (!app.requires_permanent || permanentRemoval);
      row.dataset.selectable = String(app.selectable);
      const checkbox = row.querySelector("input");
      checkbox.dataset.selectable = String(app.selectable);
      const detailCell = details.firstChild;
      let warning = detailCell.querySelector(".uninstall-app-warning");
      if (app.selectable) {
        row.removeAttribute("title");
        checkbox.removeAttribute("aria-description");
        if (warning) warning.hidden = true;
      } else {
        row.title = app.protected ? app.blocked_reason : "Enable permanent removal and acknowledge the warning to remove these component files.";
        checkbox.setAttribute("aria-description", row.title);
        if (!warning) {
          warning = element("p", "uninstall-app-warning");
          detailCell.append(warning);
        }
        warning.textContent = row.title;
        warning.hidden = false;
      }
    }
    const scrollTop = tableScroll.scrollTop;
    sortApps(apps);
    for (const [index, app] of apps.entries()) {
      const [row, details] = pairs.get(app);
      row.dataset.index = String(index);
      details.dataset.index = String(index);
      rows.append(row, details);
    }
    tableScroll.scrollTop = scrollTop;
  }

  function finishDrag(cancelled = false) {
    if (!drag) return;
    const previous = drag;
    drag = null;
    cancelAnimationFrame(dragFrame);
    dragFrame = 0;
    tableScroll.classList.remove("is-selecting");
    if (cancelled && previous.active) {
      selected.clear();
      for (const key of previous.initial) selected.add(key);
      updateControls();
    }
    if (tableScroll.hasPointerCapture(previous.pointerId)) tableScroll.releasePointerCapture(previous.pointerId);
    if (previous.active) {
      suppressClick = true;
    }
  }

  function paintRange() {
    if (!drag?.active) return;
    const appRows = [...rows.querySelectorAll(".uninstall-app-row")];
    const bounds = tableScroll.getBoundingClientRect();
    const headerHeight = byId("uninstall-columns").getBoundingClientRect().height;
    const y = Math.max(bounds.top + headerHeight + 1, Math.min(drag.y, bounds.bottom - 1));
    let end = appRows.length - 1;
    for (let index = 0; index < appRows.length; index += 1) {
      const next = appRows[index + 1]?.getBoundingClientRect().top ?? Infinity;
      if (y < next) { end = index; break; }
    }
    if (end === drag.end) return;
    drag.end = end;
    selected.clear();
    for (const key of drag.initial) selected.add(key);
    const first = Math.min(drag.start, end);
    const last = Math.max(drag.start, end);
    for (let index = first; index <= last; index += 1) {
      const app = apps[index];
      if (!app?.selectable) continue;
      if (drag.selecting) selected.add(app.key);
      else selected.delete(app.key);
    }
    updateControls();
  }

  function scrollWhileSelecting() {
    if (!drag?.active) return;
    const bounds = tableScroll.getBoundingClientRect();
    const edge = Math.min(36, bounds.height / 4);
    const speed = drag.y < bounds.top + edge ? -Math.min(12, (bounds.top + edge - drag.y) / 3)
      : drag.y > bounds.bottom - edge ? Math.min(12, (drag.y - bounds.bottom + edge) / 3) : 0;
    if (speed) tableScroll.scrollTop += speed;
    paintRange();
    dragFrame = requestAnimationFrame(scrollWhileSelecting);
  }

  rows.addEventListener("pointerdown", event => {
    if (![0, 2].includes(event.button) || event.pointerType !== "mouse" || operation || !selectionAllowed()) return;
    const row = event.target.closest(event.button === 2 ? ".uninstall-app-row, .uninstall-app-details" : ".uninstall-app-row");
    if (!row || !apps[Number(row.dataset.index)]?.selectable) return;
    if (event.button === 2) event.preventDefault();
    finishDrag(true);
    suppressClick = false;
    suppressContextMenu = event.button === 2;
    const start = Number(row.dataset.index);
    drag = {
      pointerId: event.pointerId, button: event.button, x: event.clientX, y: event.clientY, originY: event.clientY,
      start, end: -1, selecting: !selected.has(apps[start].key), initial: new Set(selected), active: false,
    };
  });

  window.addEventListener("pointermove", event => {
    if (!drag || event.pointerId !== drag.pointerId) return;
    drag.y = event.clientY;
    if (!drag.active) {
      if (Math.hypot(event.clientX - drag.x, event.clientY - drag.originY) < 6) return;
      drag.active = true;
      tableScroll.focus({ preventScroll: true });
      tableScroll.setPointerCapture(drag.pointerId);
      tableScroll.classList.add("is-selecting");
      dragFrame = requestAnimationFrame(scrollWhileSelecting);
    }
    event.preventDefault();
    paintRange();
  }, { passive: false });

  window.addEventListener("pointerup", event => {
    if (drag?.pointerId !== event.pointerId) return;
    drag.y = event.clientY;
    paintRange();
    if (drag.button === 2 && !drag.active) toggleSelection(drag.start);
    finishDrag();
  });
  window.addEventListener("pointercancel", event => { if (drag?.pointerId === event.pointerId) finishDrag(true); });
  tableScroll.addEventListener("lostpointercapture", event => { if (drag?.active && drag.pointerId === event.pointerId) finishDrag(true); });
  tableScroll.addEventListener("pointerdown", () => { suppressClick = false; suppressContextMenu = false; }, true);
  tableScroll.addEventListener("keydown", () => { if (!drag) { suppressClick = false; suppressContextMenu = false; } });
  window.addEventListener("blur", () => finishDrag(true));
  window.addEventListener("keydown", event => {
    if (event.key === "Escape" && drag?.active) {
      event.preventDefault();
      finishDrag(true);
    }
  });
  tableScroll.addEventListener("click", event => {
    if (!suppressClick) return;
    event.preventDefault();
    event.stopImmediatePropagation();
    suppressClick = false;
  }, true);

  function toggleSelection(index) {
    if (operation || !selectionAllowed()) return;
    const app = apps[index];
    if (!app?.selectable) return;
    if (selected.has(app.key)) selected.delete(app.key);
    else selected.add(app.key);
    updateControls();
  }

  tableScroll.addEventListener("contextmenu", event => {
    event.preventDefault();
    if (suppressContextMenu || drag) return;
    const row = event.target.closest(".uninstall-app-row, .uninstall-app-details");
    if (!row || operation || !selectionAllowed()) return;
    toggleSelection(Number(row.dataset.index));
  });

  function render(result) {
    finishDrag();
    apps = result.apps;
    byId("uninstall-name-column").textContent = result.headers[0].toLocaleLowerCase();
    rows.replaceChildren(...apps.flatMap((app, index) => {
      const row = element("tr", "uninstall-app-row");
      row.dataset.index = String(index);
      row.dataset.detailsId = `uninstall-details-${index}`;
      row.dataset.selectable = String(app.selectable);
      row.dataset.state = app.protected ? "protected" : app.state === "Installed" ? "installed" : app.state === "Not reported" ? "not-reported" : "";
      const selectCell = element("td");
      const checkbox = document.createElement("input");
      checkbox.type = "checkbox";
      checkbox.value = app.key;
      checkbox.dataset.selectable = String(app.selectable);
      checkbox.setAttribute("aria-label", `Select ${app.name}, version ${app.version}`);
      if (!app.selectable) {
        row.title = app.blocked_reason || "Windows does not allow this app to be removed.";
        checkbox.setAttribute("aria-description", row.title);
      }
      checkbox.addEventListener("change", () => {
        if (checkbox.checked) selected.add(app.key);
        else selected.delete(app.key);
        updateControls();
      });
      const nameCell = element("td");
      const toggle = element("button", "uninstall-app-toggle");
      toggle.type = "button";
      toggle.setAttribute("aria-expanded", "false");
      toggle.setAttribute("aria-controls", `uninstall-details-${index}`);
      toggle.title = app.name;
      const indicators = element("span", "uninstall-app-indicators");
      if (app.state || app.protected) indicators.append(element("span", "uninstall-app-state", app.protected ? "Protected" : app.state));
      const arrow = element("span", "uninstall-app-arrow");
      arrow.setAttribute("aria-hidden", "true");
      indicators.append(arrow);
      toggle.append(element("span", "uninstall-app-name", app.name), indicators);
      const details = element("tr", "uninstall-app-details");
      details.id = `uninstall-details-${index}`;
      details.dataset.index = String(index);
      details.hidden = true;
      const detailCell = element("td");
      detailCell.colSpan = 2;
      const metadata = element("dl");
      const fields = app.details || [];
      const valueFor = labels => [...new Set(fields.filter(field => labels.test(field.label)).map(field => field.value).filter(Boolean))].join("\n") || "—";
      const version = ["feature", "capability"].includes(app.kind) || app.version === "Bundled component files" ? "—" : app.version || "—";
      const location = valueFor(app.kind === "capability" ? /^(install location|location)$/i : /^(install location|location|component files)$/i);
      for (const [label, value] of [
        ["package", app.catalog_id || app.id],
        ["location", location],
        ["publisher", valueFor(/^publisher$/i)],
        ["version", version],
        ["scope", valueFor(/^scope$/i)],
      ]) {
        const fieldRow = element("div", "uninstall-metadata-row");
        fieldRow.append(element("dt", "", label), element("dd", "", value));
        metadata.append(fieldRow);
      }
      detailCell.append(metadata);
      const notes = fields.filter(field => /^(permanent removal|dependency)$/i.test(field.label)
        || (app.kind === "capability" && field.label === "Component files")).map(field => field.value);
      for (const note of new Set(notes.filter(Boolean))) detailCell.append(element("p", "uninstall-app-notice", note));
      if (!app.selectable) {
        const warning = element("p", "uninstall-app-warning", row.title);
        detailCell.append(warning);
      }
      details.append(detailCell);
      toggle.addEventListener("click", () => {
        details.hidden = !details.hidden;
        toggle.setAttribute("aria-expanded", String(!details.hidden));
      });
      selectCell.append(checkbox);
      nameCell.append(toggle);
      row.append(selectCell, nameCell);
      return [row, details];
    }));
    tableScroll.hidden = !apps.length;
    byId("uninstall-empty").hidden = Boolean(apps.length);
    byId("uninstall-empty").textContent = "Windows did not report any installed apps or components.";
    tableScroll.scrollTop = 0;
  }

  async function loadApps() {
    finishDrag();
    inventoryValid = false;
    updateControls();
    if (!apps.length) {
      byId("uninstall-empty").hidden = false;
      byId("uninstall-empty").textContent = "Reading installed apps…";
    }
    try {
      refreshDots.hidden = false;
      const onCommand = new window.__TAURI__.core.Channel();
      onCommand.onmessage = command => {
        if (view !== "installed") return;
        const commandText = byId("uninstall-command-text");
        commandText.textContent = "Read Windows apps and components";
        commandText.title = command;
      };
      const result = await invoke("list_installed_apps", { onCommand });
      installed = result;
      for (const app of result.apps) {
        const key = selectionKey(app);
        if (!known.has(key)) known.set(key, { name: app.name, id: app.id, kind: app.kind });
      }
      inventoryValid = true;
      renderView();
      for (const warning of result.warnings || []) appendLog(warning);
      return { warning: (result.warnings || []).join(" ") };
    } catch (reason) {
      if (!apps.length) byId("uninstall-empty").textContent = "Could not load installed apps. Try Refresh.";
      const message = reason?.message || String(reason).split(/\r?\n/, 1)[0];
      appendLog(reason?.output || (typeof reason === "string" ? reason : message), true);
      return { error: message };
    } finally {
      refreshDots.hidden = true;
      renderView();
    }
  }

  async function refreshApps() {
    if (operation) return;
    if (!invoke) {
      byId("uninstall-empty").textContent = "Open the Orbital desktop app to read installed apps and Windows components.";
      status("Windows app servicing is available in the desktop app.");
      updateControls();
      return;
    }
    operation = "refresh";
    status("Reading Windows apps and components…");
    const { error, warning } = await loadApps();
    operation = "";
    status(error || warning || "Ready.", Boolean(error), true);
    updateControls();
  }

  function appendLog(text, error = false) {
    writeTweaksOutput(text, { error, reveal: true });
  }

  async function uninstallSelected(event) {
    if (operation || !chosenInstalled().length) return;
    const skipConfirmation = skipPrompts.enabled || Boolean(event?.shiftKey);
    const chosen = chosenInstalled();
    operation = "confirm";
    updateControls();
    byId("uninstall-confirm-title").textContent = `Uninstall ${chosen.length} ${chosen.length === 1 ? "app" : "apps"}?`;
    byId("uninstall-confirm-description").textContent = `Selected apps will be removed from your PC. Windows apps are removed for all users and from provisioning for new accounts. Administrator access may be required.${permanentRemoval ? " Permanent removal is enabled: bundled component files and selected capability payloads may be deleted. Orbital cannot undo these changes." : " Windows capabilities require permanent removal to be enabled."}`;
    byId("uninstall-confirm-list").replaceChildren(...chosen.map(app => {
      const item = element("li", "", app.name);
      item.append(element("span", "", `${app.id} · ${app.version}`));
      return item;
    }));
    if (!skipConfirmation && !await confirmDialog(byId("uninstall-confirm"))) {
      operation = "";
      updateControls();
      return;
    }
    operation = "uninstall";
    updateControls();
    let results = null;
    let failure = null;
    try {
      const onEvent = new window.__TAURI__.core.Channel();
      let completed = 0;
      onEvent.onmessage = event => {
        if (event.phase === "started") {
          status(`Uninstalling ${completed + 1} of ${chosen.length}: ${event.name}…`);
          appendLog(`> ${event.command}`);
        } else {
          completed += 1;
          appendLog(`${event.success ? "Removed" : "Could not remove"}: ${event.name}\n${event.output}`, !event.success);
        }
      };
      results = await invoke("uninstall_apps", { keys: chosen.map(app => app.key), onEvent });
    } catch (reason) {
      failure = String(reason);
      appendLog(failure, true);
    }
    status("Refreshing installed apps…");
    const { error: refreshError } = await loadApps();
    operation = "";
    const removed = results?.filter(result => result.success).length || 0;
    const failed = results ? results.length - removed : 0;
    const summary = failure || `${removed} ${removed === 1 ? "app" : "apps"} uninstalled.${failed ? ` ${failed} could not be removed.` : ""}`;
    const failureDetails = results?.filter(result => !result.success).map(result => `${result.name}: ${result.output}`).join("\n\n");
    const restartRequired = results?.some(result => result.restart_required);
    status([`${summary}${restartRequired ? " Restart Windows to finish removal." : ""}${refreshError ? ` Could not refresh: ${refreshError}` : ""}`, failureDetails].filter(Boolean).join("\n\n"), Boolean(failure || failed || refreshError), true);
    updateControls();
  }

  for (const section of [category, parent, dropdown]) {
    section.addEventListener("toggle", () => {
      if (category.open && parent.open && dropdown.open && !inventoryValid) refreshApps();
      else finishDrag(true);
    });
  }
  for (const [id, next] of [["apps-installed", "installed"], ["apps-default", "default"]]) {
    byId(id).addEventListener("click", () => {
      finishDrag(true);
      view = next;
      renderView();
    });
  }
  refreshButton.addEventListener("click", refreshApps);
  permanentCheckbox.addEventListener("click", async event => {
    const skipConfirmation = skipPrompts.enabled || event.shiftKey;
    const enable = permanentCheckbox.checked;
    if (operation || !invoke) {
      permanentCheckbox.checked = permanentRemoval;
      return;
    }
    finishDrag(true);
    const previous = permanentRemoval;
    permanentRemoval = enable;
    operation = "permanent";
    updateControls();
    try {
      if (enable && !skipConfirmation && !await confirmDialog(byId("permanent-removal-confirm"))) {
        permanentRemoval = previous;
        return;
      }
      await invoke("set_permanent_removal", { enabled: enable, acknowledged: enable });
      updateAvailability();
      status(enable ? "Permanent component removal enabled for this session." : "Permanent component removal disabled.");
    } catch (reason) {
      permanentRemoval = previous;
      status(String(reason), true);
    } finally {
      operation = "";
      updateControls();
    }
  });
  uninstallButton.addEventListener("click", uninstallSelected);
  updateControls();
})();
