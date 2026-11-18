"use strict";

const appWindow = window.__TAURI__?.window.getCurrentWindow();

if (appWindow) {
  document.body.insertAdjacentHTML("afterbegin", `
    <header id="window-titlebar" aria-label="Title bar">
      <div class="window-drag-region" aria-hidden="true"></div>
      <button type="button" id="window-minimize" aria-label="Minimize" title="Minimize">
        <span class="icon icon-minus" aria-hidden="true"></span>
      </button>
      <button type="button" id="window-close" aria-label="Close" title="Close">
        <span class="icon icon-close" aria-hidden="true"></span>
      </button>
    </header>`);
  byId("window-minimize").addEventListener("click", () => appWindow.minimize());
  byId("window-close").addEventListener("click", () => appWindow.close());
}
