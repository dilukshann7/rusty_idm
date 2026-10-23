import test from "node:test";
import assert from "node:assert/strict";
import { dom } from "./helpers/media-dom.js";

const flush = () => new Promise(resolve => setImmediate(resolve));
async function panel({ direct = false, submitStatus = "declined", hover = false, ready = true, holdSubmit = false, failSubmitOnce = false, singleSource = false } = {}) {
  const env = dom({ hover });
  const video = env.video;
  video.currentSrc = direct ? "https://example.com/movie.mp4" : "blob:https://example.com/video";
  video.readyState = ready ? 2 : 0;
  video.paused = !ready;
  const inspections = [], submissions = [], messages = [], submitReplies = [];
  const catalog = [
    { url: "https://example.com/master.m3u8", kind: "hls" },
    { url: "https://example.com/master.mpd", kind: "dash" },
  ];
  if (singleSource) catalog.splice(1);
  let controls, now = Date.now();
  env.context.Date = { now: () => now };
  const factory = env.context.RustyMediaToolbar;
  env.context.RustyMediaToolbar = { create(...args) { return controls = factory.create(...args); } };
  const settings = { enabled: true, video: true, hover_only: hover };
  env.context.location = { href: "https://example.com/watch" };
  env.context.chrome = { runtime: { sendMessage: async message => {
    messages.push(message);
    if (message.type === "page-policy") return { documentKey: "document", panels: { ...settings } };
    if (message.type === "media-candidates") return { candidates: [...catalog] };
    if (message.type === "media-inspect") return new Promise(resolve => inspections.push({ message, resolve }));
    if (message.type === "media-submit") {
      submissions.push(message);
      if (failSubmitOnce && submissions.length === 1) throw Error("connection lost after send");
      return holdSubmit ? new Promise(resolve => submitReplies.push(resolve)) : { status: submitStatus };
    }
    return { outcome: "accepted" };
  } } };
  env.runContent();
  await flush();
  assert.ok(controls, "content.js uses the floating toolbar");
  return { ...env, ...controls, sources: controls.sourceChoice, inspections, submissions, submitReplies, messages, settings, catalog,
    advance(ms) { now += ms; } };
}
function preview(id, title, variants = ["720"]) {
  return { status: "media_preview", preview_id: id, title, expires_at_ms: Date.now() + 60000,
    variants: variants.map(id => ({ id, quality: `${id}p`, container: "mp4", audio: "aac", size: 1048576 })) };
}

test("formats prepare during playback before the user clicks and reopening ready menus does no work", async () => {
  const p = await panel();
  assert.equal(p.inspections.length, 1, "inspect started before click");
  assert.equal(p.picker.hidden, true, "warming does not open the menu");
  assert.equal(p.submissions.length, 0);
  p.inspections[0].resolve(preview("warm", "Public video", ["1080", "720"]));
  await flush();
  const count = p.messages.length;
  await p.button.fire("click");
  assert.equal(p.picker.hidden, false);
  assert.equal(p.formatList.children.length, 2);
  assert.match(p.formatList.children[0].textContent, /1080p.*mp4.*aac.*1\.0 MB.*Public video/u);
  p.closePopover();
  await p.button.fire("click");
  assert.equal(p.messages.length, count, "ready click uses memory and does not repeat inspection");
});

test("cold click opens immediately and joins an in-flight inspection", async () => {
  const p = await panel();
  await p.button.fire("click");
  assert.equal(p.picker.hidden, false);
  assert.equal(p.inspections.length, 1);
  assert.ok(!p.button.disabled, "background inspection does not disable dragging");
  p.inspections[0].resolve(preview("warm", "Video")); await flush();
  assert.equal(p.formatList.children.length, 1);
});

test("source choice changes ignore an older inspection and quality rows submit directly", async () => {
  const p = await panel();
  assert.equal(p.inspections.length, 1);
  p.sources.value = "1";
  p.sources.fire("change"); await flush();
  p.inspections[1].resolve(preview("second-preview", "DASH choice", ["1080"])); await flush();
  p.inspections[0].resolve(preview("first-preview", "Old HLS choice")); await flush();
  assert.equal(p.formatList.children.length, 1);
  assert.match(p.formatList.children[0].textContent, /1080p.*DASH choice/u);
  await p.formatList.children[0].fire("click");
  assert.equal(p.submissions[0].previewId, "second-preview");
  assert.equal(p.submissions[0].variantId, "1080");
});

test("uncertain retries keep their receipt but a definitive decline allows a fresh attempt", async () => {
  const p = await panel({ failSubmitOnce: true });
  p.inspections[0].resolve(preview("preview", "Video", ["720", "1080"])); await flush();
  const [first, second] = p.formatList.children;
  await first.fire("click"); await first.fire("click"); await second.fire("click"); await first.fire("click");
  assert.equal(p.submissions[0].submitId, p.submissions[1].submitId);
  assert.notEqual(p.submissions[0].submitId, p.submissions[2].submitId);
  assert.equal(p.submissions[2].variantId, "1080");
  assert.notEqual(p.submissions[1].submitId, p.submissions[3].submitId);
});

test("Refresh formats obtains a new preview after a decline even if the old one has not expired", async () => {
  const p = await panel();
  p.inspections[0].resolve(preview("old", "Video")); await flush();
  await p.formatList.children[0].fire("click");
  assert.equal(p.retryButton.hidden, false);
  await p.retryButton.fire("click"); await flush();
  assert.equal(p.inspections.length, 2);
  assert.equal(p.inspections[1].message.refresh, true);
});

test("background refresh cannot replace an active submission or reactivate an accepted quality", async () => {
  const p = await panel({ holdSubmit: true });
  p.inspections[0].resolve(preview("old", "Video")); await flush();
  const row = p.formatList.children[0];
  p.advance(46000); p.tick(); await flush();
  assert.equal(p.inspections.length, 2, "near-expiry refresh is in flight");
  const submission = row.fire("click"); await flush();
  p.inspections[1].resolve(preview("new", "Video")); await flush();
  assert.equal(p.formatList.children[0], row, "keep the submitted preview until the receipt arrives");
  p.submitReplies[0]({ status: "accepted" }); await submission;
  assert.equal(row.disabled, true);
  p.advance(6000); await p.video.fire("play"); await flush();
  p.inspections[2].resolve(preview("newer", "Video")); await flush();
  assert.equal(p.formatList.children[0].disabled, true, "automatic refresh keeps accepted quality disabled");
});

test("expired rows cannot submit and trigger another preparation", async () => {
  const p = await panel();
  p.inspections[0].resolve(preview("old", "Video")); await flush();
  const row = p.formatList.children[0];
  p.advance(61000);
  await row.fire("click"); await flush();
  assert.equal(p.submissions.length, 0);
  assert.equal(p.inspections.length, 2);
  assert.equal(p.formatList.children.length, 0);
});

test("source changes during inspection or before submission never use old formats", async () => {
  const p = await panel();
  p.video.currentSrc = "blob:https://example.com/new-video";
  p.inspections[0].resolve(preview("old", "Old video")); await flush();
  assert.equal(p.formatList.children.length, 0);
  await p.button.fire("click"); await flush();
  p.inspections[1].resolve(preview("new", "New video", ["720", "1080"])); await flush();
  const row = p.formatList.children[0];
  p.context.location.href = "https://example.com/next-video";
  await row.fire("click");
  assert.equal(p.submissions.length, 0);
});

test("changing the available source cannot pair new source labels with old quality actions", async () => {
  const p = await panel();
  p.inspections[0].resolve(preview("old", "Old HLS video")); await flush();
  const oldRow = p.formatList.children[0];
  p.catalog.shift();
  p.advance(6000); p.tick(); await flush();
  assert.equal(p.sources.value, "0");
  assert.equal(p.inspections.length, 2);
  assert.equal(p.formatList.children.length, 0);
  await oldRow.fire("click");
  assert.equal(p.submissions.length, 0);
});

test("direct file actions stay direct, and switching to streaming starts warming", async () => {
  const p = await panel({ direct: true });
  assert.equal(p.inspections.length, 0);
  p.video.currentSrc = "https://example.com/second.webm";
  await p.button.fire("click");
  assert.equal(p.picker.hidden, true, "direct downloads never display an empty quality popup");
  assert.equal(p.messages.find(message => message.type === "panel-download").url, p.video.currentSrc);
  p.video.currentSrc = "blob:https://example.com/stream";
  await p.video.fire("loadedmetadata"); await flush();
  assert.equal(p.inspections.length, 1);
});

test("a direct-file cancellation keeps the quality popup closed", async () => {
  const p = await panel({ direct: true });
  const send = p.context.chrome.runtime.sendMessage;
  p.context.chrome.runtime.sendMessage = message => message.type === "panel-download"
    ? Promise.resolve({ outcome: "browser_continues" }) : send(message);
  await p.button.fire("click");
  assert.equal(p.picker.hidden, true);
  assert.equal(p.formatList.children.length, 0);
});

test("one ready quality goes straight to Rusty after a click, without background submission", async () => {
  const p = await panel({ singleSource: true });
  p.inspections[0].resolve(preview("only", "Video")); await flush();
  assert.equal(p.submissions.length, 0);
  await p.button.fire("click");
  assert.equal(p.picker.hidden, true);
  assert.equal(p.submissions.length, 1);
  assert.equal(p.submissions[0].variantId, "720");
});

test("one quality on the first source still offers the other source choices", async () => {
  const p = await panel();
  p.inspections[0].resolve(preview("first", "Video")); await flush();
  await p.button.fire("click");
  assert.equal(p.picker.hidden, false);
  assert.equal(p.sources.style.display, "block");
  assert.equal(p.submissions.length, 0);
});

test("choosing a quality closes the browser popup before waiting for the native dialog", async () => {
  const p = await panel({ holdSubmit: true });
  p.inspections[0].resolve(preview("multiple", "Video", ["720", "1080"])); await flush();
  await p.button.fire("click");
  const submission = p.formatList.children[0].fire("click"); await flush();
  assert.equal(p.picker.hidden, true);
  assert.equal(p.submissions.length, 1);
  p.submitReplies[0]({ status: "declined" }); await submission;
  assert.equal(p.picker.hidden, true);
});

test("a cold single-quality lookup only hands off for the still-open original click", async () => {
  const p = await panel({ singleSource: true });
  await p.button.fire("click");
  p.closePopover();
  p.inspections[0].resolve(preview("closed", "Video")); await flush();
  assert.equal(p.submissions.length, 0, "Escape/outside-close cancels the pending one-quality handoff");

  const changed = await panel();
  await changed.button.fire("click");
  changed.sources.value = "1";
  await changed.sources.fire("change"); await flush();
  changed.inspections[1].resolve(preview("dash", "New choice")); await flush();
  changed.inspections[0].resolve(preview("hls", "Old choice")); await flush();
  assert.equal(changed.submissions.length, 0, "a different source choice invalidates the original click");
});

test("unloaded players wait for playback or hover, and drag completion cannot download", async () => {
  const p = await panel({ ready: false });
  assert.equal(p.inspections.length, 0);
  await p.video.fire("mouseenter"); await flush();
  assert.equal(p.inspections.length, 1);
  const direct = await panel({ direct: true });
  await direct.button.fire("pointerdown");
  await direct.button.fire("pointermove", { clientX: 640 });
  await direct.button.fire("pointerup");
  await direct.button.fire("click");
  assert.equal(direct.messages.some(message => message.type === "panel-download"), false);
});

test("dismissal survives refresh, and disabling or unloading discards late warm results", async () => {
  const p = await panel();
  await p.closeButton.fire("click");
  p.inspections[0].resolve(preview("late", "Old video")); await flush();
  p.tick(); await flush();
  assert.equal(p.document.documentElement.children.filter(item => item.isConnected).length, 0);
  await p.window.fire("pagehide");
  assert.equal(p.frames.size + p.timers.size, 0);
  assert.equal(p.window.listenerCount, 0);
  const disabled = await panel();
  disabled.settings.enabled = false; disabled.tick(); await flush();
  assert.equal(disabled.root.isConnected, false);
  await disabled.window.fire("pagehide");
  assert.ok(disabled.observers.every(item => item.disconnected));
});
