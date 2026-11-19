"use strict";

const invoke = window.__TAURI__?.core.invoke;
const byId = (id) => document.getElementById(id);
const element = (tag, className = "", textContent = "") => Object.assign(document.createElement(tag), { className, textContent });
const administratorState = value => value === true ? "granted" : value === false ? "missing" : "unknown";

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

function openDialog(dialog) {
  dialog.showModal();
  const titleBar = byId("window-titlebar");
  if (titleBar) {
    const logo = titleBar.querySelector(".window-logo");
    const spin = () => logo?.getAnimations().find(animation => animation.animationName === "orbit-spin");
    const startTime = spin()?.startTime ?? document.timeline.currentTime;
    const restoreSpin = () => {
      const animation = spin();
      if (animation && startTime != null) animation.startTime = startTime;
    };
    dialog.append(titleBar);
    restoreSpin();
    dialog.addEventListener("close", () => {
      if (titleBar.parentElement !== dialog) return;
      document.body.prepend(titleBar);
      restoreSpin();
    }, { once: true });
  }
}

function showDialog(dialog) {
  dialog.returnValue = "cancel";
  return new Promise((resolve) => {
    dialog.addEventListener("close", () => resolve(dialog.returnValue), { once: true });
    openDialog(dialog);
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
