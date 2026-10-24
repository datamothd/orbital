"use strict";

const invoke = window.__TAURI__?.core.invoke;
const byId = (id) => document.getElementById(id);

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
