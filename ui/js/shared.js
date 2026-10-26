"use strict";

const invoke = window.__TAURI__?.core.invoke;
const byId = (id) => document.getElementById(id);

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

function createChangeRow(value, title) {
  const row = document.createElement("label");
  const checkbox = document.createElement("input");
  checkbox.type = "checkbox";
  checkbox.value = value;
  checkbox.checked = true;
  const text = document.createElement("span");
  text.textContent = title;
  row.append(checkbox, text);
  return { row, checkbox, text };
}
