"use strict";

const injectFiles = { iso: null, xml: null };
let injectBusy = false;

function setInjectBusy(value) {
  injectBusy = value;
  for (const kind of Object.keys(injectFiles)) {
    byId(`inject-${kind}`).disabled = value;
    byId(`inject-${kind}-remove`).disabled = value;
  }
  byId("inject-start").disabled = value || !invoke || !injectFiles.iso || !injectFiles.xml;
  document.querySelector(".inject-inputs").setAttribute("aria-busy", String(value));
}

function setInjectStatus(message, error = false) {
  const status = byId("inject-status");
  status.textContent = message;
  status.classList.toggle("error", error);
}

function setInjectFile(kind, file = null) {
  const input = byId(`inject-${kind}`);
  injectFiles[kind] = file;
  if (!file) input.value = "";
  input.removeAttribute("aria-invalid");
  renderInject();
}

async function changeInjectFile(kind, clear = false) {
  if (injectBusy) return;
  setInjectBusy(true);
  try {
    const file = invoke ? await invoke(clear ? "clear_injection_file" : "select_injection_file", { kind }) : null;
    if (clear || file) setInjectFile(kind, file || null);
  } catch (error) {
    setInjectStatus(String(error), true);
  } finally {
    setInjectBusy(false);
    byId(`inject-${kind}`).focus();
  }
}

function renderInject() {
  for (const [kind, file] of Object.entries(injectFiles)) {
    byId(`inject-${kind}-name`).textContent = file ? file.name
      : kind === "iso" ? "No image selected" : "No answer file selected";
    byId(`inject-${kind}-action`).textContent = `${file ? "Replace" : "Choose"} ${kind.toUpperCase()}`;
    byId(`inject-${kind}`).closest(".inject-file").classList.toggle("selected", Boolean(file));
    byId(`inject-${kind}-remove`).hidden = !file;
  }
  byId("inject-output").textContent = injectFiles.iso
    ? `${injectFiles.iso.name.replace(/\.iso$/i, "")}-orbital.iso` : "Choose a source image first";
  byId("inject-source-entry").classList.toggle("ready", Boolean(injectFiles.iso));
  byId("inject-answer-entry").classList.toggle("ready", Boolean(injectFiles.xml));
  const count = Object.values(injectFiles).filter(Boolean).length;
  setInjectStatus(count === 2
    ? invoke ? "Both files are ready. Choose Inject ISO to continue." : "Open Orbital to write your ISO."
    : count ? `Choose ${injectFiles.iso ? "an answer file" : "a source ISO"} to complete the image plan.`
    : "Select an ISO and an answer file to prepare your image.");
  setInjectBusy(injectBusy);
}

for (const kind of Object.keys(injectFiles)) {
  const input = byId(`inject-${kind}`);
  input.addEventListener("click", (event) => {
    if (!invoke) return;
    event.preventDefault();
    changeInjectFile(kind);
  });
  input.addEventListener("change", async () => {
    if (invoke) return;
    const file = input.files[0];
    if (!file) return;
    let message = "";
    try {
      if (!file.name.toLowerCase().endsWith(`.${kind}`) || !file.size) {
        throw new Error(`Choose a non-empty .${kind} file.`);
      }
      if (kind === "xml") {
        if (file.size > 1_048_576) throw new Error("Choose an answer file under 1 MiB.");
        const bytes = new Uint8Array(await file.arrayBuffer());
        const encoding = bytes[0] === 0xff && bytes[1] === 0xfe ? "utf-16le"
          : bytes[0] === 0xfe && bytes[1] === 0xff ? "utf-16be" : "utf-8";
        const xml = new DOMParser().parseFromString(new TextDecoder(encoding).decode(bytes), "application/xml");
        if (xml.querySelector("parsererror") || xml.documentElement.localName !== "unattend"
          || xml.documentElement.namespaceURI !== "urn:schemas-microsoft-com:unattend") {
          throw new Error("Choose a valid Windows unattend XML file.");
        }
      }
    } catch (error) {
      message = error.message || String(error);
    }
    if (input.files[0] !== file) return;
    setInjectFile(kind, message ? null : file);
    if (message) {
      input.setAttribute("aria-invalid", "true");
      setInjectStatus(message, true);
    }
  });
  byId(`inject-${kind}-remove`).addEventListener("click", () => changeInjectFile(kind, true));
}

byId("inject-start").addEventListener("click", async () => {
  if (injectBusy || !invoke || !injectFiles.iso || !injectFiles.xml) return;
  setInjectBusy(true);
  const indicator = byId("inject-progress");
  const cancel = byId("inject-cancel");
  const start = byId("inject-start");
  let receivingProgress = true;
  try {
    const mode = await showDialog(byId("inject-confirm"));
    if (mode !== "copy" && mode !== "original") return;
    indicator.hidden = false;
    indicator.removeAttribute("value");
    cancel.hidden = false;
    cancel.disabled = false;
    start.textContent = "Injecting…";
    setInjectStatus(mode === "copy" ? "Choose where to save the injected copy…" : "Preparing the selected ISO…");
    const progress = new window.__TAURI__.core.Channel();
    progress.onmessage = ({ message, percent }) => {
      if (!receivingProgress) return;
      if (percent == null) indicator.removeAttribute("value");
      else indicator.value = percent;
      setInjectStatus(percent == null ? message : `${message} ${percent}%`);
    };
    const output = await invoke("inject_iso", { duplicate: mode === "copy", progress });
    if (output) {
      byId("inject-output").textContent = output;
      setInjectStatus("ISO injected and verified.");
      showResult("ISO injected", `Your bootable ISO is ready: ${output}`);
    } else {
      setInjectStatus("Injection cancelled. The source ISO was not changed.");
    }
  } catch (error) {
    setInjectStatus(String(error), true);
    showResult("Could not inject ISO", String(error));
  } finally {
    receivingProgress = false;
    indicator.hidden = true;
    cancel.hidden = true;
    start.textContent = "Inject ISO";
    setInjectBusy(false);
  }
});

byId("inject-cancel").addEventListener("click", async () => {
  if (!injectBusy || byId("inject-cancel").hidden) return;
  byId("inject-cancel").disabled = true;
  try {
    await invoke("cancel_injection");
    if (injectBusy && !byId("inject-cancel").hidden) setInjectStatus("Cancelling injection…");
  } catch (error) {
    if (!injectBusy || byId("inject-cancel").hidden) return;
    setInjectStatus(String(error), true);
    byId("inject-cancel").disabled = false;
  }
});

byId("inject-download").addEventListener("click", async (event) => {
  if (!invoke) return;
  event.preventDefault();
  try {
    await invoke("open_windows_download");
  } catch (error) {
    setInjectStatus(`Could not open the download page: ${error}`, true);
  }
});
