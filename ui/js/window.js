"use strict";

let scrollFrame = 0;
let scrollTarget = 0;
const smoothScrolling = createSettingToggle("smooth-scroll", "smoothScrolling", true, stopScrolling);

function stopScrolling() {
  cancelAnimationFrame(scrollFrame);
  scrollFrame = 0;
}
window.addEventListener("pointerdown", stopScrolling);
window.addEventListener("keydown", stopScrolling);
window.addEventListener("wheel", (event) => {
  if (event.target.closest("textarea")) { stopScrolling(); return; }
  if (!smoothScrolling.enabled || event.ctrlKey || event.shiftKey || !event.cancelable || !event.deltaY
    || document.querySelector("dialog:modal") || event.target.closest(".folder-view-options, dialog")) return;
  event.preventDefault();
  const unit = event.deltaMode === 1 ? 16 : event.deltaMode === 2 ? innerHeight : 1;
  const page = document.scrollingElement;
  scrollTarget = Math.max(0, Math.min((scrollFrame ? scrollTarget : page.scrollTop)
    + event.deltaY * unit, page.scrollHeight - page.clientHeight));
  stopScrolling();
  const start = page.scrollTop;
  const started = performance.now();
  function step(now) {
    if (document.querySelector("dialog:modal")) {
      stopScrolling();
      return;
    }
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
  const root = document.documentElement;
  const baseWidth = 640;
  const maxHeight = 720;
  const scrollbar = document.createElement("div");
  scrollbar.style.cssText = "position:fixed;width:100px;height:100px;overflow:scroll;visibility:hidden";
  document.body.append(scrollbar);
  const scrollbarWidth = scrollbar.offsetWidth - scrollbar.clientWidth;
  scrollbar.remove();
  let resizing = false;
  let pending = false;
  let needsScrollbar = false;
  let lastWidth = 0;
  let lastHeight = 0;

  function syncScrollbar() {
    const sizeReady = Math.abs(window.innerWidth - lastWidth) <= 1
      && Math.abs(window.innerHeight - lastHeight) <= 1;
    root.toggleAttribute("data-window-content-fits", !needsScrollbar || pending || !sizeReady);
  }

  async function fitWindow() {
    if (resizing) {
      pending = true;
      syncScrollbar();
      return;
    }

    resizing = true;
    try {
      do {
        pending = false;
        const contentHeight = Math.ceil(main.getBoundingClientRect().height);
        needsScrollbar = contentHeight > maxHeight;
        const tweaksActive = byId("tweaks-tab").getAttribute("aria-selected") === "true";
        const width = baseWidth + (tweaksActive && needsScrollbar ? scrollbarWidth : 0);
        const height = Math.min(maxHeight, contentHeight);
        if (width !== lastWidth || height !== lastHeight) {
          lastWidth = width;
          lastHeight = height;
          syncScrollbar();
          await appWindow.setSize(new LogicalSize(width, height));
        }
        syncScrollbar();
      } while (pending);
    } catch (error) {
      observer.disconnect();
      window.removeEventListener("resize", syncScrollbar);
      root.removeAttribute("data-window-content-fits");
      setStatus(`Could not fit window to content: ${error}`, true);
    } finally {
      resizing = false;
    }
  }

  const observer = new ResizeObserver(fitWindow);
  observer.observe(main);
  window.addEventListener("resize", syncScrollbar);
  document.fonts.ready.then(fitWindow);
}
