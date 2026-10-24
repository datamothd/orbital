"use strict";

const injectFiles = { iso: null, xml: null };

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
    ? "Both files are staged. ISO writing will be available in a future update."
    : count ? `Choose ${injectFiles.iso ? "an answer file" : "a source ISO"} to complete the image plan.`
    : "Select an ISO and an answer file to preview the image. No files are modified.");
}

for (const kind of Object.keys(injectFiles)) {
  const input = byId(`inject-${kind}`);
  input.addEventListener("change", async () => {
    const file = input.files[0];
    if (!file) return;
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
      if (input.files[0] !== file) return;
      setInjectFile(kind, file);
    } catch (error) {
      if (input.files[0] !== file) return;
      setInjectFile(kind);
      input.setAttribute("aria-invalid", "true");
      setInjectStatus(error.message, true);
    }
  });
  byId(`inject-${kind}-remove`).addEventListener("click", () => {
    setInjectFile(kind);
    input.focus();
  });
}

byId("inject-download").addEventListener("click", async (event) => {
  if (!invoke) return;
  event.preventDefault();
  try {
    await invoke("open_windows_download");
  } catch (error) {
    setInjectStatus(`Could not open the download page: ${error}`, true);
  }
});
