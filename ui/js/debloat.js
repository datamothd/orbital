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
  const log = byId("uninstall-log");
  const selected = new Set();
  let apps = [];
  let operation = "";
  let inventoryValid = false;
  let drag = null;
  let dragFrame = 0;
  let suppressClick = false;
  let suppressContextMenu = false;

  function status(message, error = false) {
    byId("uninstall-status").textContent = message;
    byId("uninstall-status").classList.toggle("error", error);
  }

  function updateControls() {
    refreshButton.disabled = Boolean(operation) || !invoke;
    uninstallButton.disabled = Boolean(operation) || !inventoryValid || !selected.size;
    uninstallButton.textContent = operation === "uninstall" ? "Uninstalling…" : "Uninstall apps";
    inventory.setAttribute("aria-busy", String(Boolean(operation)));
    const selection = byId("uninstall-selection");
    selection.textContent = `${selected.size} ${selected.size === 1 ? "app" : "apps"} selected`;
    selection.dataset.selected = String(selected.size > 0);
    for (const checkbox of rows.querySelectorAll("input")) {
      checkbox.disabled = Boolean(operation) || !inventoryValid || checkbox.dataset.selectable !== "true";
      checkbox.checked = selected.has(checkbox.value);
    }
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
    if (![0, 2].includes(event.button) || event.pointerType !== "mouse" || operation || !inventoryValid) return;
    const row = event.target.closest(event.button === 2 ? ".uninstall-app-row, .uninstall-app-details" : ".uninstall-app-row");
    if (!row || !apps[Number(row.dataset.index)]?.selectable) return;
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
    if (operation || !inventoryValid) return;
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
    if (!row || operation || !inventoryValid) return;
    toggleSelection(Number(row.dataset.index));
  });

  function render(result) {
    finishDrag();
    apps = result.apps;
    selected.clear();
    const selectionHeader = element("th");
    selectionHeader.scope = "col";
    selectionHeader.textContent = "select";
    const nameHeader = element("th", "", result.headers[0].toLocaleLowerCase());
    nameHeader.scope = "col";
    byId("uninstall-columns").replaceChildren(selectionHeader, nameHeader);
    rows.replaceChildren(...apps.flatMap((app, index) => {
      const row = element("tr", "uninstall-app-row");
      row.dataset.index = String(index);
      row.dataset.selectable = String(app.selectable);
      const selectCell = element("td");
      const checkbox = document.createElement("input");
      checkbox.type = "checkbox";
      checkbox.value = app.key;
      checkbox.dataset.selectable = String(app.selectable);
      checkbox.setAttribute("aria-label", `Select ${app.name}, version ${app.version}`);
      if (!app.selectable) {
        row.title = "Refresh the installed apps list before selecting this app.";
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
      toggle.append(element("span", "", app.name), element("span", "uninstall-app-arrow"));
      toggle.lastChild.setAttribute("aria-hidden", "true");
      const details = element("tr", "uninstall-app-details");
      details.id = `uninstall-details-${index}`;
      details.dataset.index = String(index);
      details.hidden = true;
      const detailCell = element("td");
      detailCell.colSpan = 2;
      const metadata = element("dl");
      for (let column = 1; column < result.headers.length; column += 1) {
        metadata.append(element("dt", "", result.headers[column].toLocaleLowerCase()), element("dd", "", app.cells[column] || "—"));
      }
      for (const field of app.details || []) {
        metadata.append(element("dt", "", field.label.toLocaleLowerCase()), element("dd", "", field.value || "—"));
      }
      if (!app.selectable) {
        const warning = element("p", "uninstall-app-warning", row.title);
        detailCell.append(warning);
      }
      detailCell.prepend(metadata);
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
    byId("uninstall-empty").textContent = "WinGet did not report any installed apps.";
    tableScroll.scrollTop = 0;
  }

  async function loadApps() {
    finishDrag();
    inventoryValid = false;
    selected.clear();
    updateControls();
    if (!apps.length) {
      byId("uninstall-empty").hidden = false;
      byId("uninstall-empty").textContent = "Reading installed apps…";
    }
    try {
      refreshDots.hidden = false;
      const onCommand = new window.__TAURI__.core.Channel();
      onCommand.onmessage = command => {
        const commandText = byId("uninstall-command-text");
        commandText.textContent = "winget list --details";
        commandText.title = command;
      };
      const result = await invoke("list_installed_apps", { onCommand });
      render(result);
      inventoryValid = true;
      return null;
    } catch (reason) {
      if (!apps.length) byId("uninstall-empty").textContent = "Could not load installed apps. Try Refresh.";
      const message = reason?.message || String(reason).split(/\r?\n/, 1)[0];
      appendLog(reason?.output || (typeof reason === "string" ? reason : message));
      return message;
    } finally {
      refreshDots.hidden = true;
    }
  }

  async function refreshApps() {
    if (operation) return;
    if (!invoke) {
      byId("uninstall-empty").textContent = "Open the Orbital desktop app to read installed apps with WinGet.";
      status("WinGet is available in the desktop app.");
      updateControls();
      return;
    }
    operation = "refresh";
    status("Running winget list…");
    const error = await loadApps();
    operation = "";
    status(error || `${apps.length} ${apps.length === 1 ? "app" : "apps"} found.`, Boolean(error));
    updateControls();
  }

  function appendLog(text) {
    log.hidden = false;
    log.textContent += `${log.textContent ? "\n\n" : ""}${text}`;
    log.scrollTop = log.scrollHeight;
  }

  async function uninstallSelected() {
    if (operation || !inventoryValid || !selected.size) return;
    const chosen = apps.filter(app => selected.has(app.key));
    operation = "confirm";
    updateControls();
    byId("uninstall-confirm-title").textContent = `Uninstall ${chosen.length} ${chosen.length === 1 ? "app" : "apps"}?`;
    byId("uninstall-confirm-list").replaceChildren(...chosen.map(app => {
      const item = element("li", "", app.name);
      item.append(element("span", "", `${app.id} · ${app.version}`));
      return item;
    }));
    if (!await confirmDialog(byId("uninstall-confirm"))) {
      operation = "";
      updateControls();
      return;
    }
    operation = "uninstall";
    updateControls();
    log.textContent = "";
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
          appendLog(`${event.success ? "Removed" : "Could not remove"}: ${event.name}\n${event.output}`);
        }
      };
      results = await invoke("uninstall_apps", { keys: chosen.map(app => app.key), onEvent });
    } catch (reason) {
      failure = String(reason);
      appendLog(failure);
    }
    status("Refreshing installed apps…");
    const refreshError = await loadApps();
    operation = "";
    const removed = results?.filter(result => result.success).length || 0;
    const failed = results ? results.length - removed : 0;
    const summary = failure || `${removed} ${removed === 1 ? "app" : "apps"} uninstalled.${failed ? ` ${failed} could not be removed. See the output above.` : ""}`;
    status(`${summary}${refreshError ? ` Could not refresh: ${refreshError}` : ""}`, Boolean(failure || failed || refreshError));
    updateControls();
  }

  for (const section of [category, parent, dropdown]) {
    section.addEventListener("toggle", () => {
      if (category.open && parent.open && dropdown.open) refreshApps();
      else finishDrag(true);
    });
  }
  refreshButton.addEventListener("click", refreshApps);
  uninstallButton.addEventListener("click", uninstallSelected);
  log.tabIndex = 0;
  updateControls();
})();
