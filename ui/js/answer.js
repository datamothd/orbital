"use strict";

const answer = (() => {
  const ns = "urn:schemas-microsoft-com:unattend", extraNs = "urn:orbital:answer";
  const wcm = "http://schemas.microsoft.com/WMIConfig/2002/State";
  const passes = ["windowsPE", "offlineServicing", "generalize", "specialize", "auditSystem", "auditUser", "oobeSystem"];
  const utf8 = new TextEncoder();
  const yesNo = [["", "Windows default"], ["true", "Y"], ["false", "N"]];
  const options = values => values.map(value => Array.isArray(value) ? value : [value, value]);
  const choice = (values, config = {}) => ({ choices: options(values), toggle: values === yesNo, ...config });
  const languageNames = new Intl.DisplayNames(["en"], { type: "language" });
  const languages = [["", "Windows default"], ..."en-US en-GB ar-SA bg-BG cs-CZ da-DK de-DE el-GR es-ES es-MX et-EE fi-FI fr-CA fr-FR he-IL hr-HR hu-HU it-IT ja-JP ko-KR lt-LT lv-LV nb-NO nl-NL pl-PL pt-BR pt-PT ro-RO ru-RU sk-SK sl-SI sr-Latn-RS sv-SE th-TH tr-TR uk-UA zh-CN zh-TW".split(" ").map(code => [code, languageNames.of(code)])];
  const number = (value, min, max) => ({ type: "number", value, min, max, required: true });
  const defaultEditions = ["Home", "Home N", "Pro", "Pro N", "Pro Education", "Pro for Workstations", "Education", "Enterprise", "Enterprise LTSC", "Enterprise N LTSC", "IoT Enterprise LTSC"].map(edition => `Windows 11 ${edition}`);
  let isoEditions = null;
  const groups = [
    ["installation", "Installation", [
      ["edition", "Windows edition", choice([["", "Choose during Setup"], ...defaultEditions], { help: "When an ISO is selected, this list shows its image names. Without an ISO, it shows common Windows 11 editions." })],
      ["product-key", "Installation product key", { wide: true, placeholder: "XXXXX-XXXXX-XXXXX-XXXXX-XXXXX", help: "Leave blank to skip key entry. A supplied key must match the Windows image.", pattern: "[A-Za-z0-9]{5}(-[A-Za-z0-9]{5}){4}" }],
      ["product-key-ui", "Skip product key screen", choice([["", "Windows default"], ["Never", "Y"], ["Always", "N"]], { toggle: true, help: "Equivalent to skipping product-key entry during installation." })],
      ["accept-eula", "Accept Windows license terms", choice(yesNo)]
    ]],
    ["storage", "Storage", [
      ["disk-mode", "Installation disk", choice([["", "Choose during Setup"], ["erase", "Erase and partition a disk"], ["existing", "Use an existing partition"]], { help: "Existing-partition installation writes Windows to the selected destination; back up its files first." })],
      ["disk", "Target disk number", { ...number("0", 0, 999), when: () => ["erase", "existing"].includes(value("disk-mode")) }],
      ["partition", "Target partition number", { ...number("1", 1, 999), when: () => value("disk-mode") === "existing", help: "Partition numbers start at 1. The partition must already be prepared for Windows Setup." }],
      ["disk-confirm", "Confirm disk number to erase", { required: true, when: () => value("disk-mode") === "erase", help: "Every partition and file on this disk will be erased during the Windows installation. Type its number again." }]
    ]],
    ["language", "Language & region", [
      ["language", "Language", choice(languages, { help: "Sets the setup and Windows display language, keyboard layout and regional formats. The selected language must be available in your Windows image." })]
    ]],
    ["identity", "Computer name", [["computer", "Computer name", { help: "Leave blank for Windows defaults or enter * for a random name. Custom names are limited to 15 bytes." }]]],
    ["accounts", "Local accounts", []],
    ["welcome", "Windows welcome", [
      ["hide-license", "Skip license terms page", choice(yesNo)], ["hide-oem", "Skip manufacturer registration", choice(yesNo)],
      ["hide-online", "Skip Microsoft account screens", choice(yesNo, { help: "Configure a local account. Support depends on the Windows image." })],
      ["hide-wireless", "Skip Wi-Fi setup screen", choice(yesNo)]
    ]],
    ["privacy", "Setup privacy", [
      ["protection", "Decline optional setup data sharing", choice([["", "Windows default"], ["3", "Y"], ["1", "N"]], { toggle: true, help: "Y chooses the privacy-friendly answers for the setup privacy page using Microsoft's bundled answer-file setting. Individual choices depend on the Windows image; required diagnostic data can still be sent." })]
    ]],
    ["advanced", "Advanced XML", [
      ["mode", "Answer file source", choice([["guided", "Installation settings"], ["xml", "Complete XML"]], { value: "guided" })],
      ["xml", "Complete answer file", { type: "textarea", wide: true, required: true, when: () => value("mode") === "xml", help: "This XML is used as the answer file. Copy settings to XML replaces it with the current installation settings." }]
    ]]
  ];
  const rowFields = {
    accounts: [["name", "User name", { required: true }], ["password", "Password", { type: "password", help: "Leave blank for an account without a password." }]]
  };
  const rowLabels = { accounts: "local account" };
  const controls = new Map(), sections = new Map(), lists = new Map(), pickers = new WeakMap();
  function layoutFields(grids) {
    for (const grid of grids) {
      if (!grid.clientWidth) continue;
      const style = getComputedStyle(grid), gap = parseFloat(style.columnGap) || 0;
      const columnWidth = (grid.clientWidth - (parseFloat(style.paddingLeft) || 0) - (parseFloat(style.paddingRight) || 0) - gap) / 2;
      for (const field of grid.querySelectorAll(":scope > .answer-field:not(.answer-wide)")) {
        if (!field.getClientRects().length) continue;
        const title = field.querySelector(".answer-field-label"), children = [...title.children];
        const width = children.reduce((total, child) => total + child.getBoundingClientRect().width, 0) + (children.length - 1) * (parseFloat(getComputedStyle(title).columnGap) || 0);
        field.classList.toggle("answer-auto-wide", width > columnWidth);
      }
      const children = [...grid.children];
      for (const child of children) child.classList.remove("answer-stacked-dropdown");
      if (getComputedStyle(grid).gridTemplateColumns.split(" ").length !== 2) continue;
      const layoutRows = [];
      for (const child of children) {
        if (!child.getClientRects().length) continue;
        const wide = child.classList.contains("answer-wide") || child.classList.contains("answer-auto-wide");
        const last = layoutRows.at(-1);
        if (!wide && last?.length === 1 && !last[0].classList.contains("answer-wide") && !last[0].classList.contains("answer-auto-wide")) last.push(child);
        else layoutRows.push([child]);
      }
      for (let index = 0; index < layoutRows.length - 1; index++) {
        const first = layoutRows[index], second = layoutRows[index + 1];
        if (first.length === 1 && second.length === 1 && first[0].classList.contains("answer-field") && second[0].classList.contains("answer-field")) {
          for (const field of [first[0], second[0]]) if (field.querySelector(":scope > .answer-dropdown")) field.classList.add("answer-stacked-dropdown");
        }
      }
    }
  }
  const fieldLayout = new ResizeObserver(entries => requestAnimationFrame(() => layoutFields(entries.map(entry => entry.target))));
  let rowId = 0, initialized = false;
  const read = control => control.type === "password" ? control.value : control.value.trim();
  const value = id => !controls.has(id) || controls.get(id).disabled ? "" : read(controls.get(id));
  const rows = kind => [...lists.get(kind).children];
  const rowControl = (row, key) => row.querySelector(`[data-key="${key}"]`);
  const rowValue = (row, key) => read(rowControl(row, key));
  const focusControl = control => control.hidden ? control.closest(".answer-field").querySelector(".answer-dropdown") || control : control;
  function actionButton(className, text = "", title = text) {
    const button = element("button", className, text);
    button.type = "button";
    button.title = title;
    button.setAttribute("aria-label", title);
    return button;
  }
  function dropdown(picker, label) {
    const button = actionButton("folder-view-picker answer-dropdown", "", label);
    const text = element("span"), arrow = element("span", "picker-arrow"), menu = element("div", "folder-view-options");
    button.id = `${picker.id}-button`;
    for (const [name, value] of Object.entries({ role: "combobox", "aria-haspopup": "listbox", "aria-expanded": "false", "aria-controls": `${picker.id}-options` })) button.setAttribute(name, value);
    menu.id = `${picker.id}-options`; menu.hidden = true; menu.tabIndex = -1; menu.setAttribute("role", "listbox"); menu.setAttribute("aria-label", label);
    arrow.setAttribute("aria-hidden", "true");
    button.append(text, arrow); picker.hidden = true; picker.after(button, menu);
    pickers.set(picker, createChoicePicker(button, menu, text, () => [...picker.options].filter(option => !option.hidden && !option.disabled).map(option => ({ value: option.value, label: option.textContent })), label, selected => {
      picker.value = selected.value;
      for (const type of ["input", "change"]) picker.dispatchEvent(new Event(type, { bubbles: true }));
    }));
  }
  function field(parent, id, label, config) {
    const wrapper = element(config.choices ? "div" : "label", `answer-field${config.wide || config.toggle ? " answer-wide" : ""}${config.toggle ? " answer-binary" : ""}`), title = element("span", "answer-field-label"), caption = element("span", "", label);
    title.append(caption);
    if (!config.toggle && !config.required && !config.value && (!config.choices || config.choices.some(([key]) => key === ""))) caption.append(element("small", "answer-optional", "(optional)"));
    if (config.help) {
      const help = element("span", "answer-info");
      help.innerHTML = '<svg viewBox="0 0 12 12" width="12" height="12" fill="currentColor" shape-rendering="crispEdges" aria-hidden="true"><path d="M5 2h2v2H5zM5 5h2v5H5z"/></svg>';
      help.title = config.help; help.tabIndex = 0; help.setAttribute("aria-label", config.help);
      title.append(help);
    }
    const input = element(config.toggle ? "input" : config.choices ? "select" : config.type === "textarea" ? "textarea" : "input");
    if (config.toggle) input.hidden = true;
    else if (config.choices) for (const [key, text] of config.choices) input.add(new Option(text, key, key === (config.value ?? ""), key === (config.value ?? "")));
    else if (input.tagName === "INPUT") input.type = config.type || "text";
    input.id = `answer-${id}`; input.dataset.key = id; input.dataset.label = label;
    for (const key of ["placeholder", "min", "max", "maxLength", "pattern", "required"]) if (config[key] != null) input[key] = config[key];
    if (input.type === "number") input.step = "1";
    if (!config.choices || config.toggle) input.defaultValue = config.value || "";
    input.autocomplete = input.type === "password" ? "new-password" : "off";
    wrapper.append(title, input);
    if (config.toggle) {
      const buttons = element("div", "answer-toggle"); buttons.setAttribute("role", "group"); buttons.setAttribute("aria-label", label);
      for (const [key, text] of config.choices.slice(1)) {
        const button = actionButton(text === "N" ? "answer-no" : "", text, text === "Y" ? "Yes" : "No");
        button.dataset.value = key;
        button.addEventListener("click", () => { input.value = key; update(); });
        buttons.append(button);
      }
      const forget = actionButton("answer-forget", "", `Forget change: ${label}`);
      forget.title = "Forget change (Windows default)";
      forget.innerHTML = '<svg viewBox="0 0 16 16" width="16" height="16" fill="currentColor" shape-rendering="crispEdges" aria-hidden="true"><path d="M6 1h4v2H6zM2 3h12v2H2zM3 6h2v7h6V6h2v7h-1v2H4v-2H3zM7 6h2v5H7z"/></svg>';
      forget.addEventListener("click", () => { input.value = ""; update(); }); buttons.append(forget); wrapper.append(buttons);
    }
    for (const picker of wrapper.querySelectorAll("select")) dropdown(picker, label);
    parent.append(wrapper);
    fieldLayout.observe(parent);
    return input;
  }
  function addRow(kind, values = {}) {
    const row = element("details", "answer-row"), summary = element("summary"), title = element("span", "answer-row-title"), count = element("span", "answer-count"), grid = element("div", "answer-grid");
    row.open = !Object.keys(values).length;
    if (row.open) for (const entry of rows(kind)) entry.open = false;
    count.append(element("span")); count.hidden = true;
    for (const [key, label, config] of rowFields[kind]) {
      const input = field(grid, `row-${++rowId}`, label, config);
      input.dataset.key = key;
      if (values[key] != null) input.value = values[key];
    }
    const remove = actionButton("answer-row-remove", "", "Remove entry");
    remove.innerHTML = '<svg viewBox="0 0 16 16" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true"><path d="m4 4 8 8m0-8-8 8"/></svg>';
    remove.addEventListener("click", event => { event.preventDefault(); const focus = (row.nextElementSibling || row.previousElementSibling)?.querySelector("summary") || lists.get(kind).nextElementSibling; row.remove(); update(); focus.focus(); });
    summary.append(title, count, remove); row.append(summary, grid); lists.get(kind).append(row); update();
  }
  function initialize() {
    if (initialized) return;
    initialized = true;
    for (const [id, title, fields] of groups) {
      const section = element("details"), summary = element("summary", "", title), count = element("span", "answer-count");
      section.id = `answer-group-${id}`; section.open = id === "installation";
      count.append(element("span")); count.hidden = true; summary.append(count); section.append(summary);
      const grid = element("div", "answer-grid");
      for (const [key, label, config] of fields) controls.set(key, field(grid, key, label, config));
      if (rowFields[id]) {
        const list = element("div", "answer-wide answer-list"), add = actionButton("answer-add answer-wide", `Add ${rowLabels[id]}`);
        add.addEventListener("click", () => addRow(id)); lists.set(id, list); grid.append(list, add);
      }
      section.append(grid); sections.set(id, section);
      byId("answer-groups").append(section);
    }
    const copy = actionButton("answer-add answer-wide", "Copy settings to XML");
    copy.addEventListener("click", () => attempt(() => { const xml = build("guided"); controls.get("xml").value = xml; controls.get("mode").value = "xml"; update(); status("Installation settings copied to XML."); }));
    const copyNote = element("p", "note answer-wide", "Copying settings replaces any XML in the complete answer file field.");
    controls.get("xml").closest(".answer-field").before(copy, copyNote);
    byId("answer-form").addEventListener("input", update);
    byId("answer-form").addEventListener("change", update);
    byId("answer-form").addEventListener("reset", () => { for (const list of lists.values()) list.replaceChildren(); byId("answer-search").value = ""; requestAnimationFrame(update); });
    refreshEditions();
    update();
  }
  function refreshEditions() {
    if (!initialized || !controls.has("edition")) return;
    const control = controls.get("edition"), selected = control.value;
    const onlyLtsc = isoEditions?.length === 1 && /\bLTSC\b/i.test(isoEditions[0]);
    control.replaceChildren(...(onlyLtsc ? [] : [new Option("Choose during Setup", "")]), ...(isoEditions ?? defaultEditions).map(name => new Option(name, name)));
    control.value = onlyLtsc ? isoEditions[0] : [...control.options].some(option => option.value === selected) ? selected : "";
    if (selected && !control.value) status("The previous Windows edition is not available on the selected ISO. Choose an available image.", true);
  }
  function setIso(file) {
    isoEditions = file ? (Array.isArray(file.editions) ? file.editions : []) : null;
    if (initialized) { refreshEditions(); update(); }
  }
  function update() {
    closeChoicePicker();
    if (isoEditions?.length === 1 && /\bLTSC\b/i.test(isoEditions[0])) controls.get("edition").value = isoEditions[0];
    for (let pass = 0; pass < 2; pass++) for (const [, , fields] of groups) for (const [id, , config] of fields) {
      const control = controls.get(id), visible = !config.when || config.when();
      control.closest(".answer-field").hidden = control.disabled = !visible;
      if (config.toggle) {
        for (const button of control.parentElement.querySelectorAll("[data-value]")) button.setAttribute("aria-pressed", String(button.dataset.value === control.value));
        control.parentElement.querySelector(".answer-forget").disabled = !control.value;
      }
    }
    lists.get("accounts").nextElementSibling.disabled = rows("accounts").length >= 500;
    rows("accounts").forEach((row, index) => { row.querySelector(".answer-row-title").textContent = rowValue(row, "name") || `Local account ${index + 1}`; });
    for (const picker of byId("answer-form").querySelectorAll("select")) {
      const control = pickers.get(picker);
      control.load(picker.value);
      control.setDisabled(picker.disabled);
    }
    filterSearch(byId("answer-search").value.trim().toLowerCase());
    requestAnimationFrame(() => layoutFields(byId("answer-groups").querySelectorAll(".answer-grid")));
    if (value("mode") === "xml") sections.get("advanced").open = true;
    renderIssues(validate());
  }
  function filterSearch(term) {
    const matches = text => text.toLowerCase().includes(term);
    for (const [id, title] of groups) {
      const section = sections.get(id), categoryMatches = Boolean(term && matches(title));
      let hasMatch = false;
      for (const child of section.querySelector(":scope > .answer-grid").children) {
        if (child.classList.contains("answer-list")) {
          let listMatches = false;
          for (const row of child.children) {
            const rowMatches = categoryMatches || matches(row.querySelector(".answer-row-title").textContent);
            let fieldMatches = false;
            for (const field of row.querySelectorAll(":scope > .answer-grid > .answer-field")) {
              const match = rowMatches || matches(field.querySelector("[data-label]").dataset.label);
              field.classList.toggle("answer-search-hidden", Boolean(term && !match));
              if (match && !field.hidden) fieldMatches = true;
            }
            const match = rowMatches || fieldMatches;
            row.classList.toggle("answer-search-hidden", Boolean(term && !match));
            if (term && match) row.open = true;
            if (match) listMatches = true;
          }
          child.classList.toggle("answer-search-hidden", Boolean(term && !listMatches));
          if (listMatches) hasMatch = true;
        } else {
          const label = child.classList.contains("answer-field") ? child.querySelector("[data-label]").dataset.label : child.textContent;
          const match = categoryMatches || matches(label);
          child.classList.toggle("answer-search-hidden", Boolean(term && !match));
          if (match && !child.hidden) hasMatch = true;
        }
      }
      section.hidden = Boolean(term && !categoryMatches && !hasMatch);
      if (term && !section.hidden) section.open = true;
    }
  }
  function parse(source, root = "unattend", namespace = ns) {
    const xml = new DOMParser().parseFromString(source, "application/xml");
    if (xml.doctype || xml.querySelector("parsererror") || xml.documentElement.localName !== root || xml.documentElement.namespaceURI !== namespace) throw new Error(`Provide valid ${root} XML with the ${namespace} namespace and no document type.`);
    return xml;
  }
  function validate(mode = value("mode")) {
    const issues = [], add = (targets, message) => issues.push({ controls: [targets].flat().map(target => typeof target === "string" ? controls.get(target) : target), message });
    if (mode === "xml") {
      try {
        const xml = parse(value("xml"));
        if (utf8.encode(value("xml")).length > 1048576) add("xml", "Answer files must be under 1 MiB.");
        for (const setting of xml.documentElement.children) if (setting.localName === "settings") {
          if (!passes.includes(setting.getAttribute("pass"))) add("xml", "Every settings element needs a supported configuration pass.");
          for (const component of setting.children) if (component.localName === "component" && !component.getAttribute("name")) add("xml", "Every component needs a name.");
        }
      } catch (error) { add("xml", error.message); }
      return issues;
    }
    for (const control of byId("answer-form").querySelectorAll("input, select, textarea")) {
      if (control === controls.get("xml") || control.disabled || control.closest(".answer-field[hidden]")) continue;
      if (!control.validity.valid) add(control, `${control.dataset.label}: ${control.validationMessage}`);
      else if (control.required && !read(control)) add(control, `Enter ${control.dataset.label.toLowerCase()}.`);
      if (control.maxLength >= 0 && read(control).length > control.maxLength) add(control, `${control.dataset.label} must be at most ${control.maxLength} characters.`);
      if (/[\u0000-\u0008\u000b\u000c\u000e-\u001f\ud800-\udfff\ufffe\uffff]/u.test(read(control))) add(control, "Remove characters that XML does not support.");
    }
    const name = value("computer");
    if (name && name !== "*" && (utf8.encode(name).length > 15 || !/^[\p{L}\p{M}\p{N}_-]+$/u.test(name) || /^\d+$/.test(name))) add("computer", "Use a computer name up to 15 bytes with letters, numbers, underscores or hyphens; it cannot contain only numbers.");
    if (value("disk-mode") === "erase" && value("disk-confirm") && value("disk-confirm") !== String(Number(value("disk")))) add(["disk", "disk-confirm"], "Confirm disk erasure by typing the target disk number again.");
    const seen = new Map();
    for (const row of rows("accounts")) {
      const control = rowControl(row, "name"), name = rowValue(row, "name");
      if (name && (name.length > 256 || /["/\\\[\]:|<>+=;,?*%@\u0000-\u001f]/.test(name) || name.endsWith(".") || name.toUpperCase() === "NONE")) add(control, "Use a valid local account name without reserved characters.");
      if (name && seen.has(name.toLowerCase())) add([seen.get(name.toLowerCase()), control], "Account names must be unique.");
      if (name) seen.set(name.toLowerCase(), control);
    }
    return issues;
  }
  function renderIssues(issues, scroll = false) {
    for (const field of byId("answer-form").querySelectorAll(".answer-invalid")) { field.classList.remove("answer-invalid"); field.querySelector(".answer-field-error")?.remove(); for (const control of field.querySelectorAll("[aria-invalid]")) { control.removeAttribute("aria-invalid"); control.removeAttribute("aria-describedby"); } }
    const counts = new Map(), summary = byId("answer-errors"), list = element("ul");
    summary.replaceChildren(element("h2", "", `${issues.length} ${issues.length === 1 ? "issue prevents" : "issues prevent"} creating this answer file`), list); summary.hidden = !issues.length;
    for (const issue of issues) {
      const item = element("li"), button = actionButton("", issue.message);
      button.addEventListener("click", () => { byId("answer-search").value = ""; update(); const control = focusControl(issue.controls[0]); for (let section = control.closest("details"); section; section = section.parentElement.closest("details")) section.open = true; control.scrollIntoView({ block: "center" }); control.focus({ preventScroll: true }); });
      item.append(button); list.append(item);
      const affected = new Set();
      for (const control of issue.controls) {
        const field = control.closest(".answer-field"); field.classList.add("answer-invalid");
        let message = field.querySelector(".answer-field-error");
        if (!message) { message = element("span", "answer-field-error"); message.id = `${control.id}-error`; field.append(message); }
        message.textContent += (message.textContent ? " " : "") + issue.message;
        for (const target of new Set([control, focusControl(control)])) { target.setAttribute("aria-invalid", "true"); target.setAttribute("aria-describedby", message.id); }
        for (let section = control.closest("details"); section; section = section.parentElement.closest("details")) affected.add(section);
      }
      for (const section of affected) counts.set(section, (counts.get(section) || 0) + 1);
    }
    for (const section of [...sections.values(), ...Object.keys(rowFields).flatMap(rows)]) { const badge = section.querySelector(".answer-count"), count = counts.get(section) || 0; badge.hidden = !count; badge.firstChild.textContent = count; badge.setAttribute("aria-label", `${count} blocking issues`); }
    if (scroll && issues.length) { window.scrollTo(0, window.scrollY + byId("answer-search").getBoundingClientRect().bottom); summary.focus({ preventScroll: true }); }
  }
  function status(text, error = false) {
    const notice = byId("answer-status");
    notice.textContent = text;
    notice.classList.toggle("error", error);
  }
  async function attempt(operation) {
    try {
      await operation();
    } catch (error) {
      if (error.issues) renderIssues(error.issues, true);
      const report = byId("answer-page").hidden ? setInjectStatus : status;
      report(error.message || String(error), true);
    }
  }
  function capture() {
    const values = Object.fromEntries([...controls].map(([key, control]) => [key, read(control)]));
    const entries = Object.fromEntries(Object.keys(rowFields).map(kind => [kind,
      rows(kind).map(row => Object.fromEntries(rowFields[kind].map(([key]) => [key, rowValue(row, key)])))
    ]));
    return { format: "orbital-answer", version: 1, values, ...entries };
  }
  function restore(profile) {
    if (!profile || profile.format !== "orbital-answer" || profile.version !== 1 || !profile.values || Array.isArray(profile.values)) throw new Error("Choose installation settings exported by this version of Orbital.");
    const check = (data, fields) => {
      if (!data || typeof data !== "object" || Array.isArray(data)) throw new Error("Invalid settings entry.");
      for (const [key, text] of Object.entries(data)) {
        const config = fields.find(([id]) => id === key)?.[2];
        if (!config || typeof text !== "string" || config.choices && !config.choices.some(([value]) => value === text) && key !== "edition") throw new Error(`Unsupported setting or value: ${key}.`);
      }
    };
    check(profile.values, groups.flatMap(([, , fields]) => fields));
    if (["commands", "scripts", "data-images", "partitions", "drivers", "winpe-interfaces", "winpe-routes"].some(kind => profile[kind]?.length)) throw new Error("These settings contain removed advanced options. Import their original answer XML using Complete XML instead.");
    for (const [kind, fields] of Object.entries(rowFields)) {
      const entries = profile[kind] ?? [];
      if (!Array.isArray(entries) || entries.length > 500) throw new Error(`Invalid ${kind} entries.`);
      for (const row of entries) check(row, fields);
    }
    byId("answer-form").reset();
    if (isoEditions === null && profile.values.edition && ![...controls.get("edition").options].some(option => option.value === profile.values.edition)) controls.get("edition").add(new Option(profile.values.edition, profile.values.edition));
    for (const [key, text] of Object.entries(profile.values)) controls.get(key).value = text;
    if (!Object.hasOwn(profile.values, "product-key-ui")) controls.get("product-key-ui").value = "Never";
    for (const kind of Object.keys(rowFields)) for (const row of profile[kind] || []) addRow(kind, row);
    controls.get("disk-confirm").value = ""; update();
  }
  function base64(text, utf16 = false) {
    const bytes = utf16 ? new Uint8Array(text.length * 2) : utf8.encode(text);
    if (utf16) { const view = new DataView(bytes.buffer); for (let i = 0; i < text.length; i++) view.setUint16(i * 2, text.charCodeAt(i), true); }
    let binary = ""; for (let i = 0; i < bytes.length; i += 8192) binary += String.fromCharCode(...bytes.subarray(i, i + 8192));
    return btoa(binary);
  }
  function fingerprint(root) {
    const copy = root.cloneNode(true);
    for (const node of [...copy.children]) {
      if (node.namespaceURI === extraNs && node.localName === "Configuration") node.remove();
    }
    let hash = 2166136261;
    for (const character of new XMLSerializer().serializeToString(copy)) {
      hash = Math.imul(hash ^ character.charCodeAt(0), 16777619);
    }
    return (hash >>> 0).toString(16);
  }
  function indentGeneratedXml(element, depth = 0) {
    const children = [...element.children];
    if (!children.length || [...element.childNodes].some(child => child.nodeType !== Node.ELEMENT_NODE)) return;
    for (const child of children) {
      element.insertBefore(element.ownerDocument.createTextNode("\n" + "  ".repeat(depth + 1)), child);
      indentGeneratedXml(child, depth + 1);
    }
    element.append(element.ownerDocument.createTextNode("\n" + "  ".repeat(depth)));
  }
  function serialize(xml) {
    const text = '<?xml version="1.0" encoding="utf-8"?>\n' + new XMLSerializer().serializeToString(xml.documentElement);
    if (utf8.encode(text).length > 1048576) throw new Error("The generated answer file exceeds 1 MiB. Reduce embedded content.");
    return text;
  }
  function node(parent, path, text, repeated = false) {
    const parts = path.split("/"), xml = parent.ownerDocument;
    for (const [index, part] of parts.entries()) {
      const add = repeated && index === parts.length - 1;
      let child = add ? null : [...parent.children].find(child => child.localName === part);
      if (!child) {
        child = xml.createElementNS(parent.namespaceURI, part);
        if (add) child.setAttributeNS(wcm, "wcm:action", "add");
        parent.append(child);
      }
      parent = child;
    }
    if (text !== undefined) parent.textContent = String(text);
    return parent;
  }
  function build(mode = value("mode")) {
    const issues = validate(mode); renderIssues(issues);
    if (issues.length) throw Object.assign(new Error(`Fix ${issues.length} ${issues.length === 1 ? "issue" : "issues"} listed above.`), { issues });
    if (mode === "xml") return serialize(parse(value("xml")));
    const xml = document.implementation.createDocument(ns, "unattend"), root = xml.documentElement;
    root.setAttributeNS("http://www.w3.org/2000/xmlns/", "xmlns:wcm", wcm);
    const names = { setup: "Microsoft-Windows-Setup", shell: "Microsoft-Windows-Shell-Setup", locale: "Microsoft-Windows-International-Core", winpe: "Microsoft-Windows-International-Core-WinPE", deployment: "Microsoft-Windows-Deployment" };
    const architecture = "amd64";
    function component(pass, name) {
      let settings = [...root.children].find(child => child.getAttribute("pass") === pass);
      if (!settings) { settings = node(root, "settings", undefined, true); settings.removeAttributeNS(wcm, "action"); settings.setAttribute("pass", pass); }
      name = names[name] || name;
      let result = [...settings.children].find(child => child.getAttribute("name") === name);
      if (!result) { result = xml.createElementNS(ns, "component"); for (const [key, text] of Object.entries({ name, processorArchitecture: architecture, publicKeyToken: "31bf3856ad364e35", language: "neutral", versionScope: "nonSxS" })) result.setAttribute(key, text); settings.append(result); }
      return result;
    }
    const setting = (pass, name, path, text) => node(component(pass, name), path, text);
    const finish = () => {
      for (const pass of passes) { const settings = [...root.children].find(child => child.getAttribute("pass") === pass); if (settings) root.append(settings); }
      for (const settings of [...root.children].filter(child => child.localName === "settings")) for (const original of [...settings.children]) { const copy = original.cloneNode(true); copy.setAttribute("processorArchitecture", "arm64"); settings.append(copy); }
      for (const child of [...root.children]) if (child.namespaceURI !== ns) root.append(child);
      const metadata = xml.createElementNS(extraNs, "Configuration"), profile = capture(); profile.values.mode = "guided"; delete profile.values.xml; metadata.textContent = base64(JSON.stringify(profile)); root.append(metadata);
      indentGeneratedXml(root); metadata.setAttribute("hash", fingerprint(root));
      return serialize(xml);
    };
    for (const [id, pass, name, path] of [
      ["product-key", "windowsPE", "setup", "UserData/ProductKey/Key"], ["product-key-ui", "windowsPE", "setup", "UserData/ProductKey/WillShowUI"], ["accept-eula", "windowsPE", "setup", "UserData/AcceptEula"], ["computer", "specialize", "shell", "ComputerName"],
      ["hide-license", "oobeSystem", "shell", "OOBE/HideEULAPage"], ["hide-oem", "oobeSystem", "shell", "OOBE/HideOEMRegistrationScreen"], ["hide-online", "oobeSystem", "shell", "OOBE/HideOnlineAccountScreens"], ["hide-wireless", "oobeSystem", "shell", "OOBE/HideWirelessSetupInOOBE"], ["protection", "oobeSystem", "shell", "OOBE/ProtectYourPC"]
    ]) if (value(id)) setting(pass, name, path, value(id));
    if (value("language")) {
      setting("windowsPE", "winpe", "SetupUILanguage/UILanguage", value("language"));
      for (const path of ["UILanguage", "UserLocale", "InputLocale", "SystemLocale"]) for (const [pass, name] of [["windowsPE", "winpe"], ["oobeSystem", "locale"]]) setting(pass, name, path, value("language"));
    }
    if (value("edition")) { const metadata = node(component("windowsPE", "setup"), "ImageInstall/OSImage/InstallFrom/MetaData", undefined, true); node(metadata, "Key", "/IMAGE/NAME"); node(metadata, "Value", value("edition")); }
    const destination = "ImageInstall/OSImage/InstallTo";
    if (value("disk-mode") === "existing") for (const [id, path] of [["disk", "DiskID"], ["partition", "PartitionID"]]) setting("windowsPE", "setup", `${destination}/${path}`, Number(value(id)));
    for (const row of rows("accounts")) {
      const account = node(component("oobeSystem", "shell"), "UserAccounts/LocalAccounts/LocalAccount", undefined, true);
      node(account, "Name", rowValue(row, "name")); node(account, "Group", "Administrators");
      node(account, "Password/Value", rowValue(row, "password")); node(account, "Password/PlainText", "true");
    }
    const orders = {}, commands = [];
    if (value("disk-mode") === "erase") {
      const disk = Number(value("disk"));
      const lines = [`select disk ${disk}`, "clean", "convert gpt", "create partition efi size=300", "format quick fs=fat32 label=System", "create partition msr size=16", "create partition primary", "format quick fs=ntfs label=Windows"];
      lines.push("shrink desired=2048 minimum=2048", "create partition primary", "format quick fs=ntfs label=Recovery", "set id=de94bba4-06d1-4d40-a16a-bfd50179d6ac", "gpt attributes=0x8000000000000001");
      lines.push("exit");
      commands.push(["windowsPE", `cmd.exe /d /c "wpeutil.exe UpdateBootInfo && (reg.exe query HKLM\\SYSTEM\\CurrentControlSet\\Control /v PEFirmwareType | findstr.exe /c:0x2 >nul) && (${lines.map(line => `echo ${line}`).join("&")}) > \"%TEMP%\\OrbitalDisk.txt\" && diskpart.exe /s \"%TEMP%\\OrbitalDisk.txt\""`, "Verify firmware and partition the confirmed disk", ""]);
      for (const [path, text] of [["DiskID", disk], ["PartitionID", 3]]) setting("windowsPE", "setup", `${destination}/${path}`, text);
      setting("windowsPE", "setup", "ImageInstall/OSImage/WillShowUI", "OnError");
    }
    for (const [pass, path, description, reboot] of commands) {
      if (path.length > (pass === "windowsPE" ? 1024 : 259)) throw new Error("A generated installation command exceeds the Windows limit.");
      const kind = "RunSynchronous", orderKey = pass;
      const order = orders[orderKey] = (orders[orderKey] || 0) + 1;
      if (order > 500) throw new Error("Windows supports up to 500 commands per configuration pass.");
      const command = node(component(pass, pass === "windowsPE" ? "setup" : "deployment"), `${kind}/${kind}Command`, undefined, true);
      node(command, "Order", order); node(command, "Path", path); if (description) node(command, "Description", description); if (reboot && pass !== "windowsPE") node(command, "WillReboot", reboot);
    }
    return finish();
  }
  function show(open) {
    closeChoicePicker(); initialize(); byId("answer-page").hidden = !open; byId("inject-panel").hidden = open; document.querySelector(".top-bar").hidden = open;
    if (typeof stopScrolling === "function") stopScrolling(); window.scrollTo(0, 0); (open ? byId("answer-search") : byId("inject-xml")).focus({ preventScroll: true });
  }
  function showImportedChanges() {
    byId("answer-search").value = "";
    update();
    for (const detail of byId("answer-groups").querySelectorAll("details")) detail.open = false;
    if (value("mode") === "xml") {
      sections.get("advanced").open = true;
      return;
    }
    for (const [id, , fields] of groups) {
      const changed = fields.some(([key, , config]) => {
        const control = controls.get(key), defaultValue = config.value ?? (config.toggle ? "" : config.choices?.[0]?.[0] ?? "");
        return !control.disabled && read(control) !== defaultValue;
      });
      sections.get(id).open = changed || Boolean(lists.get(id)?.children.length);
    }
  }
  const presetPicker = createChoicePicker(
    byId("answer-preset-picker"), byId("answer-preset-options"), byId("answer-preset-selection"),
    () => [{ value: "easy", label: "Easy installation" }, { value: "defaults", label: "Windows defaults" }, ...(debugMode.enabled ? [{ value: "debug", label: "All preferences (debug)" }, { value: "debug-install", label: "Installation test (debug)" }] : [])],
    "Choose a preset", choice => {
      initialize();
      if (choice.value === "defaults") {
        byId("answer-form").reset();
        showImportedChanges();
        status("Windows defaults restored.");
      } else if (choice.value === "easy") {
        restore({ format: "orbital-answer", version: 1, values: { "accept-eula": "true", "hide-license": "true", "hide-oem": "true", "hide-wireless": "true", "product-key-ui": "Never", protection: "3" }, accounts: [] });
        showImportedChanges();
        status("Easy installation loaded. Choose your edition, language and accounts. Optional setup data sharing is declined.");
      } else {
        const profile = {
          format: "orbital-answer", version: 1,
          values: { "accept-eula": "true", "product-key-ui": "Never", language: "en-US", computer: "ORBITAL-DEBUG", "hide-license": "true", "hide-oem": "true", "hide-online": "true", "hide-wireless": "true", protection: "3", mode: "guided" },
          accounts: [{ name: "OrbitalDebug", password: "DebugOnly123!" }]
        };
        if (choice.value === "debug") Object.assign(profile.values, { "disk-mode": "erase", disk: "1" });
        restore(profile);
        showImportedChanges();
        status("Debug preset loaded. Review the example account credentials; disk erasure still requires confirmation.");
      }
      byId("answer-preset-picker").title = `${choice.label} (preset)`;
    }
  );
  presetPicker.load(null);
  document.addEventListener("orbital-debug-mode-change", () => {
    presetPicker.load(presetPicker.selected);
    if (presetPicker.selected !== null) byId("answer-preset-picker").title = `${presetPicker.label} preset`;
  });
  function importText(source) {
    initialize();
    if (utf8.encode(source).length > 1048576) throw new Error("Choose a file under 1 MiB.");
    if (source.trimStart().startsWith("{")) restore(JSON.parse(source));
    else {
      const xml = parse(source), metadata = [...xml.documentElement.children].find(child => child.namespaceURI === extraNs && child.localName === "Configuration");
      let restored = false;
      if (metadata && metadata.getAttribute("hash") === fingerprint(xml.documentElement)) try {
        restore(JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode(Uint8Array.from(atob(metadata.textContent), character => character.charCodeAt(0)))));
        restored = true;
      } catch {}
      if (!restored) { controls.get("xml").value = serialize(xml); controls.get("mode").value = "xml"; update(); }
    }
    showImportedChanges();
    status("Settings imported. Confirm disk erasure or formatting again if applicable." + (isoEditions !== null ? " Check the Windows edition against the selected ISO." : ""));
  }
  async function upload(settings) {
    const input = byId("answer-upload"); input.accept = settings ? ".xml,.json" : ".xml"; input.value = "";
    const file = await new Promise(resolve => {
      const done = () => { input.removeEventListener("change", done); input.removeEventListener("cancel", done); resolve(input.files[0]); };
      input.addEventListener("change", done); input.addEventListener("cancel", done); input.click();
    });
    if (!file) return false;
    let selected = false;
    await attempt(async () => {
      if (!file.size || file.size > 1048576) throw new Error("Choose a non-empty file under 1 MiB.");
      const bytes = new Uint8Array(await file.arrayBuffer()), encoding = bytes[0] === 255 && bytes[1] === 254 ? "utf-16le" : bytes[0] === 254 && bytes[1] === 255 ? "utf-16be" : "utf-8", text = new TextDecoder(encoding, { fatal: true }).decode(bytes);
      if (settings) importText(text); else { parse(text); setInjectFile("xml", { name: file.name, xml: text }); }
      selected = true;
    });
    return selected;
  }
  let choosing = false;
  async function choose() {
    if (choosing) return;
    choosing = true;
    try {
      if (typeof waitForWindowLayout === "function") await waitForWindowLayout();
      const dialog = byId("answer-source"); if (dialog.open) return;
      while (await showDialog(dialog) === "continue") {
        if (dialog.querySelector("input:checked").value === "configure") { show(true); return; }
        if (invoke ? await changeInjectFile("xml") : await upload(false)) return;
      }
    } finally {
      choosing = false;
    }
  }
  byId("answer-back").addEventListener("click", () => { show(false); if (!injectFiles.xml) choose(); });
  byId("answer-reset").addEventListener("click", () => {
    presetPicker.load(null);
    for (const section of byId("answer-groups").querySelectorAll("details")) section.open = false;
  });
  byId("answer-search").addEventListener("input", update);
  byId("answer-collapse").addEventListener("click", () => { for (const section of byId("answer-groups").querySelectorAll("details")) section.open = false; });
  byId("answer-import").addEventListener("click", () => attempt(async () => {
    if (invoke) { const text = await invoke("answer_document", { content: null, kind: "open" }); if (text !== null) importText(text); }
    else await upload(true);
  }));
  for (const button of document.querySelectorAll("[data-answer-save]")) button.addEventListener("click", () => attempt(async () => {
    const content = build();
    if (invoke) { if (await invoke("answer_document", { content, kind: "xml" }) !== null) status("File saved."); }
    else { const link = element("a"), url = URL.createObjectURL(new Blob([content], { type: "application/xml" })); link.href = url; link.download = "autounattend.xml"; link.click(); setTimeout(() => URL.revokeObjectURL(url), 1000); status("File downloaded."); }
  }));
  byId("answer-form").addEventListener("submit", event => { event.preventDefault(); attempt(async () => { const xml = build(); const file = invoke ? await invoke("set_answer_file", { xml }) : { name: "Orbital answer file.xml", xml }; setInjectFile("xml", file); show(false); }); });
  return { choose, setIso };
})();
