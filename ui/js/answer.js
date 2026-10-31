"use strict";

const answer = (() => {
  const ns = "urn:schemas-microsoft-com:unattend", extraNs = "urn:orbital:answer";
  const wcm = "http://schemas.microsoft.com/WMIConfig/2002/State";
  const passes = ["windowsPE", "offlineServicing", "generalize", "specialize", "auditSystem", "auditUser", "oobeSystem"];
  const utf8 = new TextEncoder();
  const quote = text => "'" + text.replaceAll("'", "''") + "'";
  const nonEmptyLines = text => text.split(/\r?\n/).map(line => line.trim()).filter(Boolean);
  const yesNo = [["", "Windows default"], ["true", "Y"], ["false", "N"]];
  const options = values => values.map(value => Array.isArray(value) ? value : [value, value]);
  const choice = (values, config = {}) => ({ choices: options(values), toggle: values === yesNo, ...config });
  const languageNames = new Intl.DisplayNames(["en"], { type: "language" });
  const languages = [["", "Windows default"], ..."ar-SA bg-BG cs-CZ da-DK de-DE el-GR en-GB en-US es-ES es-MX et-EE fi-FI fr-CA fr-FR he-IL hr-HR hu-HU it-IT ja-JP ko-KR lt-LT lv-LV nb-NO nl-NL pl-PL pt-BR pt-PT ro-RO ru-RU sk-SK sl-SI sr-Latn-RS sv-SE th-TH tr-TR uk-UA zh-CN zh-TW".split(" ").map(code => [code, languageNames.of(code)])];
  const timezones = [["", "Windows default"], ...options(["UTC", "Pacific Standard Time", "Mountain Standard Time", "US Mountain Standard Time", "Central Standard Time", "Eastern Standard Time", "Atlantic Standard Time", "GMT Standard Time", "W. Europe Standard Time", "Romance Standard Time", "India Standard Time", "China Standard Time", "Tokyo Standard Time", "AUS Eastern Standard Time", "New Zealand Standard Time"])];
  const number = (value, min, max) => ({ type: "number", value, min, max, required: true });
  const groups = [
    ["installation", "Installation", [
      ["edition", "Windows edition", choice([["", "Choose during Setup"], ...[11, 10].flatMap(version => ["Home", "Home N", "Pro", "Pro N", "Pro Education", "Pro for Workstations", "Education", "Enterprise"].map(edition => `Windows ${version} ${edition}`))])],
      ["architecture", "Processor architecture", choice([["amd64", "x64"], ["arm64", "ARM64"], ["x86", "x86 (Windows 10)"], ["all", "All matching"]], { value: "amd64" })],
      ["product-key", "Installation product key", { wide: true, placeholder: "XXXXX-XXXXX-XXXXX-XXXXX-XXXXX", help: "Leave blank to skip key entry. A supplied key must match the Windows image.", pattern: "[A-Za-z0-9]{5}(-[A-Za-z0-9]{5}){4}" }],
      ["accept-eula", "Accept Windows license terms", choice(yesNo)],
      ["dynamic-update", "Download updates during setup", choice(yesNo)],
      ["compact", "Compress Windows system files", choice(yesNo, { help: "Compact OS saves disk space and may affect performance." })],
      ["encryption", "Allow automatic device encryption", choice(yesNo, { help: "N prevents automatic encryption during setup. It does not decrypt an existing drive." })]
    ]],
    ["storage", "Storage", [
      ["disk-mode", "Installation disk", choice([["", "Choose during Setup"], ["erase", "Erase and partition a disk"]])],
      ["disk", "Target disk number", { ...number("0", 0, 999), when: () => value("disk-mode") === "erase" }],
      ["disk-confirm", "Confirm disk number to erase", { required: true, when: () => value("disk-mode") === "erase", help: "Every partition and file on this disk will be erased during the Windows installation. Type its number again." }],
      ["layout", "Boot layout", choice([["gpt", "UEFI / GPT"], ["mbr", "BIOS / MBR (Windows 10)"]], { value: "gpt", when: () => value("disk-mode") === "erase" })],
      ["system-size", "System partition size (MB)", { ...number("300", 300, 32768), when: () => value("disk-mode") === "erase" }],
      ["recovery", "Create a recovery partition", choice(yesNo, { value: "true", when: () => value("disk-mode") === "erase" })],
      ["recovery-size", "Recovery partition size (MB)", { ...number("2048", 1024, 65536), when: () => value("disk-mode") === "erase" && value("recovery") !== "false" }]
    ]],
    ["language", "Language & region", [
      ["language", "Display language", { suggestions: languages, placeholder: "en-US" }],
      ["setup-language", "Setup language", { suggestions: languages, placeholder: "en-US" }],
      ["user-locale", "Regional formats", { suggestions: languages, placeholder: "en-US" }],
      ["keyboard", "Keyboard layout", { suggestions: languages, placeholder: "en-US", help: "Choose Custom for Windows input identifiers or multiple layouts separated by semicolons." }],
      ["fallback", "Fallback language", { suggestions: languages, placeholder: "en-US" }],
      ["system-locale", "Legacy language", { suggestions: languages, placeholder: "en-US" }],
      ["timezone", "Time zone", { suggestions: timezones, placeholder: "Eastern Standard Time", help: "Choose Custom for another Windows time zone identifier." }]
    ]],
    ["identity", "Device identity", [
      ["computer", "Computer name", { help: "Leave blank for Windows defaults or enter * for a random name. Custom names are limited to 15 bytes." }],
      ["owner", "Registered owner", {}], ["organization", "Registered organization", {}],
      ["activation-key", "Activation product key", { pattern: "[A-Za-z0-9]{5}(-[A-Za-z0-9]{5}){4}", help: "Optional activation key, separate from the installation key." }]
    ]],
    ["accounts", "Local accounts", [
      ["builtin-admin", "Enable built-in Administrator", choice(yesNo, { help: "Create another administrator before disabling this account." })],
      ["builtin-password", "Built-in Administrator password", { type: "password", required: true, when: () => value("builtin-admin") === "true" }],
      ["builtin-name", "Built-in Administrator sign-in name", { value: "Administrator", required: true, when: () => value("autologon") === "@Administrator", help: "Use the localized or renamed account name in your Windows image." }],
      ["autologon", "Automatically sign in as", choice([["", "Disabled"]])],
      ["logon-count", "Automatic sign-in counter", { ...number("1", 0, 4294967295), when: () => Boolean(value("autologon")), help: "Windows adds one to positive values, so 1 can allow two automatic sign-ins." }],
      ["obscure", "Obscure account passwords", choice(yesNo, { value: "false", help: "Uses reversible Base64 encoding, not encryption. Domain and Wi-Fi passwords remain readable." })]
    ]],
    ["welcome", "Windows welcome", [
      ["hide-license", "Skip license terms page", choice(yesNo)], ["hide-oem", "Skip manufacturer registration", choice(yesNo)],
      ["hide-online", "Skip Microsoft account screens", choice(yesNo, { help: "Configure a local account. Support depends on the Windows image." })],
      ["hide-wireless", "Skip Wi-Fi setup screen", choice(yesNo)],
      ["protection", "Use Express settings", choice([["", "Windows default"], ["1", "Y"], ["3", "N"]], { toggle: true, help: "Y enables Express settings, including data sharing, location and advertising personalization, and some web protections. N turns Express settings off; it does not disable all Windows telemetry. Available settings depend on the Windows version." })]
    ]],
    ["network", "Workgroup & domain", [
      ["join-mode", "Membership", choice([["", "Windows default"], ["workgroup", "Workgroup"], ["domain", "Active Directory domain"]])],
      ["join-name", "Workgroup or domain name", { required: true, when: () => Boolean(value("join-mode")) }],
      ...[["join-user", "Domain join user", "text"], ["join-password", "Domain join password", "password"], ["join-domain", "Credentials domain", "text"], ["join-ou", "Computer organizational unit", "text"]].map(([id, label, type]) => [id, label, { type, required: ["join-user", "join-password"].includes(id), when: () => value("join-mode") === "domain" }])
    ]],
    ["wifi", "Initial Wi-Fi connection", [
      ["wifi-mode", "Wireless connection", choice([["", "Choose during Setup"], ["personal", "Network credentials"], ["xml", "Existing WLAN profile XML"]])],
      ["ssid", "Network name (SSID)", { required: true, when: () => value("wifi-mode") === "personal" }],
      ["wifi-auth", "Security", choice(["WPA2PSK", "WPA3SAE", "open"], { value: "WPA2PSK", when: () => value("wifi-mode") === "personal" })],
      ["wifi-password", "Wi-Fi password", { type: "password", required: true, when: () => value("wifi-mode") === "personal" && value("wifi-auth") !== "open" }],
      ["wifi-hidden", "Hidden network", choice(yesNo, { value: "false", when: () => value("wifi-mode") === "personal" })],
      ["wifi-xml", "WLAN profile XML", { type: "textarea", required: true, wide: true, when: () => value("wifi-mode") === "xml", help: "Enterprise profiles may require certificates supplied separately." }]
    ]],
    ["drivers", "Installation drivers", [["drivers", "Driver folders", { type: "textarea", wide: true, placeholder: "D:\\Drivers", help: "One full folder path per line, available to Windows PE. Include only drivers needed to boot or install Windows." }]]],
    ["virtual-machine", "Virtual machine", [
      ["guest-tools", "Guest integration software", choice([["", "None"], ["virtualbox", "VirtualBox Guest Additions"]], { help: "Add the Guest Additions ISO through VirtualBox. Attach it to a second virtual optical drive and leave it attached through the first Windows sign-in. Orbital does not include the ISO, and can typically be found included in your VirtualBox installation. Installation runs automatically at sign-in and restarts Windows afterward. If installation fails, it retries at the next sign-in. Logs are kept in ProgramData\\Orbital\\Logs unless disabled." })]
    ]],
    ["commands", "Setup commands", []],
    ["scripts", "Embedded installation scripts", [
      ["payload", "Setup files folder", { value: "C:\\ProgramData\\Orbital\\Setup", maxLength: 110, help: "Dedicated folder ending in \\Orbital\\Setup. Files are restricted to administrators and SYSTEM." }],
      ["logging", "Keep setup logs", choice(yesNo, { help: "Logs are kept under ProgramData\\Orbital\\Logs. Reset also keeps logs." })],
      ["windows", "Script windows", choice([["hidden", "Hidden"], ["visible", "Visible"]], { value: "hidden", help: "Never use interactive prompts in hidden scripts or restart the PC directly." })]
    ]],
    ["advanced", "Advanced XML", [
      ["mode", "Answer file source", choice([["guided", "Installation settings"], ["xml", "Complete XML"]], { value: "guided" })],
      ["xml", "Complete answer file", { type: "textarea", wide: true, required: true, when: () => value("mode") === "xml", help: "This XML is used as the answer file. Copy settings to XML replaces it with the current installation settings." }]
    ]]
  ];
  const rowFields = {
    accounts: [["name", "User name", { required: true }], ["display", "Display name", {}], ["group", "Account group", choice(["Administrators", "Users", "Custom"], { value: "Administrators" })], ["groups", "Custom groups", { help: "Separate existing Windows group names with semicolons." }], ["password-mode", "Password", choice([["", "Do not set"], ["custom", "Set password"], ["blank", "Explicitly empty"]])], ["password", "Account password", { type: "password" }], ["description", "Description", { wide: true }]],
    commands: [["pass", "Run during", choice(["specialize", "windowsPE", "auditUser"], { value: "specialize" })], ["description", "Description", { maxLength: 259 }], ["path", "Command line", { required: true, wide: true }], ["reboot", "Restart after command", choice([["", "Windows default"], ["Never", "Never"], ["OnRequest", "If requested"], ["Always", "Always"]])]],
    scripts: [["type", "Script type", choice(["ps1", "cmd", "reg"], { value: "ps1" })], ["label", "Description", {}], ["content", "Installation script", { type: "textarea", required: true, wide: true, help: "Runs as SYSTEM during device setup. Supply any external programs separately. Do not restart Windows from the script." }]]
  };
  const controls = new Map(), sections = new Map(), lists = new Map(), pickers = new WeakMap();
  let rowId = 0, initialized = false;
  const read = control => control.type === "password" || control.dataset.key === "ssid" ? control.value : control.value.trim();
  const value = id => controls.get(id)?.disabled ? "" : read(controls.get(id));
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
    const wrapper = element(config.choices || config.suggestions ? "div" : "label", `answer-field${config.wide || config.toggle ? " answer-wide" : ""}${config.toggle ? " answer-binary" : ""}`), title = element("span", "", label);
    if (!config.toggle && !config.required && !config.value && (!config.choices || config.choices.some(([key]) => key === ""))) title.append(element("small", "answer-optional", "(optional)"));
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
    if (config.suggestions) {
      const picker = element("select"); picker.id = `${input.id}-picker`; input.setAttribute("aria-label", `${label} (custom)`);
      for (const [key, text] of [...config.suggestions, ["@custom", "Custom…"]]) picker.add(new Option(text, key));
      picker.addEventListener("input", () => { input.value = picker.value === "@custom" ? "" : picker.value; input.hidden = picker.value !== "@custom"; if (!input.hidden) input.focus(); });
      input.before(picker);
    }
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
        const list = element("div", "answer-wide answer-list"), add = actionButton("answer-add answer-wide", `Add ${id === "accounts" ? "local account" : id === "commands" ? "setup command" : "installation script"}`);
        add.addEventListener("click", () => addRow(id)); lists.set(id, list); grid.append(list, add);
      }
      section.append(grid); sections.set(id, section); byId("answer-groups").append(section);
    }
    const copy = actionButton("answer-add answer-wide", "Copy settings to XML");
    copy.addEventListener("click", () => attempt(() => { const xml = build("guided"); controls.get("xml").value = xml; controls.get("mode").value = "xml"; update(); status("Installation settings copied to XML."); }));
    const copyNote = element("p", "note answer-wide", "Copying settings replaces any XML in the complete answer file field.");
    controls.get("xml").closest(".answer-field").before(copy, copyNote);
    byId("answer-form").addEventListener("input", update);
    byId("answer-form").addEventListener("change", update);
    byId("answer-form").addEventListener("reset", () => { for (const list of lists.values()) list.replaceChildren(); byId("answer-search").value = ""; requestAnimationFrame(update); });
    update();
  }
  function update() {
    closeChoicePicker();
    const windows10 = value("edition").startsWith("Windows 10 "), home = / Home(?: N)?$/.test(value("edition"));
    for (const [id, blocked] of [["architecture", windows10 ? "" : "x86"], ["layout", windows10 ? "" : "mbr"], ["join-mode", home ? "domain" : ""]]) {
      const control = controls.get(id);
      for (const option of control.options) option.hidden = option.disabled = Boolean(blocked && option.value === blocked);
      if (control.selectedOptions[0]?.disabled) control.value = control.options[0].value;
    }
    const logon = controls.get("autologon"), selected = logon.value;
    logon.replaceChildren(new Option("Disabled", ""), ...rows("accounts").map(row => rowValue(row, "name")).filter(Boolean).map(name => new Option(name, name)));
    if (value("builtin-admin") === "true") logon.add(new Option("Built-in Administrator", "@Administrator"));
    logon.value = [...logon.options].some(option => option.value === selected) ? selected : "";
    for (const [, , fields] of groups) for (const [id, , config] of fields) {
      const control = controls.get(id), visible = !config.when || config.when();
      control.closest(".answer-field").hidden = control.disabled = !visible;
      if (config.suggestions) {
        const picker = control.closest(".answer-field").querySelector("select"), custom = picker.value === "@custom" && !control.hidden;
        picker.value = custom || !config.suggestions.some(([key]) => key === control.value) ? "@custom" : control.value;
        control.hidden = picker.value !== "@custom";
      }
      if (config.toggle) {
        for (const button of control.parentElement.querySelectorAll("[data-value]")) button.setAttribute("aria-pressed", String(button.dataset.value === control.value));
        control.parentElement.querySelector(".answer-forget").disabled = !control.value;
      }
    }
    for (const row of rows("accounts")) for (const [key, visible] of [["groups", rowValue(row, "group") === "Custom"], ["password", rowValue(row, "password-mode") === "custom"]]) {
      const control = rowControl(row, key); control.closest(".answer-field").hidden = control.disabled = !visible; control.required = visible;
      control.closest(".answer-field").querySelector(".answer-optional")?.toggleAttribute("hidden", visible);
    }
    for (const [kind, key, label] of [["accounts", "name", "Local account"], ["commands", "description", "Setup command"], ["scripts", "label", "Installation script"]]) rows(kind).forEach((row, index) => {
      const title = rowValue(row, key) || `${label} ${index + 1}`;
      row.querySelector(".answer-row-title").textContent = title;
      row.querySelector(".answer-row-remove").setAttribute("aria-label", `Remove ${title}`);
    });
    for (const picker of byId("answer-form").querySelectorAll("select")) {
      const control = pickers.get(picker);
      control.load(picker.value);
      control.setDisabled(picker.disabled);
    }
    const term = byId("answer-search").value.trim().toLowerCase(), xmlMode = value("mode") === "xml";
    for (const [id, section] of sections) { section.hidden = Boolean(term && !section.textContent.toLowerCase().includes(term)); if (term && !section.hidden || xmlMode && id === "advanced") section.open = true; }
    if (term) for (const kind of Object.keys(rowFields)) for (const row of rows(kind)) if (row.textContent.toLowerCase().includes(term)) row.open = true;
    renderIssues(validate());
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
      if (rowValue(row, "group") === "Custom" && rowValue(row, "groups") && rowValue(row, "groups").split(";").some(name => !name.trim())) add(rowControl(row, "groups"), "Separate non-empty Windows group names with semicolons.");
    }
    if (value("autologon") && value("autologon") !== "@Administrator") {
      const row = rows("accounts").find(row => rowValue(row, "name") === value("autologon"));
      if (!rowValue(row, "password-mode")) add(["autologon", rowControl(row, "password-mode")], "Automatic sign-in requires a configured or explicitly empty password.");
    }
    if (value("wifi-mode") === "personal") {
      if (utf8.encode(value("ssid")).length > 32) add("ssid", "The network name must be at most 32 UTF-8 bytes.");
      const password = value("wifi-password");
      if (password && value("wifi-auth") !== "open" && !(password.length >= 8 && password.length <= 63 || value("wifi-auth") === "WPA2PSK" && /^[0-9a-f]{64}$/i.test(password))) add("wifi-password", "Use 8–63 characters, or a 64-digit hexadecimal WPA2 key.");
    }
    if (value("wifi-mode") === "xml") try { parse(value("wifi-xml"), "WLANProfile", "http://www.microsoft.com/networking/WLAN/profile/v1"); } catch (error) { add("wifi-xml", error.message); }
    if (nonEmptyLines(value("drivers")).some(path => path.length > 259 || !/^(?:[a-z]:\\|\\\\|%[a-z_]+%\\)/i.test(path))) add("drivers", "Use one full driver-folder path per line, at most 259 characters each.");
    if (!/^[a-z]:\\(?:[^\\]+\\)*Orbital\\Setup$/i.test(value("payload")) || /["<>|?*]/.test(value("payload")) || value("payload").split("\\").some(part => [".", ".."].includes(part))) add("payload", "Use a dedicated full path ending in \\Orbital\\Setup.");
    const commands = rows("commands");
    for (const row of commands) if (rowValue(row, "path").length > (rowValue(row, "pass") === "windowsPE" ? 1024 : 259)) add(rowControl(row, "path"), "This command is too long for its configuration pass. Use an embedded script.");
    const setupScripts = Boolean(value("wifi-mode") || value("builtin-admin") || value("guest-tools") || rows("scripts").length || value("disk-mode") === "erase" && value("recovery") === "false");
    for (const pass of ["windowsPE", "specialize", "auditUser"]) {
      const selected = commands.filter(row => rowValue(row, "pass") === pass);
      const generated = pass === "specialize" && setupScripts ? 2 : pass === "windowsPE" && value("disk-mode") === "erase" ? 1 : 0;
      if (selected.length + generated > 500) add(rowControl(selected[0], "pass"), `${pass} supports up to 500 commands, including generated setup steps.`);
    }
    return issues;
  }
  function renderIssues(issues, scroll = false) {
    for (const field of byId("answer-form").querySelectorAll(".answer-invalid")) { field.classList.remove("answer-invalid"); field.querySelector(".answer-field-error")?.remove(); for (const control of field.querySelectorAll("[aria-invalid]")) { control.removeAttribute("aria-invalid"); control.removeAttribute("aria-describedby"); } }
    const counts = new Map(), summary = byId("answer-errors"), list = element("ul");
    summary.replaceChildren(element("h2", "", `${issues.length} ${issues.length === 1 ? "issue prevents" : "issues prevent"} creating this answer file`), list); summary.hidden = !issues.length;
    for (const issue of issues) {
      const item = element("li"), button = actionButton("", issue.message);
      button.addEventListener("click", () => { byId("answer-search").value = ""; for (const section of sections.values()) section.hidden = false; const control = focusControl(issue.controls[0]); for (let section = control.closest("details"); section; section = section.parentElement.closest("details")) section.open = true; control.scrollIntoView({ block: "center" }); control.focus({ preventScroll: true }); });
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
    return { format: "orbital-answer", version: 1, values: Object.fromEntries([...controls].map(([key, control]) => [key, read(control)])), ...Object.fromEntries(Object.keys(rowFields).map(kind => [kind, rows(kind).map(row => Object.fromEntries(rowFields[kind].map(([key]) => [key, rowValue(row, key)])))])) };
  }
  function restore(profile) {
    if (!profile || profile.format !== "orbital-answer" || profile.version !== 1 || !profile.values || Array.isArray(profile.values)) throw new Error("Choose installation settings exported by this version of Orbital.");
    const check = (data, fields) => {
      if (!data || typeof data !== "object" || Array.isArray(data)) throw new Error("Invalid settings entry.");
      for (const [key, text] of Object.entries(data)) {
        const config = fields.find(([id]) => id === key)?.[2];
        if (!config || typeof text !== "string" || config.choices && !config.choices.some(([value]) => value === text) && key !== "autologon") throw new Error(`Unsupported setting or value: ${key}.`);
      }
    };
    check(profile.values, groups.flatMap(([, , fields]) => fields));
    for (const [kind, fields] of Object.entries(rowFields)) {
      if (!Array.isArray(profile[kind]) || profile[kind].length > 500) throw new Error(`Invalid ${kind} entries.`);
      for (const row of profile[kind]) check(row, fields);
    }
    const logon = profile.values.autologon;
    if (logon && !(logon === "@Administrator" && profile.values["builtin-admin"] === "true") && !profile.accounts.some(row => row.name === logon)) throw new Error("Automatic sign-in must refer to an enabled account in these settings.");
    byId("answer-form").reset();
    for (const [key, text] of Object.entries(profile.values)) if (key !== "autologon") controls.get(key).value = text;
    for (const kind of Object.keys(rowFields)) for (const row of profile[kind]) addRow(kind, row);
    update(); controls.get("autologon").value = logon || ""; controls.get("disk-confirm").value = ""; update();
  }
  function base64(text, utf16 = false) {
    const bytes = utf16 ? new Uint8Array(text.length * 2) : utf8.encode(text);
    if (utf16) { const view = new DataView(bytes.buffer); for (let i = 0; i < text.length; i++) view.setUint16(i * 2, text.charCodeAt(i), true); }
    let binary = ""; for (let i = 0; i < bytes.length; i += 8192) binary += String.fromCharCode(...bytes.subarray(i, i + 8192));
    return btoa(binary);
  }
  function fingerprint(root) {
    const copy = root.cloneNode(true); for (const node of [...copy.children]) if (node.namespaceURI === extraNs && node.localName === "Configuration") node.remove();
    let hash = 2166136261; for (const character of new XMLSerializer().serializeToString(copy)) hash = Math.imul(hash ^ character.charCodeAt(0), 16777619);
    return (hash >>> 0).toString(16);
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
  function virtualBoxScripts(folder, hidden, logging) {
    const task = "Orbital Install VirtualBox Guest Additions";
    return {
      install: `$ErrorActionPreference = 'Stop'
${logging ? `New-Item -ItemType Directory -Path "$env:ProgramData\\Orbital\\Logs" -Force | Out-Null
Start-Transcript -Path "$env:ProgramData\\Orbital\\Logs\\virtualbox.log" -Append | Out-Null` : ""}
try {
  for ($attempt = 0; (Get-ItemProperty -LiteralPath 'HKLM:\\SYSTEM\\Setup').SystemSetupInProgress; $attempt++) {
    if ($attempt -eq 60) { throw 'Windows Setup is still running. Installation will retry at the next sign-in.' }
    Start-Sleep -Seconds 5
  }
  for ($attempt = 0; $attempt -lt 12; $attempt++) {
    $installers = @(Get-CimInstance Win32_CDROMDrive | Where-Object { $_.Drive } | ForEach-Object { Join-Path ($_.Drive + '\\') 'VBoxWindowsAdditions.exe' } | Where-Object { Test-Path -LiteralPath $_ -PathType Leaf })
    if ($installers.Count -gt 0) { break }
    if ($attempt -lt 11) { Start-Sleep -Seconds 5 }
  }
  if ($installers.Count -ne 1) { throw 'Attach exactly one VirtualBox Guest Additions ISO. Installation will retry at the next sign-in.' }
  $media = Split-Path -Parent $installers[0]
  $certTool = Join-Path $media 'cert\\VBoxCertUtil.exe'
  $certificates = Join-Path $media 'cert\\vbox*.cer'
  if (!(Test-Path -LiteralPath $certTool -PathType Leaf) -or !(Test-Path -Path $certificates -PathType Leaf)) { throw 'Guest Additions signing certificates were not found on the ISO.' }
  & $certTool add-trusted-publisher $certificates --root $certificates
  if ($LASTEXITCODE -ne 0) { throw "Guest Additions certificate setup failed with exit code $LASTEXITCODE." }
  Write-Output "Installer: $($installers[0])"
  $result = Start-Process -FilePath $installers[0] -WorkingDirectory $media -ArgumentList '/S' -Wait -PassThru
  Write-Output "Installer exit code: $($result.ExitCode)"
  if ($result.ExitCode -notin @(0, 3010)) { throw "VirtualBox Guest Additions installer failed with exit code $($result.ExitCode)." }
  if (!(Get-Service -Name VBoxService -ErrorAction SilentlyContinue)) { throw 'The installer returned success, but the VirtualBox Guest Additions service was not found.' }
  Set-Service -Name VBoxService -StartupType Automatic
  Start-Service -Name VBoxService
  $service = Get-Service -Name VBoxService
  $service.WaitForStatus([System.ServiceProcess.ServiceControllerStatus]::Running, [TimeSpan]::FromSeconds(30))
  $service.Refresh()
  if ($service.Status -ne 'Running') { throw 'VirtualBox Guest Additions service did not remain running. Check the Windows System event log.' }
  Write-Output "VirtualBox Guest Additions service: $($service.Status); startup: Automatic"
  Write-Output 'VirtualBox Guest Additions is installed and its service is running. Restart Windows to finish driver installation.'
  Unregister-ScheduledTask -TaskName ${quote(task)} -Confirm:$false
} catch {
  Write-Warning $_.Exception.Message
  exit 1
} finally {
${logging ? "  Stop-Transcript -ErrorAction SilentlyContinue | Out-Null" : ""}
}`,
      register: `$action = New-ScheduledTaskAction -Execute "$env:WINDIR\\System32\\WindowsPowerShell\\v1.0\\powershell.exe" -Argument ${quote(`-NoProfile -ExecutionPolicy Bypass${hidden} -File "${folder}\\virtualbox.ps1"`)}
$trigger = New-ScheduledTaskTrigger -AtLogOn
$principal = New-ScheduledTaskPrincipal -UserId 'S-1-5-18' -LogonType ServiceAccount -RunLevel Highest
$settings = New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -StartWhenAvailable -MultipleInstances IgnoreNew -ExecutionTimeLimit (New-TimeSpan -Minutes 30)
Register-ScheduledTask -TaskName ${quote(task)} -Action $action -Trigger $trigger -Principal $principal -Settings $settings -Force | Out-Null`
    };
  }
  function build(mode = value("mode")) {
    const issues = validate(mode); renderIssues(issues);
    if (issues.length) throw Object.assign(new Error(`Fix ${issues.length} ${issues.length === 1 ? "issue" : "issues"} listed above.`), { issues });
    if (mode === "xml") return serialize(parse(value("xml")));
    const xml = document.implementation.createDocument(ns, "unattend"), root = xml.documentElement;
    root.setAttributeNS("http://www.w3.org/2000/xmlns/", "xmlns:wcm", wcm);
    const names = { setup: "Microsoft-Windows-Setup", shell: "Microsoft-Windows-Shell-Setup", locale: "Microsoft-Windows-International-Core", winpe: "Microsoft-Windows-International-Core-WinPE", join: "Microsoft-Windows-UnattendedJoin", deployment: "Microsoft-Windows-Deployment" };
    const architecture = value("architecture") === "all" ? "amd64" : value("architecture");
    function component(pass, name) {
      let settings = [...root.children].find(child => child.getAttribute("pass") === pass);
      if (!settings) { settings = node(root, "settings", undefined, true); settings.removeAttributeNS(wcm, "action"); settings.setAttribute("pass", pass); }
      name = names[name] || name;
      let result = [...settings.children].find(child => child.getAttribute("name") === name);
      if (!result) { result = xml.createElementNS(ns, "component"); for (const [key, text] of Object.entries({ name, processorArchitecture: architecture, publicKeyToken: "31bf3856ad364e35", language: "neutral", versionScope: "nonSxS" })) result.setAttribute(key, text); settings.append(result); }
      return result;
    }
    const setting = (pass, name, path, text) => node(component(pass, name), path, text);
    for (const [id, pass, name, path] of [
      ["product-key", "windowsPE", "setup", "UserData/ProductKey/Key"], ["accept-eula", "windowsPE", "setup", "UserData/AcceptEula"], ["dynamic-update", "windowsPE", "setup", "DynamicUpdate/Enable"], ["compact", "windowsPE", "setup", "ImageInstall/OSImage/Compact"],
      ["setup-language", "windowsPE", "winpe", "SetupUILanguage/UILanguage"], ["timezone", "oobeSystem", "shell", "TimeZone"], ["computer", "specialize", "shell", "ComputerName"], ["owner", "specialize", "shell", "RegisteredOwner"], ["organization", "specialize", "shell", "RegisteredOrganization"], ["activation-key", "specialize", "shell", "ProductKey"],
      ["hide-license", "oobeSystem", "shell", "OOBE/HideEULAPage"], ["hide-oem", "oobeSystem", "shell", "OOBE/HideOEMRegistrationScreen"], ["hide-online", "oobeSystem", "shell", "OOBE/HideOnlineAccountScreens"], ["hide-wireless", "oobeSystem", "shell", "OOBE/HideWirelessSetupInOOBE"], ["protection", "oobeSystem", "shell", "OOBE/ProtectYourPC"]
    ]) if (value(id)) setting(pass, name, path, value(id));
    setting("windowsPE", "setup", "UserData/ProductKey/WillShowUI", "Never");
    for (const [id, path] of [["language", "UILanguage"], ["user-locale", "UserLocale"], ["keyboard", "InputLocale"], ["fallback", "UILanguageFallback"], ["system-locale", "SystemLocale"]]) if (value(id)) for (const [pass, name] of [["windowsPE", "winpe"], ["oobeSystem", "locale"]]) setting(pass, name, path, value(id));
    if (value("edition")) { const metadata = node(component("windowsPE", "setup"), "ImageInstall/OSImage/InstallFrom/MetaData", undefined, true); node(metadata, "Key", "/IMAGE/NAME"); node(metadata, "Value", value("edition")); }
    if (value("encryption")) setting("specialize", "Microsoft-Windows-SecureStartup-FilterDriver", "PreventDeviceEncryption", String(value("encryption") === "false"));
    const password = (parent, path, text) => { node(parent, `${path}/Value`, value("obscure") === "true" ? base64(text + path, true) : text); node(parent, `${path}/PlainText`, value("obscure") === "true" ? "false" : "true"); };
    for (const row of rows("accounts")) {
      const account = node(component("oobeSystem", "shell"), "UserAccounts/LocalAccounts/LocalAccount", undefined, true);
      for (const [path, text] of [["Name", rowValue(row, "name")], ["Group", rowValue(row, "group") === "Custom" ? rowValue(row, "groups") : rowValue(row, "group")], ["DisplayName", rowValue(row, "display")], ["Description", rowValue(row, "description")]]) if (text) node(account, path, text);
      if (rowValue(row, "password-mode")) password(account, "Password", rowValue(row, "password-mode") === "blank" ? "" : rowValue(row, "password"));
    }
    if (value("builtin-admin") === "true") password(node(component("oobeSystem", "shell"), "UserAccounts"), "AdministratorPassword", value("builtin-password"));
    if (value("autologon")) {
      const builtin = value("autologon") === "@Administrator", row = rows("accounts").find(row => rowValue(row, "name") === value("autologon")), logon = node(component("oobeSystem", "shell"), "AutoLogon");
      for (const [path, text] of [["Enabled", "true"], ["Username", builtin ? value("builtin-name") : value("autologon")], ["LogonCount", Number(value("logon-count"))]]) node(logon, path, text);
      password(logon, "Password", builtin ? value("builtin-password") : rowValue(row, "password-mode") === "blank" ? "" : rowValue(row, "password"));
    }
    if (value("join-mode")) {
      setting("specialize", "join", `Identification/${value("join-mode") === "domain" ? "JoinDomain" : "JoinWorkgroup"}`, value("join-name"));
      if (value("join-mode") === "domain") for (const [id, path] of [["join-user", "Credentials/Username"], ["join-password", "Credentials/Password"], ["join-domain", "Credentials/Domain"], ["join-ou", "MachineObjectOU"]]) if (value(id)) setting("specialize", "join", `Identification/${path}`, value(id));
    }
    const orders = {}, commands = [], scripts = [], files = new Map(), folder = value("payload"), hidden = value("windows") === "hidden" ? " -WindowStyle Hidden" : "";
    const embed = (name, content) => { files.set(name, content); return `${folder}\\${name}`; };
    const script = (label, content) => scripts.push(`try {\nWrite-Output ${quote(label)}\n& {\n${content}\n}\n} catch { Write-Warning $_.Exception.Message }`);
    if (value("disk-mode") === "erase") {
      const disk = Number(value("disk")), gpt = value("layout") === "gpt", recovery = value("recovery") !== "false";
      const lines = [`select disk ${disk}`, "clean", `convert ${gpt ? "gpt" : "mbr"}`, `create partition ${gpt ? "efi" : "primary"} size=${Number(value("system-size"))}`, `format quick fs=${gpt ? "fat32" : "ntfs"} label=System`, ...(gpt ? ["create partition msr size=16"] : ["active"]), "create partition primary", "format quick fs=ntfs label=Windows"];
      if (recovery) lines.push(`shrink desired=${Number(value("recovery-size"))} minimum=${Number(value("recovery-size"))}`, "create partition primary", "format quick fs=ntfs label=Recovery", `set id=${gpt ? "de94bba4-06d1-4d40-a16a-bfd50179d6ac" : "27"}`, ...(gpt ? ["gpt attributes=0x8000000000000001"] : []));
      lines.push("exit");
      commands.push(["windowsPE", `cmd.exe /d /c "wpeutil.exe UpdateBootInfo && (reg.exe query HKLM\\SYSTEM\\CurrentControlSet\\Control /v PEFirmwareType | findstr.exe /c:0x${gpt ? "2" : "1"} >nul) && (${lines.map(line => `echo ${line}`).join("&")}) > \"%TEMP%\\OrbitalDisk.txt\" && diskpart.exe /s \"%TEMP%\\OrbitalDisk.txt\""`, "Verify firmware and partition the confirmed disk", ""]);
      for (const [path, text] of [["DiskID", disk], ["PartitionID", gpt ? 3 : 2]]) setting("windowsPE", "setup", `ImageInstall/OSImage/InstallTo/${path}`, text);
      setting("windowsPE", "setup", "ImageInstall/OSImage/WillShowUI", "OnError");
      if (!recovery) script("Disable Windows RE", "& reagentc.exe /disable\nif ($LASTEXITCODE -ne 0) { throw 'Could not disable Windows RE.' }");
    }
    if (value("builtin-admin")) script("Configure built-in Administrator", `$admin = Get-CimInstance Win32_UserAccount -Filter 'LocalAccount=True' | Where-Object { $_.SID -match '-500$' }\nif (!$admin) { throw 'Built-in Administrator was not found.' }\n& net.exe user $admin.Name /active:${value("builtin-admin") === "true" ? "yes" : "no"}\nif ($LASTEXITCODE -ne 0) { throw 'Could not configure built-in Administrator.' }`);
    if (value("guest-tools") === "virtualbox") {
      const guest = virtualBoxScripts(folder, hidden, value("logging") !== "false");
      embed("virtualbox.ps1", guest.install);
      script("Schedule VirtualBox Guest Additions installation", guest.register);
    }
    if (value("wifi-mode")) {
      let profile = value("wifi-xml");
      if (value("wifi-mode") === "personal") {
        const profileXml = document.implementation.createDocument("http://www.microsoft.com/networking/WLAN/profile/v1", "WLANProfile"), wlan = profileXml.documentElement;
        for (const [path, text] of [["name", value("ssid")], ["SSIDConfig/SSID/name", value("ssid")], ["SSIDConfig/nonBroadcast", value("wifi-hidden")], ["connectionType", "ESS"], ["connectionMode", "auto"], ["MSM/security/authEncryption/authentication", value("wifi-auth")], ["MSM/security/authEncryption/encryption", value("wifi-auth") === "open" ? "none" : "AES"], ["MSM/security/authEncryption/useOneX", "false"]]) node(wlan, path, text);
        if (value("wifi-auth") !== "open") for (const [path, text] of [["keyType", value("wifi-password").length === 64 ? "networkKey" : "passPhrase"], ["protected", "false"], ["keyMaterial", value("wifi-password")]]) node(wlan, `MSM/security/sharedKey/${path}`, text);
        profile = new XMLSerializer().serializeToString(profileXml);
      }
      const path = embed("wifi.xml", profile);
      script("Import initial Wi-Fi profile", `Set-Service WlanSvc -StartupType Automatic\nStart-Service WlanSvc\n& netsh.exe wlan add profile ${quote(`filename=${path}`)} user=all\nif ($LASTEXITCODE -ne 0) { throw 'Could not import the Wi-Fi profile.' }\nRemove-Item -LiteralPath ${quote(path)} -Force`);
    }
    for (const [index, path] of nonEmptyLines(value("drivers")).entries()) { const entry = node(component("windowsPE", "Microsoft-Windows-PnpCustomizationsWinPE"), "DriverPaths/PathAndCredentials", undefined, true); entry.setAttributeNS(wcm, "wcm:keyValue", String(index + 1)); node(entry, "Path", path); }
    for (const [index, row] of rows("scripts").entries()) {
      const type = rowValue(row, "type"), path = embed(`custom-${index + 1}.${type}`, type === "cmd" ? `@echo off\r\nchcp 65001 >nul\r\n${rowValue(row, "content")}` : rowValue(row, "content"));
      script(rowValue(row, "label") || `Installation script ${index + 1}`, `$global:LASTEXITCODE = 0\n${type === "ps1" ? `& ${quote(path)}` : type === "reg" ? `& reg.exe import ${quote(path)}` : `& $env:ComSpec /d /c ${quote('"' + path + '"')}`}\nif ($LASTEXITCODE -ne 0) { throw \"Script failed with exit code $LASTEXITCODE\" }`);
    }
    if (scripts.length) {
      const logging = value("logging") !== "false";
      const path = embed("setup.ps1", `$ErrorActionPreference = 'Stop'\n${logging ? 'New-Item -ItemType Directory -Path "$env:ProgramData\\Orbital\\Logs" -Force | Out-Null\nStart-Transcript -Path "$env:ProgramData\\Orbital\\Logs\\setup.log" -Append | Out-Null' : ""}\ntry {\n${scripts.join("\n")}\n} finally {\n${logging ? 'Stop-Transcript -ErrorAction SilentlyContinue | Out-Null' : ""}\n}`);
      commands.unshift(["specialize", `powershell.exe -NoProfile -ExecutionPolicy Bypass${hidden} -File "${path}"`, "Orbital installation steps", "Never"]);
    }
    if (files.size) {
      const extensions = xml.createElementNS(extraNs, "Extensions"), extract = xml.createElementNS(extraNs, "ExtractScript");
      extract.textContent = `param([xml]$Document)\n$ErrorActionPreference = 'Stop'\n$folder = ${quote(folder)}\nNew-Item -ItemType Directory -Path $folder -Force | Out-Null\n& icacls.exe $folder /inheritance:r /grant:r '*S-1-5-18:(OI)(CI)F' '*S-1-5-32-544:(OI)(CI)F' | Out-Null\nif ($LASTEXITCODE -ne 0) { throw 'Could not secure setup files.' }\nforeach ($payload in $Document.unattend.Extensions.File) {\n  $path = Join-Path $folder $payload.GetAttribute('name')\n  $text = [Text.Encoding]::UTF8.GetString([Convert]::FromBase64String($payload.InnerText))\n  $encoding = if ($path.EndsWith('.reg')) { [Text.Encoding]::Unicode } else { [Text.UTF8Encoding]::new(-not $path.EndsWith('.cmd')) }\n  [IO.File]::WriteAllText($path, $text, $encoding)\n}`;
      extensions.append(extract);
      for (const [name, text] of files) { const file = xml.createElementNS(extraNs, "File"); file.setAttribute("name", name); file.textContent = base64(text); extensions.append(file); }
      root.append(extensions);
      commands.unshift(["specialize", `powershell.exe -NoProfile -ExecutionPolicy Bypass${hidden} -Command "$x=[xml]::new();$x.Load($env:WINDIR+'\\Panther\\unattend.xml');& ([scriptblock]::Create($x.unattend.Extensions.ExtractScript)) $x"`, "Extract Orbital installation files", "Never"]);
    }
    for (const row of rows("commands")) commands.push([rowValue(row, "pass"), rowValue(row, "path"), rowValue(row, "description"), rowValue(row, "reboot")]);
    for (const [pass, path, description, reboot] of commands) {
      if (path.length > (pass === "windowsPE" ? 1024 : 259)) throw new Error("An installation command exceeds the Windows limit. Shorten the setup folder or use an embedded script.");
      const order = orders[pass] = (orders[pass] || 0) + 1;
      if (order > 500) throw new Error("Windows supports up to 500 commands per configuration pass.");
      const command = node(component(pass, pass === "windowsPE" ? "setup" : "deployment"), "RunSynchronous/RunSynchronousCommand", undefined, true);
      node(command, "Order", order); node(command, "Path", path); if (description) node(command, "Description", description); if (reboot && pass !== "windowsPE") node(command, "WillReboot", reboot);
    }
    for (const pass of passes) { const settings = [...root.children].find(child => child.getAttribute("pass") === pass); if (settings) root.append(settings); }
    if (value("architecture") === "all") for (const settings of [...root.children].filter(child => child.localName === "settings")) for (const original of [...settings.children]) for (const arch of value("edition").startsWith("Windows 10 ") ? ["arm64", "x86"] : ["arm64"]) { const copy = original.cloneNode(true); copy.setAttribute("processorArchitecture", arch); settings.append(copy); }
    for (const child of [...root.children]) if (child.namespaceURI !== ns) root.append(child);
    const metadata = xml.createElementNS(extraNs, "Configuration"), profile = capture(); profile.values.mode = "guided"; delete profile.values.xml; metadata.setAttribute("hash", fingerprint(root)); metadata.textContent = base64(JSON.stringify(profile)); root.append(metadata);
    return serialize(xml);
  }
  function show(open) {
    closeChoicePicker(); initialize(); byId("answer-page").hidden = !open; byId("inject-panel").hidden = open; document.querySelector(".top-bar").hidden = open;
    if (typeof stopScrolling === "function") stopScrolling(); window.scrollTo(0, 0); (open ? byId("answer-search") : byId("inject-xml")).focus({ preventScroll: true });
  }
  function importText(source) {
    initialize();
    if (utf8.encode(source).length > 1048576) throw new Error("Choose a file under 1 MiB.");
    if (source.trimStart().startsWith("{")) restore(JSON.parse(source));
    else {
      const xml = parse(source), metadata = [...xml.documentElement.children].find(child => child.namespaceURI === extraNs && child.localName === "Configuration");
      if (metadata && metadata.getAttribute("hash") === fingerprint(xml.documentElement)) restore(JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode(Uint8Array.from(atob(metadata.textContent), character => character.charCodeAt(0)))));
      else { controls.get("xml").value = serialize(xml); controls.get("mode").value = "xml"; update(); }
    }
    status("Settings imported. Confirm disk erasure again if applicable.");
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
  async function choose() {
    const dialog = byId("answer-source"); if (dialog.open) return;
    while (await showDialog(dialog) === "continue") {
      if (dialog.querySelector("input:checked").value === "configure") { show(true); return; }
      if (invoke ? await changeInjectFile("xml") : await upload(false)) return;
    }
  }
  byId("answer-back").addEventListener("click", () => { show(false); if (!injectFiles.xml) choose(); });
  byId("answer-search").addEventListener("input", update);
  byId("answer-collapse").addEventListener("click", () => { for (const section of byId("answer-groups").querySelectorAll("details")) section.open = false; });
  byId("answer-import").addEventListener("click", () => attempt(async () => {
    if (invoke) { const text = await invoke("answer_document", { content: null, kind: "open" }); if (text !== null) importText(text); }
    else await upload(true);
  }));
  for (const button of document.querySelectorAll("[data-answer-save]")) button.addEventListener("click", () => attempt(async () => {
    const kind = button.dataset.answerSave, content = kind === "json" ? JSON.stringify(capture(), null, 2) : build();
    if (utf8.encode(content).length > 1048576) throw new Error("Reduce the configuration to less than 1 MiB.");
    if (invoke) { if (await invoke("answer_document", { content, kind }) !== null) status("File saved."); }
    else { const link = element("a"), url = URL.createObjectURL(new Blob([content], { type: kind === "json" ? "application/json" : "application/xml" })); link.href = url; link.download = kind === "json" ? "orbital-answer.json" : "autounattend.xml"; link.click(); setTimeout(() => URL.revokeObjectURL(url), 1000); status("File downloaded."); }
  }));
  byId("answer-form").addEventListener("submit", event => { event.preventDefault(); attempt(async () => { const xml = build(); const file = invoke ? await invoke("set_answer_file", { xml }) : { name: "Orbital answer file.xml", xml }; setInjectFile("xml", file); show(false); }); });
  return { choose };
})();
