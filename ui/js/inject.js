"use strict";

const injectFiles = { iso: null, xml: null };
let injectBusy = false;
let injectSelecting = null;
let injectWriting = false;

function setInjectBusy(value, selecting = null) {
  injectBusy = value;
  injectSelecting = value ? selecting : null;
  if (!value) injectWriting = false;
  updateInjectControls();
}

function updateInjectControls() {
  for (const kind of Object.keys(injectFiles)) {
    const input = byId(`inject-${kind}`);
    const card = input.closest(".inject-file");
    const dimmed = injectWriting || injectSelecting === kind;
    input.disabled = injectBusy;
    byId(`inject-${kind}-remove`).disabled = injectBusy;
    card.inert = injectBusy;
    card.setAttribute("aria-busy", String(dimmed));
  }
  byId("inject-iso").title = injectWriting || injectSelecting === "iso" ? "" : injectFiles.iso?.name || "No image selected";
  byId("inject-start").disabled = injectBusy || !invoke || !injectFiles.iso || !injectFiles.xml;
  document.querySelector(".inject-inputs").setAttribute("aria-busy", String(injectWriting));
}

function setInjectStatus(message, error = false) {
  const status = byId("inject-status");
  status.textContent = message;
  status.title = message;
  status.classList.toggle("error", error);
}

function setInjectFile(kind, file = null) {
  const input = byId(`inject-${kind}`);
  injectFiles[kind] = file;
  if (kind === "iso") answer.setIso(file);
  if (!file) input.value = "";
  input.removeAttribute("aria-invalid");
  renderInject();
}

async function changeInjectFile(kind, clear = false) {
  if (injectBusy) return;
  setInjectBusy(true, kind);
  try {
    const file = invoke ? await invoke(clear ? "clear_injection_file" : "select_injection_file", { kind }) : null;
    if (clear || file) setInjectFile(kind, file || null);
    return clear || Boolean(file);
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
    byId(`inject-${kind}-name`).title = file?.name || "";
    if (kind === "xml") byId("inject-xml").title = file?.name || "";
    byId(`inject-${kind}-action`).textContent = `${file ? "Replace" : "Choose"} ${kind === "xml" ? "answer file" : "ISO"}`;
    byId(`inject-${kind}`).closest(".inject-file").classList.toggle("selected", Boolean(file));
    byId(`inject-${kind}-remove`).hidden = !file;
  }
  byId("inject-output").textContent = injectFiles.iso
    ? `${injectFiles.iso.name.replace(/\.iso$/i, "")}-orbital.iso` : "Choose a source image first";
  byId("inject-output").title = injectFiles.iso ? byId("inject-output").textContent : "";
  byId("inject-source-entry").classList.toggle("ready", Boolean(injectFiles.iso));
  byId("inject-answer-entry").classList.toggle("ready", Boolean(injectFiles.xml));
  const count = Object.values(injectFiles).filter(Boolean).length;
  setInjectStatus(count === 2
    ? invoke ? "Both files are ready. Choose Inject ISO to continue." : "Open Orbital to write your ISO."
    : count ? `Choose ${injectFiles.iso ? "an answer file" : "a source ISO"} to complete the image plan.`
    : "Select an ISO and an answer file to prepare your image.");
  updateInjectControls();
}

for (const kind of Object.keys(injectFiles)) {
  const input = byId(`inject-${kind}`);
  input.addEventListener("click", (event) => {
    if (kind === "xml") { answer.choose(); return; }
    if (!invoke) return;
    event.preventDefault();
    changeInjectFile(kind);
  });
  input.addEventListener("change", () => {
    if (invoke || kind === "xml") return;
    const file = input.files[0];
    if (!file) return;
    const message = !file.name.toLowerCase().endsWith(`.${kind}`) || !file.size
      ? `Choose a non-empty .${kind} file.` : "";
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
    setInjectStatus(mode === "copy" ? "Choose where to save the injected copy…" : "Preparing the selected ISO…");
    const progress = new window.__TAURI__.core.Channel();
    progress.onmessage = ({ message, percent }) => {
      if (!receivingProgress) return;
      if (!injectWriting) {
        injectWriting = true;
        updateInjectControls();
        indicator.hidden = false;
        start.hidden = true;
        cancel.hidden = false;
        cancel.disabled = false;
      }
      if (percent == null) indicator.removeAttribute("value");
      else indicator.value = percent;
      setInjectStatus(percent == null ? message : `${message} ${percent}%`);
    };
    const output = await invoke("inject_iso", { duplicate: mode === "copy", progress });
    if (output) {
      byId("inject-output").textContent = output;
      byId("inject-output").title = output;
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
    start.hidden = false;
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
