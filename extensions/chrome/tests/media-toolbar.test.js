import test from "node:test";
import assert from "node:assert/strict";
import { dom, Element } from "./helpers/media-dom.js";

function toolbar(options) {
  const env = dom(options);
  const bar = env.create();
  assert.ok(bar, "floating toolbar factory is installed in the isolated world");
  env.frame();
  return { ...env, bar };
}

test("toolbar floats above the page at the player's upper right without inserting a sibling", () => {
  const { bar, document } = toolbar();
  assert.equal(bar.root.parentNode, document.documentElement);
  assert.equal(bar.button.textContent, "Download this video");
  assert.equal(bar.root.style.left, "511px");
  assert.equal(bar.root.style.top, "104px");
  assert.equal(bar.root.popoverOpen, true);
});

test("drag threshold preserves clicks and a real drag suppresses its click without changing label", async () => {
  const { bar } = toolbar();
  await bar.button.fire("pointerdown");
  await bar.button.fire("pointermove", { clientX: 603 });
  await bar.button.fire("pointerup", { clientX: 603 });
  assert.equal(bar.consumeDragClick(), false);
  await bar.button.fire("pointerdown");
  await bar.button.fire("pointermove", { clientX: 640, clientY: 135 });
  await bar.button.fire("pointerup", { clientX: 640, clientY: 135 });
  assert.equal(bar.root.style.left, "551px");
  assert.equal(bar.root.style.top, "124px");
  assert.equal(bar.consumeDragClick(), true);
  assert.equal(bar.consumeDragClick(), false);
});

test("dragged offset follows player scrolling, layout movement, resizing, and zoom events", async () => {
  const { video, window, frame, bar } = toolbar();
  await bar.button.fire("pointerdown");
  await bar.button.fire("pointermove", { clientX: 630, clientY: 125 });
  await bar.button.fire("pointerup");
  video.rect = { ...video.rect, top: 50, bottom: 410, right: 800 };
  await window.fire("scroll"); frame();
  assert.equal(bar.root.style.left, "601px");
  assert.equal(bar.root.style.top, "64px");
  video.rect.right = 850;
  frame(); // player movement without a scroll/resize notification
  assert.equal(bar.root.style.left, "651px");
  await window.fire("resize"); frame();
  assert.equal(bar.root.style.left, "651px");
});

test("toolbar clamps inside the viewport and hides offscreen players", async () => {
  const { video, bar, frame, window } = toolbar();
  await bar.button.fire("pointerdown");
  await bar.button.fire("pointermove", { clientX: -600, clientY: -1000 });
  await bar.button.fire("pointerup");
  assert.equal(bar.root.style.left, "4px");
  assert.equal(bar.root.style.top, "4px");
  video.rect = { left: 0, right: 300, top: -400, bottom: -40, width: 300, height: 360 };
  await window.fire("scroll"); frame();
  assert.equal(bar.root.style.visibility, "hidden");
  video.rect.top = 0; video.rect.bottom = 360;
  frame();
  assert.equal(bar.root.style.visibility, "visible");
});

test("hover-only controls stay visible for toolbar focus and picker interaction", async () => {
  const { bar, video, tick, frame } = toolbar({ hover: true });
  assert.equal(bar.root.style.opacity, "0");
  assert.equal(bar.root.style.visibility, "visible", "idle hover bars remain keyboard focusable");
  await video.fire("mouseenter");
  assert.equal(bar.root.style.opacity, "1");
  await video.fire("mouseleave"); frame();
  assert.equal(bar.root.style.opacity, "1", "allow time to move from player to toolbar");
  tick();
  assert.equal(bar.root.style.opacity, "0");
  await video.fire("mouseenter");
  await bar.root.fire("focusin");
  await video.fire("mouseleave"); tick();
  assert.equal(bar.root.style.opacity, "1");
  bar.openPicker();
  await bar.root.fire("focusout", { relatedTarget: null }); tick();
  assert.equal(bar.root.style.opacity, "1");
  bar.closePopover(); tick();
  assert.equal(bar.root.style.opacity, "0");
});

test("picker and help close with Escape or an outside click and restore focus", async () => {
  const { bar, document } = toolbar();
  bar.openPicker();
  assert.equal(bar.picker.hidden, false);
  await document.fire("keydown", { key: "Escape" });
  assert.equal(bar.picker.hidden, true);
  assert.equal(bar.button.focused, true);
  await bar.helpButton.fire("click");
  assert.equal(bar.help.hidden, false);
  await document.fire("pointerdown");
  assert.equal(bar.help.hidden, true);
});

test("fullscreen hides unrelated bars and promotes a matching bar to the top layer", async () => {
  const { bar, document, video, frame } = toolbar();
  document.fullscreenElement = new Element("div");
  await document.fire("fullscreenchange"); frame();
  assert.equal(bar.root.style.visibility, "hidden");
  document.fullscreenElement.append(video);
  await document.fire("fullscreenchange"); frame();
  assert.equal(bar.root.style.visibility, "visible");
  assert.equal(bar.root.popoverOpen, true);
  document.fullscreenElement = video;
  await document.fire("fullscreenchange"); frame();
  assert.equal(bar.root.style.visibility, "hidden"); // native media fullscreen cannot contain an HTML bar
  document.fullscreenElement = null;
  await document.fire("fullscreenchange"); frame();
  assert.equal(bar.root.style.visibility, "visible");
});

test("fixed-position fallback supports browsers without popovers", () => {
  const { bar } = toolbar({ topLayer: false });
  assert.equal(bar.root.style.left, "511px");
  assert.equal(bar.root.style.visibility, "visible");
});

test("a failed top-layer request removes popover hiding so fixed fallback can render", () => {
  const env = dom();
  const createElement = env.document.createElement;
  env.document.createElement = tag => {
    const element = createElement(tag);
    element.showPopover = () => { throw new Error("top layer unavailable"); };
    return element;
  };
  const bar = env.create();
  assert.equal(bar.root.getAttribute("popover"), null);
  assert.equal(bar.root.style.visibility, "visible");
});

test("a cancelled drag does not swallow a later keyboard download action", async () => {
  const { bar } = toolbar();
  await bar.button.fire("pointerdown");
  await bar.button.fire("pointermove", { clientX: 640 });
  await bar.button.fire("pointercancel");
  assert.equal(bar.consumeDragClick(), false);
});

test("dismiss and disposal remove overlays, observers, and all event handlers", async () => {
  const env = dom();
  let dismissed = 0;
  const bar = env.create(() => dismissed++);
  assert.ok(bar, "floating toolbar exists");
  await bar.closeButton.fire("click");
  assert.equal(dismissed, 1);
  assert.equal(bar.root.isConnected, false);
  bar.dispose(); // idempotent
  assert.equal(env.frames.size, 0);
  assert.equal(env.timers.size, 0);
  assert.equal(env.window.listenerCount + env.document.listenerCount + env.video.listenerCount, 0);
  assert.ok(env.observers.every(item => item.disconnected));
});

test("formats render as numbered actions with keyboard navigation and trusted-click submission", async () => {
  const { bar } = toolbar();
  assert.equal(typeof bar.setFormats, "function", "menu accepts a ready format preview");
  const chosen = [];
  bar.setFormats("Video title", [
    { id: "1080", quality: "1080p", container: "mp4", audio: "aac", size: 1048576 },
    { id: "720", quality: "720p", container: "webm", audio: "opus", size: null },
  ], id => chosen.push(id));
  const [first, second] = bar.formatList.children;
  assert.match(first.textContent, /^1\. 1080p.*mp4.*aac.*1\.0 MB.*Video title/u);
  assert.match(second.textContent, /size unknown/u);
  await first.fire("keydown", { key: "ArrowDown" });
  assert.equal(second.focused, true);
  await second.fire("click", { isTrusted: false });
  assert.deepEqual(chosen, []);
  await second.fire("click");
  assert.deepEqual(chosen, ["720"]);
  bar.setBusy(true);
  await first.fire("click");
  assert.deepEqual(chosen, ["720"]);
});

test("all 128 quality choices render in engine order and remain keyboard selectable", async () => {
  const { bar } = toolbar();
  const variants = Array.from({ length: 128 }, (_, index) => ({
    id: `format-${index}`, quality: `${index + 144}p60`, container: index < 64 ? "mp4" : "webm",
    audio: index < 64 ? "aac" : "opus", size: null,
  }));
  const chosen = [];
  bar.setFormats("Video title", variants, id => chosen.push(id));
  assert.equal(bar.formatList.children.length, 128);
  const first = bar.formatList.children[0], last = bar.formatList.children[127];
  assert.match(first.textContent, /^1\. 144p60.*mp4/u);
  assert.match(last.textContent, /^128\. 271p60.*webm/u);
  await first.fire("keydown", { key: "End" });
  assert.equal(last.focused, true);
  await last.fire("click");
  assert.deepEqual(chosen, ["format-127"]);
  await last.fire("keydown", { key: "ArrowDown" });
  assert.equal(first.focused, true);
  bar.setBusy(true);
  await last.fire("click");
  assert.deepEqual(chosen, ["format-127"]);
});

test("an oversized quality list is rejected rather than truncated", () => {
  const { bar } = toolbar();
  const variant = { id: "720", quality: "720p", container: "mp4", audio: "aac", size: null };
  assert.throws(() => bar.setFormats("Video", Array(129).fill(variant), () => {}), /Too many video quality choices/u);
  assert.equal(bar.formatList.children.length, 0);
});
