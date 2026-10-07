"use strict";

const preferences = document.getElementById("explorer-preferences");

preferences.addEventListener("click", (event) => {
  if (event.target.closest("summary, button, a, input, select, textarea")) return;
  preferences.open = !preferences.open;
  if (!preferences.open) preferences.querySelector("summary").focus();
});

if (window.__TAURI__) {
  const appWindow = window.__TAURI__.window.getCurrentWindow();
  const { LogicalSize } = window.__TAURI__.dpi;
  const main = document.querySelector("main");
  let resizing = false;
  let pending = false;
  let lastHeight = 0;

  async function fitWindow() {
    if (resizing) {
      pending = true;
      return;
    }

    resizing = true;
    try {
      do {
        pending = false;
        const height = Math.ceil(main.getBoundingClientRect().height);
        if (height !== lastHeight) {
          await appWindow.setSize(new LogicalSize(640, height));
          lastHeight = height;
        }
      } while (pending);
    } catch (error) {
      observer.disconnect();
      const status = document.getElementById("status");
      status.textContent = `Could not fit window to content: ${error}`;
      status.classList.add("error");
    } finally {
      resizing = false;
    }
  }

  const observer = new ResizeObserver(() => requestAnimationFrame(fitWindow));
  observer.observe(main);
  preferences.addEventListener("toggle", () => requestAnimationFrame(fitWindow));
  document.fonts.ready.then(fitWindow);
}
