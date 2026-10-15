"use strict";

if (window.__TAURI__) {
  const appWindow = window.__TAURI__.window.getCurrentWindow();
  const { LogicalSize } = window.__TAURI__.dpi;
  const main = document.querySelector("main");
  const maxHeight = 720;
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
        const height = Math.min(maxHeight, Math.ceil(main.getBoundingClientRect().height));
        if (height !== lastHeight) {
          await appWindow.setSize(new LogicalSize(640, height));
          lastHeight = height;
        }
      } while (pending);
    } catch (error) {
      observer.disconnect();
      setStatus(`Could not fit window to content: ${error}`, true);
    } finally {
      resizing = false;
    }
  }

  const observer = new ResizeObserver(() => requestAnimationFrame(fitWindow));
  observer.observe(main);
  document.fonts.ready.then(fitWindow);
}
