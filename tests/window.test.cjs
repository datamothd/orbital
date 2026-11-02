const { test } = require("node:test");
const assert = require("node:assert/strict");
const { readFileSync } = require("node:fs");
const { join } = require("node:path");
const { runInNewContext } = require("node:vm");

function windowLayout() {
  const attributes = new Set();
  const events = new Map();
  const sizes = [];
  const state = { editor: true, height: 700, wrapping: 40 };
  const root = {
    setAttribute: name => attributes.add(name),
    removeAttribute: name => attributes.delete(name),
    toggleAttribute: (name, enabled) => enabled ? attributes.add(name) : attributes.delete(name),
  };
  const main = {
    getBoundingClientRect: () => ({
      height: state.editor ? 1200 : state.height + (attributes.has("data-window-content-fits") ? 0 : state.wrapping),
    }),
  };
  const window = {
    innerWidth: 640,
    innerHeight: 720,
    addEventListener: (name, callback) => events.set(name, callback),
    removeEventListener: name => events.delete(name),
    __TAURI__: {
      window: {
        getCurrentWindow: () => ({
          setSize: async ({ width, height }) => {
            sizes.push([width, height]);
            window.innerWidth = width;
            window.innerHeight = height;
            events.get("resize")?.();
          },
        }),
      },
      dpi: { LogicalSize: class { constructor(width, height) { Object.assign(this, { width, height }); } } },
    },
  };
  let fit;
  runInNewContext(readFileSync(join(__dirname, "../ui/js/window.js"), "utf8"), {
    window,
    document: {
      documentElement: root,
      querySelector: selector => selector === "main" ? main : null,
      createElement: () => ({ offsetWidth: 100, clientWidth: 85, remove() {} }),
      body: { append() {} },
      fonts: { ready: { then() {} } },
    },
    ResizeObserver: class {
      constructor(callback) { fit = callback; }
      observe() {}
      disconnect() {}
    },
    byId: () => ({ getAttribute: () => "false" }),
    createSettingToggle: () => ({ enabled: true }),
    setStatus: message => assert.fail(message),
  });
  return { state, sizes, fit, attributes };
}

test("returning from the answer editor fits the ISO selector without a leftover scrollbar", async () => {
  const layout = windowLayout();
  await layout.fit();
  layout.state.editor = false;
  await layout.fit();
  assert.deepEqual(layout.sizes.at(-1), [640, 700]);
  assert(layout.attributes.has("data-window-content-fits"));
  const resizeCount = layout.sizes.length;
  await layout.fit();
  assert.equal(layout.sizes.length, resizeCount);
});

test("content at the maximum window height fits without a scrollbar", async () => {
  const layout = windowLayout();
  await layout.fit();
  layout.state.editor = false;
  layout.state.height = 720;
  await layout.fit();
  assert(layout.attributes.has("data-window-content-fits"));
  assert.deepEqual(layout.sizes.at(-1), [640, 720]);
});

test("content taller than the maximum window height keeps its scrollbar", async () => {
  const layout = windowLayout();
  layout.state.editor = false;
  layout.state.height = 760;
  await layout.fit();
  assert(!layout.attributes.has("data-window-content-fits"));
  assert.deepEqual(layout.sizes.at(-1), [640, 720]);
  await layout.fit();
  assert.equal(layout.sizes.length, 1);
});
