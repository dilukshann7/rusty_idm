import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { runInNewContext } from "node:vm";
import { MediaPreviewCache } from "../media-preview-cache.js";

const worker = readFileSync(new URL("../service-worker.js", import.meta.url), "utf8");
const start = worker.indexOf('      if (message.type === "media-inspect"');
const branch = worker.slice(start, worker.indexOf('      if (message.type === "media-submit"', start));
function context(load) {
  const candidate = { url: "https://example.com/master.m3u8", kind: "hls" };
  let observed = [candidate];
  const scope = {
    message: { type: "media-inspect", documentKey: "doc", ...candidate },
    sender: { tab: { id: 1 }, url: "https://example.com/watch" },
    policy: { panels: { enabled: true } }, excluded: () => false,
    documents: new Map([[1, { key: "doc" }]]),
    mediaPreviews: new MediaPreviewCache(), previewOwnership: new Map(),
    mediaObservations: { list: () => observed }, native: { send: load }, BROWSER: "chrome",
  };
  return { scope, removeObservation() { observed = []; },
    inspect: () => runInNewContext(`(async()=>{${branch}})()`, scope) };
}
const preview = () => ({ status: "media_preview", preview_id: "preview", expires_at_ms: Date.now() + 120000, title: "Public video", variants: [{ id: "1080" }] });

test("worker cache avoids repeat native inspection while retaining preview ownership", async () => {
  let calls = 0;
  const c = context(async () => { calls++; return preview(); });
  await c.inspect(); await c.inspect();
  assert.equal(calls, 1);
  assert.equal(c.scope.previewOwnership.get("preview").key, "doc");
  assert.equal(c.scope.previewOwnership.get("preview").variants.has("1080"), true);
  c.removeObservation();
  assert.equal((await c.inspect()).code, "unsupported", "cache cannot bypass public request evidence");
  c.scope.message.documentKey = "another-doc";
  assert.equal((await c.inspect()).status, "declined");
  assert.equal(calls, 1);
});

test("navigation or panel disable during inspection cannot issue a usable preview", async () => {
  let finish;
  const c = context(() => new Promise(resolve => finish = resolve));
  const pending = c.inspect(); await Promise.resolve();
  c.scope.documents.set(1, { key: "next" });
  finish(preview());
  assert.equal((await pending).code, "expired");
  assert.equal(c.scope.previewOwnership.size, 0);
  c.scope.documents.set(1, { key: "doc" });
  c.scope.policy.panels.enabled = false;
  assert.equal(await c.inspect(), undefined);
});

test("a cache hit at ownership capacity preserves other previews and explicit refresh reaches native", async () => {
  let calls = 0;
  const c = context(async () => { calls++; return preview(); });
  await c.inspect();
  const owner = c.scope.previewOwnership.get("preview");
  c.scope.previewOwnership.clear();
  for (let i = 0; i < 63; i++) c.scope.previewOwnership.set(`other-${i}`, owner);
  c.scope.previewOwnership.set("preview", owner);
  await c.inspect();
  assert.equal(c.scope.previewOwnership.has("other-0"), true);
  assert.equal(c.scope.previewOwnership.size, 64);
  c.scope.message.refresh = true;
  await c.inspect();
  assert.equal(calls, 2);
});
