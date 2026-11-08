"use strict";

(() => {
  const shield = byId("administrator-access");
  const dialog = byId("administrator-access-dialog");
  const relaunch = byId("administrator-access-relaunch");
  const dismiss = byId("administrator-access-dismiss");
  const note = byId("administrator-access-note");
  const error = byId("administrator-access-error");
  let administrator = null;
  let offered = false;
  let opening = false;
  let relaunching = false;

  const working = () => busy || injectBusy || Boolean(injectSelecting)
    || byId("uninstall-inventory").getAttribute("aria-busy") === "true";

  function showError(message = "") {
    error.textContent = message;
    error.hidden = !message;
  }

  function render() {
    shield.dataset.state = administrator === true ? "granted" : administrator === false ? "missing" : "unknown";
    shield.title = administrator === true ? "Administrator access detected" : administrator === false ? "Administrator access not detected" : "Administrator access could not be determined";
    shield.setAttribute("aria-label", shield.title);
    byId("administrator-access-title").textContent = shield.title;
    relaunch.hidden = administrator === true;
    relaunch.disabled = !invoke || relaunching || working();
    note.hidden = administrator === true;
    dismiss.textContent = administrator === true ? "Close" : "Continue without";
  }

  async function explain() {
    if (opening || relaunching || document.querySelector("dialog[open]")) return;
    offered = true;
    opening = true;
    showError(working() ? "Finish the current operation before relaunching." : "");
    render();
    try {
      while (await confirmDialog(dialog)) {
        if (working()) {
          showError("Finish the current operation before relaunching.");
          render();
          continue;
        }
        relaunching = true;
        render();
        try {
          if (await invoke("relaunch_as_administrator")) return;
          showError("The Windows permission request was cancelled. Orbital is still running without administrator access.");
        } catch (reason) {
          showError(String(reason));
        } finally {
          relaunching = false;
          render();
        }
      }
    } finally {
      opening = false;
    }
  }

  function offerAtStartup() {
    if (offered || administrator !== false) return;
    if (working() || document.querySelector("dialog[open]")) {
      setTimeout(offerAtStartup, 250);
      return;
    }
    explain();
  }

  shield.addEventListener("click", explain);
  if (invoke) {
    invoke("administrator_status").then(value => {
      administrator = value;
      render();
      offerAtStartup();
    }).catch(reason => {
      render();
      showError(`Could not check administrator access: ${String(reason)}`);
    });
  } else {
    render();
  }
})();
