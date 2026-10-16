"use strict";

let scrollFrame = 0;
let scrollTarget = 0;
function stopScrolling() {
  cancelAnimationFrame(scrollFrame);
  scrollFrame = 0;
}
window.addEventListener("pointerdown", stopScrolling);
window.addEventListener("keydown", stopScrolling);
window.addEventListener("wheel", (event) => {
  if (event.ctrlKey || event.shiftKey || !event.cancelable || !event.deltaY
    || event.target.closest(".folder-view-options, dialog")) return;
  event.preventDefault();
  const unit = event.deltaMode === 1 ? 16 : event.deltaMode === 2 ? innerHeight : 1;
  const page = document.scrollingElement;
  scrollTarget = Math.max(0, Math.min((scrollFrame ? scrollTarget : page.scrollTop)
    + event.deltaY * unit, page.scrollHeight - page.clientHeight));
  stopScrolling();
  const start = page.scrollTop;
  const started = performance.now();
  function step(now) {
    const progress = Math.min(1, (now - started) / 180);
    page.scrollTop = start + (scrollTarget - start) * (1 - (1 - progress) ** 3);
    scrollFrame = progress < 1 ? requestAnimationFrame(step) : 0;
  }
  scrollFrame = requestAnimationFrame(step);
}, { passive: false });

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
