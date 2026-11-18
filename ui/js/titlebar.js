"use strict";

const appWindow = window.__TAURI__?.window.getCurrentWindow();

if (appWindow) {
  document.body.insertAdjacentHTML("afterbegin", `
    <header id="window-titlebar" aria-label="Title bar">
      ${byId("navigation-sidebar") ? `
      <div class="window-brand" aria-hidden="true">
        <span class="icon icon-orbital window-logo"></span>
        <span class="window-wordmark"><span>&gt;</span> ORBITAL</span>
      </div>` : ""}
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
