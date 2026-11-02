"use strict";

let scrollFrame = 0;
const scrollAnimations = new Map();
const smoothScrolling = createSettingToggle("smooth-scroll", "smoothScrolling", false, stopScrolling);
let waitForWindowLayout = async () => { await new Promise(requestAnimationFrame); };

function stopScrolling() {
  cancelAnimationFrame(scrollFrame);
  scrollFrame = 0;
  scrollAnimations.clear();
}

function scrollContainer(target, axis, delta) {
  const vertical = axis === "y";
  const position = vertical ? "scrollTop" : "scrollLeft";
  const extent = node => vertical ? node.scrollHeight - node.clientHeight : node.scrollWidth - node.clientWidth;
  for (let node = target instanceof Element ? target : target?.parentElement;
    node && node !== document.body && node !== document.documentElement; node = node.parentElement) {
    const style = getComputedStyle(node);
    const overflow = vertical ? style.overflowY : style.overflowX;
    const contained = /^(contain|none)$/.test(vertical ? style.overscrollBehaviorY : style.overscrollBehaviorX);
    if (/^(auto|scroll|overlay)$/.test(overflow)) {
      const current = scrollAnimations.get(node)?.[axis] ?? node[position];
      const maximum = extent(node);
      if (contained || (delta < 0 ? current > 0 : current < maximum)) return node;
    }
    if (node.matches("dialog[open]")) return null;
  }
  return document.querySelector("dialog:modal") ? null : document.scrollingElement;
}

function animateScroll(now) {
  const modal = document.querySelector("dialog:modal");
  for (const [node, animation] of scrollAnimations) {
    if (!node.isConnected || (modal && node !== modal && !modal.contains(node))) {
      scrollAnimations.delete(node);
      continue;
    }
    const progress = Math.min(1, (now - animation.started) / 180);
    const eased = 1 - (1 - progress) ** 3;
    node.scrollLeft = animation.startX + (animation.x - animation.startX) * eased;
    node.scrollTop = animation.startY + (animation.y - animation.startY) * eased;
    if (progress === 1) scrollAnimations.delete(node);
  }
  scrollFrame = scrollAnimations.size ? requestAnimationFrame(animateScroll) : 0;
}

function moveScroll(node, dx, dy) {
  const previous = scrollAnimations.get(node);
  const x = Math.max(0, Math.min((previous?.x ?? node.scrollLeft) + dx, node.scrollWidth - node.clientWidth));
  const y = Math.max(0, Math.min((previous?.y ?? node.scrollTop) + dy, node.scrollHeight - node.clientHeight));
  if (!smoothScrolling.enabled) {
    node.scrollLeft = x;
    node.scrollTop = y;
    return;
  }
  if (x === node.scrollLeft && y === node.scrollTop) return;
  scrollAnimations.set(node, { x, y, startX: node.scrollLeft, startY: node.scrollTop, started: performance.now() });
  if (!scrollFrame) scrollFrame = requestAnimationFrame(animateScroll);
}

window.addEventListener("pointerdown", stopScrolling);
window.addEventListener("keydown", stopScrolling);
window.addEventListener("blur", stopScrolling);
window.addEventListener("wheel", event => {
  if (event.ctrlKey || event.defaultPrevented || !event.cancelable || (!event.deltaX && !event.deltaY)) return;
  let dx = event.deltaX, dy = event.deltaY;
  if (event.shiftKey && !dx) { dx = dy; dy = 0; }
  const movements = new Map();
  for (const [axis, delta] of [["x", dx], ["y", dy]]) {
    if (!delta) continue;
    const node = scrollContainer(event.target, axis, delta);
    if (!node) continue;
    const unit = event.deltaMode === 1 ? 16 : event.deltaMode === 2 ? axis === "y" ? node.clientHeight : node.clientWidth : 1;
    const movement = movements.get(node) || { x: 0, y: 0 };
    movement[axis] += delta * unit;
    movements.set(node, movement);
  }
  if (!movements.size) return;
  event.preventDefault();
  if (!smoothScrolling.enabled) stopScrolling();
  for (const [node, movement] of movements) moveScroll(node, movement.x, movement.y);
}, { passive: false });

if (window.__TAURI__) {
  const appWindow = window.__TAURI__.window.getCurrentWindow();
  const { LogicalSize } = window.__TAURI__.dpi;
  const main = document.querySelector("main");
  const root = document.documentElement;
  const baseWidth = 640;
  const maxHeight = 720;
  const scrollbar = document.createElement("div");
  scrollbar.className = "scrollbar-probe";
  document.body.append(scrollbar);
  const scrollbarWidth = scrollbar.offsetWidth - scrollbar.clientWidth;
  scrollbar.remove();
  let resizing = false;
  let pending = false;
  let needsScrollbar = false;
  let lastWidth = 0;
  let lastHeight = 0;
  let injectHeight = 0;
  const layoutWaiters = [];

  function waitForSize(width, height) {
    if (Math.abs(innerWidth - width) <= 1 && Math.abs(innerHeight - height) <= 1) return Promise.resolve();
    return new Promise(resolve => {
      const finish = () => {
        clearTimeout(timeout);
        window.removeEventListener("resize", changed);
        resolve();
      };
      const changed = () => {
        if (Math.abs(innerWidth - width) <= 1 && Math.abs(innerHeight - height) <= 1) finish();
      };
      const timeout = setTimeout(finish, 1000);
      window.addEventListener("resize", changed);
      changed();
    });
  }

  function syncScrollbar() {
    const sizeReady = Math.abs(window.innerWidth - lastWidth) <= 1
      && Math.abs(window.innerHeight - lastHeight) <= 1;
    root.toggleAttribute("data-window-content-fits", !needsScrollbar || pending || !sizeReady);
  }

  async function fitWindow() {
    if (resizing) {
      pending = true;
      syncScrollbar();
      return new Promise(resolve => layoutWaiters.push(resolve));
    }

    resizing = true;
    try {
      do {
        pending = false;
        root.setAttribute("data-window-content-fits", "");
        const contentHeight = Math.ceil(main.getBoundingClientRect().height);
        const editorOpen = !byId("answer-page").hidden;
        const height = editorOpen ? injectHeight || maxHeight : Math.min(maxHeight, contentHeight);
        if (!byId("inject-panel").hidden) injectHeight = height;
        needsScrollbar = contentHeight > height;
        const width = baseWidth + (needsScrollbar ? scrollbarWidth : 0);
        if (width !== lastWidth || height !== lastHeight) {
          lastWidth = width;
          lastHeight = height;
          syncScrollbar();
          await appWindow.setSize(new LogicalSize(width, height));
          await waitForSize(width, height);
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
      for (const resolve of layoutWaiters.splice(0)) resolve();
    }
  }

  waitForWindowLayout = async () => {
    await document.fonts.ready;
    await fitWindow();
    await new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)));
  };

  const observer = new ResizeObserver(fitWindow);
  observer.observe(main);
  window.addEventListener("resize", syncScrollbar);
  document.fonts.ready.then(fitWindow);
}
