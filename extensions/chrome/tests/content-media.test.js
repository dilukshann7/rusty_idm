import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { runInNewContext } from "node:vm";
import { randomUUID } from "node:crypto";

const source = readFileSync(new URL("../content.js", import.meta.url), "utf8");
const flush = () => new Promise(resolve => setImmediate(resolve));
class Element {
  constructor(tag) { this.tagName = tag.toUpperCase(); this.style = {}; this.children = []; this.handlers = {}; this.value = ""; this.isConnected = true; }
  append(...children) { this.children.push(...children); if (this.tagName === "SELECT" && !this.value) this.value = this.children[0]?.value || ""; }
  replaceChildren() { this.children = []; this.value = ""; }
  addEventListener(name, handler) { this.handlers[name] = handler; }
  removeEventListener() {}
  querySelector() { return null; }
  attachShadow() { return this.shadow = new Element("shadow"); }
  insertAdjacentElement(position, element) { this.panel = element; }
  remove() { this.isConnected = false; }
  fire(name) { return this.handlers[name]?.({ isTrusted: true }); }
}
async function panel({ direct = false, submitStatus = "declined" } = {}) {
  const video = new Element("video");
  video.currentSrc = direct ? "https://example.com/movie.mp4" : "blob:https://example.com/video";
  const inspections = [], submissions = [], messages = [];
  runInNewContext(source, {
    chrome: { runtime: { sendMessage: async message => {
      messages.push(message);
      if (message.type === "page-policy") return { documentKey: "document", panels: { enabled: true, video: true } };
      if (message.type === "media-candidates") return { candidates: [
        { url: "https://example.com/master.m3u8", kind: "hls" },
        { url: "https://example.com/master.mpd", kind: "dash" }
      ] };
      if (message.type === "media-inspect") return new Promise(resolve => inspections.push({ message, resolve }));
      if (message.type === "media-submit") { submissions.push(message); return { status: submitStatus }; }
      return { status: "accepted" };
    } } },
    document: { documentElement: {}, querySelectorAll: () => [video], createElement: tag => new Element(tag) },
    crypto: { randomUUID }, URL, MutationObserver: class { observe() {} disconnect() {} },
    addEventListener() {}, setInterval: () => 1, clearInterval() {}, setTimeout, clearTimeout
  });
  await flush();
  const [button, sources, formats, submit, detail] = video.panel.shadow.children;
  return { button, sources, formats, submit, detail, inspections, submissions, messages };
}
function preview(id, title, variants = ["720"]) {
  return { status: "media_preview", preview_id: id, title, expires_at_ms: Date.now() + 60000,
    variants: variants.map(id => ({ id, quality: `${id}p`, container: "mp4", audio: "aac", size: 1048576 })) };
}

test("media picker ignores an old inspection when the source choice changes", async () => {
  const p = await panel();
  const first = p.button.fire("click");
  await flush();
  p.sources.value = "1";
  p.sources.fire("change");
  await flush();
  p.inspections[1].resolve(preview("second-preview", "DASH choice", ["1080"]));
  await flush();
  p.inspections[0].resolve(preview("first-preview", "Old HLS choice"));
  await first;
  assert.equal(p.detail.textContent, "DASH choice");
  assert.match(p.formats.children[0].textContent, /1080p.*mp4.*aac.*1\.0 MB/u);
  await p.submit.fire("click");
  assert.equal(p.submissions[0].previewId, "second-preview");
  assert.equal(p.submissions[0].variantId, "1080");
});

test("choosing another format after a decline uses a new submission receipt id", async () => {
  const p = await panel();
  const pending = p.button.fire("click");
  await flush();
  p.inspections[0].resolve(preview("preview", "Video", ["720", "1080"]));
  await pending;
  await p.submit.fire("click");
  p.formats.value = "1080";
  p.formats.fire("change");
  await p.submit.fire("click");
  assert.notEqual(p.submissions[0].submitId, p.submissions[1].submitId);
  assert.equal(p.submissions[1].variantId, "1080");
});

test("direct file panels keep their existing download action", async () => {
  const p = await panel({ direct: true });
  await p.button.fire("click");
  assert.equal(p.messages.some(message => message.type === "media-inspect"), false);
  assert.equal(p.messages.find(message => message.type === "panel-download").url, "https://example.com/movie.mp4");
});
