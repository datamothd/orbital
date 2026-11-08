"use strict";

const invoke = window.__TAURI__?.core.invoke;
const byId = (id) => document.getElementById(id);
const element = (tag, className = "", textContent = "") => Object.assign(document.createElement(tag), { className, textContent });

let tweaksOutputReveal = 0;
let pendingTweaksOutput = null;

function revealTweaksOutput(entry) {
  pendingTweaksOutput = entry;
  const generation = ++tweaksOutputReveal;
  requestAnimationFrame(async () => {
    if (generation !== tweaksOutputReveal) return;
    if (typeof waitForWindowLayout === "function") await waitForWindowLayout();
    if (generation !== tweaksOutputReveal || byId("tweaks-panel").hidden || document.querySelector("dialog:modal")) return;
    const output = byId("tweaks-output-history");
    output.scrollTop = entry.offsetTop;
    const page = document.scrollingElement;
    if (typeof moveScroll === "function") moveScroll(page, 0, page.scrollHeight);
    else page.scrollTop = page.scrollHeight;
    pendingTweaksOutput = null;
  });
}

function writeTweaksOutput(message, { error = false, reveal = false } = {}) {
  const text = String(message || "").replace(/\r\n?/g, "\n").trim() || "Ready.";
  const output = byId("status");
  if (output.textContent !== text) output.textContent = text;
  const entry = output.parentElement;
  entry.classList.toggle("error", error);
  if (reveal || error) revealTweaksOutput(entry);
  return entry;
}

document.addEventListener("close", () => {
  if (pendingTweaksOutput) revealTweaksOutput(pendingTweaksOutput);
}, true);

let backdropDialog = null;
function dialogOutsideBounds(event) {
  const dialog = event.target;
  if (!dialog.matches?.("dialog:modal")) return null;
  const bounds = dialog.getBoundingClientRect();
  return event.clientX < bounds.left || event.clientX > bounds.right
    || event.clientY < bounds.top || event.clientY > bounds.bottom ? dialog : null;
}

document.addEventListener("pointerdown", event => {
  backdropDialog = event.button === 0 ? dialogOutsideBounds(event) : null;
});
document.addEventListener("pointercancel", () => { backdropDialog = null; });
document.addEventListener("click", event => {
  const dialog = backdropDialog;
  backdropDialog = null;
  if (dialog && dialogOutsideBounds(event) === dialog) dialog.close("cancel");
  if (pendingTweaksOutput && !byId("tweaks-panel").hidden) revealTweaksOutput(pendingTweaksOutput);
});

function showDialog(dialog) {
  dialog.returnValue = "cancel";
  return new Promise((resolve) => {
    dialog.addEventListener("close", () => resolve(dialog.returnValue), { once: true });
    dialog.showModal();
  });
}

async function confirmDialog(dialog) {
  return await showDialog(dialog) === "apply";
}

function createChangeRow(value, title, description = "", visual = false) {
  const row = element("label", visual ? "visual-preference" : "");
  const checkbox = document.createElement("input");
  checkbox.type = "checkbox";
  checkbox.value = value;
  checkbox.checked = true;
  if (description) {
    row.title = description;
    checkbox.setAttribute("aria-description", description);
  }
  const text = element("span", "", title);
  row.append(checkbox, text);
  return { row, checkbox, text };
}
