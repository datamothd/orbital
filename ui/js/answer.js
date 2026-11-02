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
  const languages = [["", "Windows default"], ..."en-US en-GB ar-SA bg-BG cs-CZ da-DK de-DE el-GR es-ES es-MX et-EE fi-FI fr-CA fr-FR he-IL hr-HR hu-HU it-IT ja-JP ko-KR lt-LT lv-LV nb-NO nl-NL pl-PL pt-BR pt-PT ro-RO ru-RU sk-SK sl-SI sr-Latn-RS sv-SE th-TH tr-TR uk-UA zh-CN zh-TW".split(" ").map(code => [code, languageNames.of(code)])];
  const timezones = [["", "Windows default"], ...options(["UTC", "Pacific Standard Time", "Mountain Standard Time", "US Mountain Standard Time", "Central Standard Time", "Eastern Standard Time", "Atlantic Standard Time", "GMT Standard Time", "W. Europe Standard Time", "Romance Standard Time", "India Standard Time", "China Standard Time", "Tokyo Standard Time", "AUS Eastern Standard Time", "New Zealand Standard Time"])];
  const number = (value, min, max) => ({ type: "number", value, min, max, required: true });
  const setupUI = [["", "Windows default"], ["Always", "Always"], ["OnError", "Only on error"], ["Never", "Never"]];
  const imageKeys = [["/IMAGE/INDEX", "Image index"], ["/IMAGE/NAME", "Image name"], ["/IMAGE/DESCRIPTION", "Image description"]];
  const defaultEditions = ["Home", "Home N", "Pro", "Pro N", "Pro Education", "Pro for Workstations", "Education", "Enterprise", "Enterprise LTSC", "Enterprise N LTSC", "IoT Enterprise LTSC"].map(edition => `Windows 11 ${edition}`);
  let isoEditions = null;
  const advancedGroups = new Set(["drivers", "virtual-machine", "image-source", "partitions", "setup-behavior", "winpe-network", "winpe-logs", "winpe-pagefile", "winpe-display", "data-images", "deployment", "commands", "scripts", "advanced"]);
  const groups = [
    ["installation", "Installation", [
      ["edition", "Windows edition", choice([["", "Choose during Setup"], ...defaultEditions], { when: () => !value("image-selection"), help: "When an ISO is selected, this list shows its image names. Without an ISO, it shows common Windows 11 editions. Custom image selection can select by index, name, or description." })],
      ["architecture", "Processor architecture", choice([["amd64", "x64"], ["arm64", "ARM64"], ["all", "All matching"]], { value: "amd64" })],
      ["product-key", "Installation product key", { wide: true, placeholder: "XXXXX-XXXXX-XXXXX-XXXXX-XXXXX", help: "Leave blank to skip key entry. A supplied key must match the Windows image.", pattern: "[A-Za-z0-9]{5}(-[A-Za-z0-9]{5}){4}" }],
      ["product-key-ui", "Show product key screen", choice([["", "Windows default (on error)"], ["Always", "Always"], ["Never", "Never"]], { help: "Controls the product-key screen during Windows Setup. The Windows default shows it if an error occurs." })],
      ["accept-eula", "Accept Windows license terms", choice(yesNo)],
      ["dynamic-update", "Download updates during setup", choice(yesNo, { help: "Enabling setup updates may cause Windows installation to hang in virtual machines. Disabling them resolved a Windows 11 26H2 installation hang in VirtualBox. This workaround has not been tested on physical machines." })],
      ["compact", "Compress Windows system files", choice(yesNo, { when: () => value("deployment-mode") !== "wds", help: "Compact OS saves disk space and may affect performance." })],
      ["encryption", "Allow automatic device encryption", choice(yesNo, { help: "N prevents automatic encryption during setup. It does not decrypt an existing drive." })]
    ]],
    ["storage", "Storage", [
      ["disk-mode", "Installation disk", choice([["", "Choose during Setup"], ["erase", "Erase and partition a disk"], ["existing", "Use an existing partition"], ["available", "First suitable partition across all disks"]], { help: "Automatic selection starts at disk 0, partition 1 and searches all disks for enough space and no existing Windows installation. It does not create or format partitions. Existing-partition installation writes Windows to the selected destination; back up its files first." })],
      ["disk", "Target disk number", { ...number("0", 0, 999), when: () => ["erase", "existing"].includes(value("disk-mode")) }],
      ["partition", "Target partition number", { ...number("1", 1, 999), when: () => value("disk-mode") === "existing", help: "Partition numbers start at 1. The partition must already be prepared for Windows Setup or created by your custom disk layout." }],
      ["disk-confirm", "Confirm disk number to erase", { required: true, when: () => value("disk-mode") === "erase", help: "Every partition and file on this disk will be erased during the Windows installation. Type its number again." }],
      ["layout", "Boot layout", choice([["gpt", "UEFI / GPT"]], { value: "gpt", when: () => value("disk-mode") === "erase" })],
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
    ["advanced-settings", "Advanced settings", []],
    ["drivers", "Installation drivers", [["drivers", "Driver folders", { type: "textarea", wide: true, placeholder: "D:\\Drivers", help: "One full folder path per line, available to Windows PE. Include only drivers needed to boot or install Windows." }]]],
    ["virtual-machine", "Virtual machine", [
      ["guest-tools", "Guest integration software", choice([["", "None"], ["virtualbox", "VirtualBox Guest Additions"]], { help: "Add the Guest Additions ISO through VirtualBox. Attach it to a second virtual optical drive and leave it attached through the first Windows sign-in. Orbital does not include the ISO, and can typically be found included in your VirtualBox installation. Installation runs automatically at sign-in and restarts Windows afterward. If installation fails, it retries at the next sign-in. Logs are kept in ProgramData\\Orbital\\Logs unless disabled." })]
    ]],
    ["image-source", "Custom image selection", [
      ["image-selection", "Select Windows image by", choice([["", "Standard Windows edition list"], ...imageKeys], { when: () => value("deployment-mode") !== "wds" })],
      ["image-version", "Custom image Windows version", choice([["11", "Windows 11"]], { value: "11", when: () => Boolean(value("image-selection")), help: "Custom installation images must contain Windows 11. This does not change the image." })],
      ["image-index", "Windows image index", { ...number("1", 1, 2147483647), when: () => value("image-selection") === "/IMAGE/INDEX" }],
      ["image-value", "Windows image name or description", { required: true, maxLength: 259, when: () => ["/IMAGE/NAME", "/IMAGE/DESCRIPTION"].includes(value("image-selection")), help: "Enter the exact name or description recorded in the WIM image." }],
      ["image-location", "Windows image source", choice([["", "Installation media"], ["custom", "Custom WIM file"]], { when: () => value("deployment-mode") !== "wds" })],
      ["image-path", "Windows WIM path", { required: true, maxLength: 259, placeholder: "\\\\server\\share\\install.wim", when: () => value("image-location") === "custom", help: "Local, network or relative path accessible during Windows PE. Relative paths start where Setup is run. Use %configsetroot% with a configuration set. The file is not bundled into the ISO." }],
      ["image-auth", "Image share authentication", choice([["", "No credentials"], ["credentials", "Supply credentials"]], { when: () => value("image-location") === "custom" })],
      ...[["image-user", "Image share user", "text"], ["image-password", "Image share password", "password"], ["image-domain", "Image share domain", "text"]].map(([id, label, type]) => [id, label, { type, required: id !== "image-domain", when: () => value("image-auth") === "credentials", help: type === "password" ? "Stored as readable text in the answer file." : undefined }])
    ]],
    ["partitions", "Custom disk & partition layouts", [
      ["partition-mode", "Disk layout configuration", choice([["", "Use Storage settings"], ["custom", "Custom disk and partition operations"]], { help: "Advanced layouts must match the target computer's firmware and disk style. Wipes run first, then partition creation, then modification; each list keeps its order per disk. Choose the Windows destination under Storage. Do not combine this with Erase and partition a disk." })]
    ]],
    ["setup-behavior", "Setup behavior & diagnostics", [
      ["diagnostics", "Send installation diagnostics to Microsoft", choice(yesNo, { help: "Sends installation statistics and failures. This setting applies to Setup diagnostics." })],
      ["compliance-ui", "Show compatibility report", choice([["", "Windows default"], ["OnError", "Only on error"], ["Never", "Never"]], { help: "Controls the report screen, not hardware requirements. Never logs errors and stops installation when checks fail." })],
      ["dynamic-update-ui", "Show setup update screen", choice(setupUI)],
      ["image-ui", "Show image installation screen", choice(setupUI, { when: () => value("deployment-mode") !== "wds" })],
      ["disk-ui", "Show disk configuration screen", choice(setupUI)],
      ["disable-disk-encryption", "Block hardware encrypted-disk provisioning", choice(yesNo, { help: "Prevents Setup from activating hardware encryption on blank capable drives. Automatic Windows device encryption has a separate installation setting." })],
      ["configuration-set", "Use a Windows SIM configuration set", choice(yesNo, { help: "Use an existing configuration set created in Windows System Image Manager. Supply its files on the installation media separately." })],
      ["pe-restart", "After Windows PE completes", choice([["", "Windows default"], ["Restart", "Restart"], ["Shutdown", "Shut down"]], { help: "For standalone Windows PE workflows. Windows ignores this once image-based installation starts." })],
      ["setup-owner", "Setup user full name", { maxLength: 63 }],
      ["setup-organization", "Setup organization", { maxLength: 63 }]
    ]],
    ["winpe-network", "Windows PE networking & firewall", [
      ["pe-network", "Enable Windows PE networking", choice(yesNo, { help: "Applies to the temporary setup environment. N skips network initialization and ignores Windows PE network settings." })],
      ["pe-firewall", "Enable Windows PE firewall", choice(yesNo, { help: "Applies to the temporary setup environment. The installed Windows firewall is configured separately." })]
    ]],
    ["winpe-logs", "Windows PE log location", [
      ["pe-log-path", "Windows PE log directory", { wide: true, maxLength: 259, placeholder: "C:\\SetupLogs", help: "Full directory path on a local fixed disk available during Windows PE. Network shares and removable drives are unsupported. Regular Windows installation logs remain in their usual locations." }]
    ]],
    ["winpe-pagefile", "Windows PE page file", [
      ["pe-pagefile", "Windows PE paging", choice([["", "Windows default"], ["custom", "Custom page file"]], { help: "Only changes the page file used by Windows PE." })],
      ["pe-pagefile-path", "Windows PE page file path", { maxLength: 259, placeholder: "%WINDIR%\\pagefile.sys", when: () => value("pe-pagefile") === "custom", help: "Full file path on a fixed disk; Windows environment variables are allowed. Leave blank to use the default page file." }],
      ["pe-pagefile-size", "Windows PE page file size (MB)", { ...number("512", 1, 4294967295), when: () => value("pe-pagefile") === "custom" }]
    ]],
    ["winpe-display", "Windows PE display settings", [
      ["pe-display", "Setup display configuration", choice([["", "Windows default (recommended)"], ["custom", "Custom display settings"]], { help: "Unsupported values can leave a blank screen. UEFI generally ignores these settings unless a graphics driver is added to boot.wim. These settings only affect Windows PE." })],
      ["pe-width", "Horizontal resolution (pixels)", { ...number("1024", 1, 4294967295), when: () => value("pe-display") === "custom" }],
      ["pe-height", "Vertical resolution (pixels)", { ...number("768", 1, 4294967295), when: () => value("pe-display") === "custom" }],
      ["pe-depth", "Color depth", choice([["", "Windows default"], ["8", "8 bit"], ["16", "16 bit"], ["24", "24 bit"], ["32", "32 bit"]], { when: () => value("pe-display") === "custom" })],
      ["pe-refresh", "Refresh rate (Hz)", { type: "number", min: 1, max: 4294967295, placeholder: "60", when: () => value("pe-display") === "custom" }]
    ]],
    ["data-images", "Secondary data images", []],
    ["deployment", "Deployment & upgrades", [
      ["deployment-mode", "Installation workflow", choice([["", "Standard installation"], ["upgrade", "Upgrade existing Windows (upgrade settings only)"]], { help: "Upgrade exports only UpgradeData settings and must run from the existing Windows installation. Other configuration choices are excluded." })],
      ["upgrade-ui", "Show upgrade screen", choice(setupUI, { when: () => value("deployment-mode") === "upgrade" })],
      ["wds-login-ui", "Show WDS sign-in screen", choice(setupUI, { when: () => value("deployment-mode") === "wds" })],
      ["wds-image-ui", "Show WDS image selection screen", choice(setupUI, { when: () => value("deployment-mode") === "wds" })],
      ...[["wds-user", "WDS user", "text"], ["wds-password", "WDS password", "password"], ["wds-domain", "WDS domain", "text"], ["wds-image", "WDS image name", "text"], ["wds-group", "WDS image group", "text"], ["wds-file", "WDS image file name", "text"]].map(([id, label, type]) => [id, label, { type, maxLength: 259, when: () => value("deployment-mode") === "wds" }])
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
    commands: [["pass", "Run during", choice(["specialize", "windowsPE", "auditUser"], { value: "specialize" })], ["execution", "Command execution", choice([["", "Wait for completion (synchronous)"], ["async", "Start without waiting (Windows PE)"]], { help: "Asynchronous commands can overlap. Use synchronous commands when later steps depend on earlier ones." })], ["description", "Description", { maxLength: 259 }], ["path", "Command line", { required: true, wide: true }], ["reboot", "Restart after command", choice([["", "Windows default"], ["Never", "Never"], ["OnRequest", "If requested"], ["Always", "Always"]])], ["auth", "Windows PE command share authentication", choice([["", "No credentials"], ["credentials", "Supply credentials"]])], ["user", "Command share user", {}], ["password", "Command share password", { type: "password" }], ["domain", "Command share domain", {}]],
    "data-images": [["path", "Data WIM path", { required: true, maxLength: 259, wide: true, help: "Local, network or relative path available during Windows PE. Use %configsetroot% with a configuration set. Data images add files; do not overwrite Windows system files. Images are applied in the order listed after the operating system image." }], ["key", "Select data image by", choice(imageKeys, { value: "/IMAGE/INDEX" })], ["value", "Data image index, name or description", { value: "1", required: true, maxLength: 259 }], ["disk", "Data destination disk number", number("0", 0, 999)], ["partition", "Data destination partition number", number("1", 1, 999)], ["auth", "Data image share authentication", choice([["", "No credentials"], ["credentials", "Supply credentials"]])], ["user", "Data image share user", {}], ["password", "Data image share password", { type: "password" }], ["domain", "Data image share domain", {}]],
    partitions: [["action", "Disk operation", choice([["create", "Create partition"], ["modify", "Modify partition"], ["wipe", "Erase every partition on a disk"]], { value: "create" })], ["disk", "Operation disk number", number("0", 0, 999)], ["type", "Partition type", choice([["Primary", "Primary"], ["EFI", "EFI system (GPT)"], ["MSR", "Microsoft reserved (GPT)"], ["Extended", "Extended (MBR)"], ["Logical", "Logical (MBR)"]], { value: "Primary" })], ["extend", "Extend partition to fill available space", choice(yesNo, { toggle: false })], ["size", "Partition size (MB)", number("1024", 1, 4294967295)], ["partition", "Partition number to modify", number("1", 1, 999)], ["format", "Partition file system", choice([["", "Do not format"], ["NTFS", "Format as NTFS"], ["FAT32", "Format as FAT32"]])], ["label", "Partition label", { maxLength: 32 }], ["letter", "Partition drive letter", choice([["", "Windows default"], ...options("CDEFGHIJKLMNOPQRSTUVWXYZ".split(""))])], ["active", "Mark partition active (MBR)", choice(yesNo, { toggle: false })], ["type-id", "Custom partition type ID", { placeholder: "0x27 or de94bba4-06d1-4d40-a16a-bfd50179d6ac", pattern: "0[xX][0-9a-fA-F]{2}|[0-9a-fA-F]{8}(-[0-9a-fA-F]{4}){3}-[0-9a-fA-F]{12}", maxLength: 36 }], ["confirm", "Confirm disk number to erase or format", { help: "Wiping deletes every partition on this disk. Formatting erases the selected partition. Type the operation's disk number again. Imported settings always require a new confirmation." }]],
    scripts: [["type", "Script type", choice(["ps1", "cmd", "reg"], { value: "ps1" })], ["label", "Description", {}], ["content", "Installation script", { type: "textarea", required: true, wide: true, help: "Runs as SYSTEM during device setup. Supply any external programs separately. Do not restart Windows from the script." }]]
  };
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
    const wrapper = element(config.choices || config.suggestions ? "div" : "label", `answer-field${config.wide || config.toggle ? " answer-wide" : ""}${config.toggle ? " answer-binary" : ""}`), title = element("span", "answer-field-label"), caption = element("span", "", label);
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
      if (advancedGroups.has(id)) summary.insertAdjacentHTML("afterbegin", '<svg class="answer-branch" viewBox="0 0 20 14" width="20" height="14" fill="currentColor" shape-rendering="crispEdges" aria-hidden="true" focusable="false"><path d="M0 0h2v6h2v4h4v2h12v2H8v-2H4v-2H2V6H0z"/></svg>');
      count.append(element("span")); count.hidden = true; summary.append(count); section.append(summary);
      const grid = element("div", "answer-grid");
      for (const [key, label, config] of fields) controls.set(key, field(grid, key, label, config));
      if (rowFields[id]) {
        const list = element("div", "answer-wide answer-list"), add = actionButton("answer-add answer-wide", `Add ${id === "accounts" ? "local account" : id === "commands" ? "setup command" : id === "data-images" ? "data image" : id === "partitions" ? "disk operation" : "installation script"}`);
        add.addEventListener("click", () => addRow(id)); lists.set(id, list); grid.append(list, add);
      }
      section.append(grid); sections.set(id, section);
      if (id === "advanced-settings") grid.className = "answer-subgroups";
      (advancedGroups.has(id) ? sections.get("advanced-settings").querySelector(".answer-subgroups") : byId("answer-groups")).append(section);
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
    const home = !value("image-selection") && / Home(?: N)?$/.test(value("edition")), upgrade = value("deployment-mode") === "upgrade";
    for (const [id, blocked] of [["join-mode", home ? "domain" : ""]]) {
      const control = controls.get(id);
      for (const option of control.options) option.hidden = option.disabled = Boolean(blocked && option.value === blocked);
      if (control.selectedOptions[0]?.disabled) control.value = control.options[0].value;
    }
    const diskMode = controls.get("disk-mode"), automatic = [...diskMode.options].find(option => option.value === "available");
    automatic.hidden = automatic.disabled = value("deployment-mode") === "wds";
    if (diskMode.selectedOptions[0]?.disabled) diskMode.value = "";
    const logon = controls.get("autologon"), selected = logon.value;
    logon.replaceChildren(new Option("Disabled", ""), ...rows("accounts").map(row => rowValue(row, "name")).filter(Boolean).map(name => new Option(name, name)));
    if (controls.get("builtin-admin").value === "true") logon.add(new Option("Built-in Administrator", "@Administrator"));
    logon.value = [...logon.options].some(option => option.value === selected) ? selected : "";
    for (let pass = 0; pass < 2; pass++) for (const [, , fields] of groups) for (const [id, , config] of fields) {
      const control = controls.get(id), visible = (!upgrade || ["architecture", "deployment-mode", "upgrade-ui", "mode", "xml"].includes(id)) && (!config.when || config.when());
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
    for (const kind of ["commands", "data-images"]) for (const row of rows(kind)) {
      const pe = kind === "commands" && rowValue(row, "pass") === "windowsPE";
      if (kind === "commands") {
        const execution = rowControl(row, "execution"), async = [...execution.options].find(option => option.value === "async");
        async.hidden = async.disabled = !pe;
        if (!pe) execution.value = "";
      }
      const credentials = (kind === "data-images" || pe) && rowValue(row, "auth") === "credentials";
      for (const [key, visible, required] of [["auth", kind === "data-images" || pe, false], ["user", credentials, credentials], ["password", credentials, credentials], ["domain", credentials, false], ...(kind === "commands" ? [["reboot", !pe, false]] : [])]) {
        const control = rowControl(row, key);
        control.closest(".answer-field").hidden = !visible; control.disabled = !visible; control.required = required;
        control.closest(".answer-field").querySelector(".answer-optional")?.toggleAttribute("hidden", required);
      }
    }
    for (const row of rows("partitions")) {
      const action = rowValue(row, "action"), create = action === "create", modify = action === "modify", destructive = action === "wipe" || modify && Boolean(rowValue(row, "format"));
      for (const [key, visible, required] of [["type", create, false], ["extend", create || modify, false], ["size", create && rowValue(row, "extend") !== "true", true], ["partition", modify, true], ["format", modify, false], ["label", modify, false], ["letter", modify, false], ["active", modify, false], ["type-id", modify, false], ["confirm", destructive, true]]) {
        const control = rowControl(row, key); control.closest(".answer-field").hidden = control.disabled = !visible; control.required = visible && required;
        control.closest(".answer-field").querySelector(".answer-optional")?.toggleAttribute("hidden", control.required);
      }
    }
    for (const [kind, list] of lists) {
      const disabled = upgrade || kind === "data-images" && value("deployment-mode") === "wds" || kind === "partitions" && value("partition-mode") !== "custom";
      list.closest(".answer-grid").querySelector(".answer-add").disabled = disabled || list.children.length >= 500;
      list.hidden = disabled;
      for (const control of list.querySelectorAll("input, select, textarea")) if (disabled) control.disabled = true;
      else if (!control.closest(".answer-field").hidden) control.disabled = false;
    }
    for (const [kind, key, label] of [["accounts", "name", "Local account"], ["commands", "description", "Setup command"], ["scripts", "label", "Installation script"], ["data-images", "path", "Data image"], ["partitions", "action", "Disk operation"]]) rows(kind).forEach((row, index) => {
      const title = kind === "partitions" ? `${rowControl(row, "action").selectedOptions[0].textContent} · disk ${rowValue(row, "disk")}` : rowValue(row, key) || `${label} ${index + 1}`;
      row.querySelector(".answer-row-title").textContent = title;
      row.querySelector(".answer-row-remove").setAttribute("aria-label", `Remove ${title}`);
    });
    for (const picker of byId("answer-form").querySelectorAll("select")) {
      const control = pickers.get(picker);
      control.load(picker.value);
      control.setDisabled(picker.disabled);
    }
    filterSearch(byId("answer-search").value.trim().toLowerCase());
    requestAnimationFrame(() => layoutFields(byId("answer-groups").querySelectorAll(".answer-grid")));
    if (value("mode") === "xml") sections.get("advanced").open = sections.get("advanced-settings").open = true;
    renderIssues(validate());
  }
  function filterSearch(term) {
    const matches = text => text.toLowerCase().includes(term);
    for (const [id, title] of groups) {
      if (id === "advanced-settings") continue;
      const section = sections.get(id), categoryMatches = Boolean(term && (matches(title) || advancedGroups.has(id) && matches("Advanced settings")));
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
      section.hidden = value("deployment-mode") === "upgrade" && !["installation", "deployment", "advanced"].includes(id) || value("deployment-mode") === "wds" && id === "data-images" || Boolean(term && !categoryMatches && !hasMatch);
      if (term && !section.hidden) section.open = true;
    }
    const advanced = sections.get("advanced-settings");
    advanced.hidden = Boolean(term && [...advancedGroups].every(id => sections.get(id).hidden));
    if (term && !advanced.hidden) advanced.open = true;
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
    if (value("deployment-mode") === "upgrade") return issues;
    if (value("disk-ui") === "Never" && value("partition-mode") !== "custom" && !["erase", "existing"].includes(value("disk-mode"))) add(["disk-ui", "disk-mode"], "Choose a specific installation disk before hiding the disk configuration screen completely.");
    if (value("partition-mode") === "custom") {
      const operations = rows("partitions"), wiped = new Map(), extended = new Map();
      if (!operations.length) add("partition-mode", "Add at least one operation for the custom disk layout.");
      if (value("disk-mode") === "erase") add(["partition-mode", "disk-mode"], "Choose an existing, automatic or interactive destination with a custom disk layout. The preset erase flow cannot run at the same time.");
      for (const row of operations) {
        const action = rowValue(row, "action"), disk = String(Number(rowValue(row, "disk")));
        if ((action === "wipe" || action === "modify" && rowValue(row, "format")) && rowValue(row, "confirm") && rowValue(row, "confirm") !== disk) add(rowControl(row, "confirm"), "Confirm this wipe or format operation by typing its disk number again.");
        if (action === "wipe") {
          if (wiped.has(disk)) add([rowControl(wiped.get(disk), "disk"), rowControl(row, "disk")], "Only one wipe operation is allowed per disk.");
          wiped.set(disk, row);
        }
        if (action === "create") {
          if (extended.has(disk) && (rowValue(extended.get(disk), "type") !== "Extended" || rowValue(row, "type") !== "Logical")) add([rowControl(extended.get(disk), "extend"), rowControl(row, "disk")], "A partition extended to fill a disk must be its final creation operation, except for logical partitions inside an extended partition.");
          if (rowValue(row, "extend") === "true") extended.set(disk, row);
        }
        if (action === "modify" && rowValue(row, "format") === "FAT32" && rowValue(row, "label").length > 11) add(rowControl(row, "label"), "FAT32 partition labels must be at most 11 characters.");
      }
    }
    const fullPath = (path, variables = false) => ( /^[a-z]:\\/i.test(path) || variables && /^%[a-z_][a-z_0-9]*%\\/i.test(path)) && !/["<>|?*]/.test(path) && !path.slice(2).includes(":") && !path.split("\\").some(part => [".", ".."].includes(part));
    const wimPath = path => /\.wim$/i.test(path) && !/["<>|?*]/.test(path) && !path.replace(/^[a-z]:\\/i, "").includes(":") && (!path.startsWith("\\\\") || /^\\\\[^\\]+\\[^\\]+\\/.test(path)) && (!path.includes("%") || /^%configsetroot%\\[^%]+$/i.test(path));
    if (value("pe-log-path") && !fullPath(value("pe-log-path"))) add("pe-log-path", "Use a full local fixed-disk directory, such as C:\\SetupLogs. Network paths and environment variables are unsupported.");
    if (value("pe-pagefile-path") && (!fullPath(value("pe-pagefile-path"), true) || value("pe-pagefile-path").endsWith("\\"))) add("pe-pagefile-path", "Use a full fixed-disk file path, optionally starting with a Windows environment variable.");
    if (value("image-path") && !wimPath(value("image-path"))) add("image-path", "Use a local, network or relative path ending in .wim.");
    if (/%configsetroot%/i.test(value("image-path")) && value("configuration-set") !== "true") add(["image-path", "configuration-set"], "Enable Use a Windows SIM configuration set when an image path uses %configsetroot%.");
    const networkSources = [value("image-path"), ...nonEmptyLines(value("drivers"))];
    if (value("deployment-mode") === "wds") {
      if (value("pe-network") === "false") add(["deployment-mode", "pe-network"], "Windows Deployment Services requires Windows PE networking.");
      if (["wds-user", "wds-password", "wds-domain"].some(id => value(id)) && (!value("wds-user") || !value("wds-password"))) add(["wds-user", "wds-password"], "Supply both a WDS user and password, or leave WDS credentials empty.");
    }
    for (const row of rows("data-images")) {
      if (rowControl(row, "path").disabled) continue;
      const path = rowValue(row, "path"); networkSources.push(path);
      if (path && !wimPath(path)) add(rowControl(row, "path"), "Use a local, network or relative data image path ending in .wim.");
      if (/%configsetroot%/i.test(path) && value("configuration-set") !== "true") add([rowControl(row, "path"), "configuration-set"], "Enable Use a Windows SIM configuration set when a data image path uses %configsetroot%.");
      if (rowValue(row, "key") === "/IMAGE/INDEX" && !/^[1-9]\d*$/.test(rowValue(row, "value"))) add(rowControl(row, "value"), "Use a positive whole number for the data image index.");
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
    for (const row of commands) {
      if (rowValue(row, "path").length > (rowValue(row, "pass") === "windowsPE" ? 1024 : 259)) add(rowControl(row, "path"), "This command is too long for its configuration pass. Use an embedded script.");
      if (rowValue(row, "pass") === "windowsPE" && /^"?\\\\/.test(rowValue(row, "path"))) {
        networkSources.push(rowValue(row, "path").replace(/^"/, ""));
        if (rowValue(row, "auth") !== "credentials") add(rowControl(row, "auth"), "Supply credentials for a Windows PE command on a network share.");
      }
    }
    if (value("pe-network") === "false" && networkSources.some(path => path.startsWith("\\\\"))) add("pe-network", "Enable Windows PE networking to access the configured network paths.");
    const setupScripts = Boolean(value("wifi-mode") || value("builtin-admin") || value("guest-tools") || rows("scripts").length || value("disk-mode") === "erase" && value("recovery") === "false");
    const asynchronous = commands.filter(row => rowValue(row, "execution") === "async");
    if (asynchronous.length > 500) add(rowControl(asynchronous[0], "execution"), "Windows PE supports up to 500 asynchronous commands.");
    for (const pass of ["windowsPE", "specialize", "auditUser"]) {
      const selected = commands.filter(row => rowValue(row, "pass") === pass && rowValue(row, "execution") !== "async");
      const generated = pass === "specialize" && setupScripts ? 2 : pass === "windowsPE" && value("disk-mode") === "erase" ? 1 : 0;
      if (selected.length + generated > 500) add(rowControl(selected[0], "pass"), `${pass} supports up to 500 synchronous commands, including generated setup steps.`);
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
        if (!config || typeof text !== "string" || config.choices && !config.choices.some(([value]) => value === text) && key !== "autologon" && key !== "edition") throw new Error(`Unsupported setting or value: ${key}.`);
      }
    };
    check(profile.values, groups.flatMap(([, , fields]) => fields));
    for (const [kind, fields] of Object.entries(rowFields)) {
      const entries = profile[kind] ?? (["data-images", "partitions"].includes(kind) ? [] : null);
      if (!Array.isArray(entries) || entries.length > 500) throw new Error(`Invalid ${kind} entries.`);
      for (const row of entries) check(row, fields);
    }
    const logon = profile.values.autologon;
    if (logon && !(logon === "@Administrator" && profile.values["builtin-admin"] === "true") && !profile.accounts.some(row => row.name === logon)) throw new Error("Automatic sign-in must refer to an enabled account in these settings.");
    byId("answer-form").reset();
    if (isoEditions === null && profile.values.edition && ![...controls.get("edition").options].some(option => option.value === profile.values.edition)) controls.get("edition").add(new Option(profile.values.edition, profile.values.edition));
    for (const [key, text] of Object.entries(profile.values)) if (key !== "autologon") controls.get(key).value = text;
    if (!Object.hasOwn(profile.values, "product-key-ui")) controls.get("product-key-ui").value = "Never";
    for (const kind of Object.keys(rowFields)) for (const row of profile[kind] || []) addRow(kind, row);
    for (const row of rows("partitions")) rowControl(row, "confirm").value = "";
    update(); controls.get("autologon").value = logon || ""; controls.get("disk-confirm").value = ""; update();
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
    const finish = () => {
      for (const pass of passes) { const settings = [...root.children].find(child => child.getAttribute("pass") === pass); if (settings) root.append(settings); }
      if (value("architecture") === "all") for (const settings of [...root.children].filter(child => child.localName === "settings")) for (const original of [...settings.children]) { const copy = original.cloneNode(true); copy.setAttribute("processorArchitecture", "arm64"); settings.append(copy); }
      for (const child of [...root.children]) if (child.namespaceURI !== ns) root.append(child);
      const metadata = xml.createElementNS(extraNs, "Configuration"), profile = capture(); profile.values.mode = "guided"; delete profile.values.xml; metadata.textContent = base64(JSON.stringify(profile)); root.append(metadata);
      indentGeneratedXml(root); metadata.setAttribute("hash", fingerprint(root));
      return serialize(xml);
    };
    if (value("deployment-mode") === "upgrade") {
      setting("windowsPE", "setup", "UpgradeData/Upgrade", "true");
      if (value("upgrade-ui")) setting("windowsPE", "setup", "UpgradeData/WillShowUI", value("upgrade-ui"));
      return finish();
    }
    for (const [id, pass, name, path] of [
      ["product-key", "windowsPE", "setup", "UserData/ProductKey/Key"], ["product-key-ui", "windowsPE", "setup", "UserData/ProductKey/WillShowUI"], ["accept-eula", "windowsPE", "setup", "UserData/AcceptEula"], ["dynamic-update", "windowsPE", "setup", "DynamicUpdate/Enable"], ["compact", "windowsPE", "setup", "ImageInstall/OSImage/Compact"],
      ...[["diagnostics", "Diagnostics/OptIn"], ["compliance-ui", "ComplianceCheck/DisplayReport"], ["dynamic-update-ui", "DynamicUpdate/WillShowUI"], ["image-ui", "ImageInstall/OSImage/WillShowUI"], ["disk-ui", "DiskConfiguration/WillShowUI"], ["disable-disk-encryption", "DiskConfiguration/DisableEncryptedDiskProvisioning"], ["configuration-set", "UseConfigurationSet"], ["pe-restart", "Restart"], ["setup-owner", "UserData/FullName"], ["setup-organization", "UserData/Organization"], ["pe-network", "EnableNetwork"], ["pe-firewall", "EnableFirewall"], ["pe-log-path", "LogPath"], ["pe-pagefile-path", "PageFile/Path"], ["pe-pagefile-size", "PageFile/Size"], ["pe-width", "Display/HorizontalResolution"], ["pe-height", "Display/VerticalResolution"], ["pe-depth", "Display/ColorDepth"], ["pe-refresh", "Display/RefreshRate"], ["image-path", "ImageInstall/OSImage/InstallFrom/Path"], ["image-user", "ImageInstall/OSImage/InstallFrom/Credentials/Username"], ["image-password", "ImageInstall/OSImage/InstallFrom/Credentials/Password"], ["image-domain", "ImageInstall/OSImage/InstallFrom/Credentials/Domain"]].map(([id, path]) => [id, "windowsPE", "setup", path]),
      ["setup-language", "windowsPE", "winpe", "SetupUILanguage/UILanguage"], ["timezone", "oobeSystem", "shell", "TimeZone"], ["computer", "specialize", "shell", "ComputerName"], ["owner", "specialize", "shell", "RegisteredOwner"], ["organization", "specialize", "shell", "RegisteredOrganization"], ["activation-key", "specialize", "shell", "ProductKey"],
      ["hide-license", "oobeSystem", "shell", "OOBE/HideEULAPage"], ["hide-oem", "oobeSystem", "shell", "OOBE/HideOEMRegistrationScreen"], ["hide-online", "oobeSystem", "shell", "OOBE/HideOnlineAccountScreens"], ["hide-wireless", "oobeSystem", "shell", "OOBE/HideWirelessSetupInOOBE"], ["protection", "oobeSystem", "shell", "OOBE/ProtectYourPC"]
    ]) if (value(id)) setting(pass, name, path, controls.get(id).type === "number" ? Number(value(id)) : value(id));
    for (const [id, path] of [["language", "UILanguage"], ["user-locale", "UserLocale"], ["keyboard", "InputLocale"], ["fallback", "UILanguageFallback"], ["system-locale", "SystemLocale"]]) if (value(id)) for (const [pass, name] of [["windowsPE", "winpe"], ["oobeSystem", "locale"]]) setting(pass, name, path, value(id));
    const imageKey = value("image-selection") || "/IMAGE/NAME", imageValue = value("image-selection") === "/IMAGE/INDEX" ? String(Number(value("image-index"))) : value("image-selection") ? value("image-value") : value("edition");
    if (imageValue) { const metadata = node(component("windowsPE", "setup"), "ImageInstall/OSImage/InstallFrom/MetaData", undefined, true); node(metadata, "Key", imageKey); node(metadata, "Value", imageValue); }
    const destination = value("deployment-mode") === "wds" ? "WindowsDeploymentServices/ImageSelection/InstallTo" : "ImageInstall/OSImage/InstallTo";
    if (value("disk-mode") === "existing") for (const [id, path] of [["disk", "DiskID"], ["partition", "PartitionID"]]) setting("windowsPE", "setup", `${destination}/${path}`, Number(value(id)));
    if (value("disk-mode") === "available") setting("windowsPE", "setup", "ImageInstall/OSImage/InstallToAvailablePartition", "true");
    if (value("disk-ui") === "Never" && value("partition-mode") !== "custom") {
      const disk = node(component("windowsPE", "setup"), "DiskConfiguration/Disk", undefined, true);
      node(disk, "DiskID", Number(value("disk"))); node(disk, "WillWipeDisk", "false");
    }
    if (value("partition-mode") === "custom") {
      const disks = new Map(), orders = new Map();
      for (const row of rows("partitions")) {
        const id = Number(rowValue(row, "disk")), action = rowValue(row, "action");
        if (!disks.has(id)) {
          const disk = node(component("windowsPE", "setup"), "DiskConfiguration/Disk", undefined, true);
          node(disk, "DiskID", id); node(disk, "WillWipeDisk", "false"); disks.set(id, disk);
        }
        const disk = disks.get(id);
        if (action === "wipe") { node(disk, "WillWipeDisk", "true"); continue; }
        const create = action === "create", kind = create ? "Create" : "Modify", orderKey = `${id}/${kind}`, order = (orders.get(orderKey) || 0) + 1;
        orders.set(orderKey, order);
        const partition = node(disk, `${kind}Partitions/${kind}Partition`, undefined, true); node(partition, "Order", order);
        if (create) {
          node(partition, "Type", rowValue(row, "type"));
          if (rowValue(row, "extend") === "true") node(partition, "Extend", "true");
          else node(partition, "Size", Number(rowValue(row, "size")));
        } else {
          node(partition, "PartitionID", Number(rowValue(row, "partition")));
          for (const [key, path] of [["extend", "Extend"], ["format", "Format"], ["label", "Label"], ["letter", "Letter"], ["active", "Active"], ["type-id", "TypeID"]]) if (rowValue(row, key)) node(partition, path, rowValue(row, key));
        }
      }
    }
    if (value("deployment-mode") === "wds") for (const [id, path] of [["wds-login-ui", "Login/WillShowUI"], ["wds-user", "Login/Credentials/Username"], ["wds-password", "Login/Credentials/Password"], ["wds-domain", "Login/Credentials/Domain"], ["wds-image-ui", "ImageSelection/WillShowUI"], ["wds-image", "ImageSelection/InstallImage/ImageName"], ["wds-group", "ImageSelection/InstallImage/ImageGroup"], ["wds-file", "ImageSelection/InstallImage/Filename"]]) if (value(id)) setting("windowsPE", "setup", `WindowsDeploymentServices/${path}`, value(id));
    if (value("deployment-mode") === "wds") for (const [id, path] of [["wds-login-ui", "Login/WillShowUI"], ["wds-image-ui", "ImageSelection/WillShowUI"]]) if (!value(id)) setting("windowsPE", "setup", `WindowsDeploymentServices/${path}`, "OnError");
    for (const [index, row] of rows("data-images").entries()) {
      if (rowControl(row, "path").disabled) continue;
      const image = node(component("windowsPE", "setup"), "ImageInstall/DataImage", undefined, true);
      node(image, "Order", index + 1); node(image, "InstallFrom/Path", rowValue(row, "path"));
      const metadata = node(image, "InstallFrom/MetaData", undefined, true);
      node(metadata, "Key", rowValue(row, "key")); node(metadata, "Value", rowValue(row, "value"));
      for (const [key, path] of [["disk", "DiskID"], ["partition", "PartitionID"]]) node(image, `InstallTo/${path}`, Number(rowValue(row, key)));
      if (rowValue(row, "auth") === "credentials") for (const [key, path] of [["user", "Username"], ["password", "Password"], ["domain", "Domain"]]) if (rowValue(row, key)) node(image, `InstallFrom/Credentials/${path}`, rowValue(row, key));
    }
    if (!value("pe-network") && (value("deployment-mode") === "wds" || [value("image-path"), ...nonEmptyLines(value("drivers")), ...rows("data-images").filter(row => !rowControl(row, "path").disabled).map(row => rowValue(row, "path")), ...rows("commands").filter(row => rowValue(row, "pass") === "windowsPE").map(row => rowValue(row, "path").replace(/^"/, ""))].some(path => path.startsWith("\\\\")))) setting("windowsPE", "setup", "EnableNetwork", "true");
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
      for (const [path, text] of [["DiskID", disk], ["PartitionID", gpt ? 3 : 2]]) setting("windowsPE", "setup", `${destination}/${path}`, text);
      if (value("deployment-mode") === "wds") { if (!value("wds-image-ui")) setting("windowsPE", "setup", "WindowsDeploymentServices/ImageSelection/WillShowUI", "OnError"); }
      else if (!value("image-ui")) setting("windowsPE", "setup", "ImageInstall/OSImage/WillShowUI", "OnError");
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
    for (const row of rows("commands")) commands.push([rowValue(row, "pass"), rowValue(row, "path"), rowValue(row, "description"), rowValue(row, "reboot"), rowValue(row, "execution"), row]);
    for (const [pass, path, description, reboot, execution, row] of commands) {
      if (path.length > (pass === "windowsPE" ? 1024 : 259)) throw new Error("An installation command exceeds the Windows limit. Shorten the setup folder or use an embedded script.");
      const kind = execution === "async" ? "RunAsynchronous" : "RunSynchronous", orderKey = `${pass}/${kind}`;
      const order = orders[orderKey] = (orders[orderKey] || 0) + 1;
      if (order > 500) throw new Error("Windows supports up to 500 commands per configuration pass.");
      const command = node(component(pass, pass === "windowsPE" ? "setup" : "deployment"), `${kind}/${kind}Command`, undefined, true);
      node(command, "Order", order); node(command, "Path", path); if (description) node(command, "Description", description); if (reboot && pass !== "windowsPE") node(command, "WillReboot", reboot);
      if (pass === "windowsPE" && row && rowValue(row, "auth") === "credentials") for (const [key, name] of [["user", execution === "async" ? "Username" : "UserName"], ["password", "Password"], ["domain", "Domain"]]) if (rowValue(row, key)) node(command, `Credentials/${name}`, rowValue(row, key));
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
      sections.get("advanced").open = sections.get("advanced-settings").open = true;
      return;
    }
    for (const [id, , fields] of groups) {
      const changed = fields.some(([key, , config]) => {
        const control = controls.get(key), defaultValue = config.value ?? (config.toggle ? "" : config.choices?.[0]?.[0] ?? "");
        return !control.disabled && read(control) !== defaultValue;
      });
      sections.get(id).open = changed || Boolean(lists.get(id)?.children.length);
    }
    sections.get("advanced-settings").open = [...advancedGroups].some(id => sections.get(id).open);
  }
  const presetPicker = createChoicePicker(
    byId("answer-preset-picker"), byId("answer-preset-options"), byId("answer-preset-selection"),
    () => [{ value: "defaults", label: "Windows defaults" }, ...(debugMode.enabled ? [{ value: "debug", label: "All preferences (debug)" }, { value: "debug-install", label: "Installation test (debug)" }] : [])],
    "Choose a preset", choice => {
      initialize();
      if (choice.value === "defaults") {
        byId("answer-form").reset();
        showImportedChanges();
        status("Windows defaults restored.");
      } else {
        const profile = {
          format: "orbital-answer", version: 1,
          values: {
            edition: "Windows 11 Pro", architecture: "all", "product-key-ui": "Never", "accept-eula": "true", "dynamic-update": "false", compact: "true", encryption: "false",
            "disk-mode": "erase", disk: "1", layout: "gpt", "system-size": "512", recovery: "true", "recovery-size": "3072",
            language: "en-US", "setup-language": "en-US", "user-locale": "en-US", keyboard: "en-US", fallback: "en-GB", "system-locale": "en-US", timezone: "UTC",
            computer: "ORBITAL-DEBUG", owner: "Orbital Debug", organization: "Orbital Debug",
            "builtin-admin": "true", "builtin-password": "DebugOnly123!", "builtin-name": "Administrator", autologon: "OrbitalDebug", "logon-count": "2", obscure: "true",
            "hide-license": "true", "hide-oem": "true", "hide-online": "true", "hide-wireless": "true", protection: "3",
            "join-mode": "domain", "join-name": "example.invalid", "join-user": "debug", "join-password": "DebugOnly123!", "join-domain": "example.invalid", "join-ou": "OU=Debug,DC=example,DC=invalid",
            "wifi-mode": "personal", ssid: "Orbital-Debug", "wifi-auth": "WPA2PSK", "wifi-password": "DebugOnly123!", "wifi-hidden": "true",
            drivers: "D:\\Drivers", "guest-tools": "virtualbox", payload: "C:\\OrbitalDebug\\Orbital\\Setup", logging: "false", windows: "visible", mode: "guided",
            "image-selection": "/IMAGE/INDEX", "image-version": "11", "image-index": "1", "image-location": "custom", "image-path": "D:\\sources\\install.wim",
            diagnostics: "false", "compliance-ui": "OnError", "dynamic-update-ui": "Never", "image-ui": "OnError", "disk-ui": "OnError", "disable-disk-encryption": "true", "configuration-set": "false", "pe-restart": "Restart", "setup-owner": "Orbital Debug", "setup-organization": "Orbital Debug",
            "pe-network": "true", "pe-firewall": "true", "pe-log-path": "C:\\SetupLogs", "pe-pagefile": "custom", "pe-pagefile-path": "C:\\pagefile.sys", "pe-pagefile-size": "512", "pe-display": "custom", "pe-width": "1024", "pe-height": "768", "pe-depth": "32", "pe-refresh": "60"
          },
          accounts: [{ name: "OrbitalDebug", display: "Orbital Debug", group: "Custom", groups: "Administrators;Users", "password-mode": "custom", password: "DebugOnly123!", description: "Example debug account" }],
          commands: [{ pass: "specialize", description: "Example debug command", path: "cmd.exe /c echo Orbital debug", reboot: "Never" }, { pass: "windowsPE", execution: "async", description: "Example asynchronous command", path: "cmd.exe /c echo Orbital Windows PE debug" }],
          scripts: [{ type: "ps1", label: "Example debug script", content: "Write-Output 'Orbital debug'" }],
          "data-images": [{ path: "D:\\sources\\data.wim", key: "/IMAGE/INDEX", value: "1", disk: "1", partition: "3" }]
        };
        if (choice.value === "debug-install") {
          for (const key of ["edition", "product-key-ui", "disk-mode", "disk", "layout", "system-size", "recovery", "recovery-size", "language", "setup-language", "fallback", "join-mode", "join-name", "join-user", "join-password", "join-domain", "join-ou", "wifi-mode", "ssid", "wifi-auth", "wifi-password", "wifi-hidden", "drivers", "guest-tools", "image-selection", "image-version", "image-index", "image-location", "image-path", "image-ui", "disk-ui", "pe-log-path", "pe-pagefile", "pe-pagefile-path", "pe-pagefile-size", "pe-display", "pe-width", "pe-height", "pe-depth", "pe-refresh"]) delete profile.values[key];
          profile["data-images"] = [];
          profile.accounts[0].group = "Administrators";
          profile.accounts[0].groups = "";
        }
        restore(profile);
        showImportedChanges();
        status(choice.value === "debug-install" ? "Installation test preset loaded. Review the image selection and example account credentials; choose the disk in Setup." : "Debug preset loaded with example values. Review all settings; disk erasure still requires confirmation.");
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
      if (metadata && metadata.getAttribute("hash") === fingerprint(xml.documentElement)) restore(JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode(Uint8Array.from(atob(metadata.textContent), character => character.charCodeAt(0)))));
      else { controls.get("xml").value = serialize(xml); controls.get("mode").value = "xml"; update(); }
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
