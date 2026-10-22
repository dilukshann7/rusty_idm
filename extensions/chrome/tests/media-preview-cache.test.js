import test from "node:test";
import assert from "node:assert/strict";
import { MediaPreviewCache } from "../media-preview-cache.js";
const cache = now => new MediaPreviewCache(now || (() => 1000));
const candidate = { kind: "hls", url: "https://example.com/master.m3u8" };
const preview = (id = "preview", expires = 121000) => ({ status: "media_preview", preview_id: id, expires_at_ms: expires });

test("concurrent players and repeated opens share one inspection for the same tab, document and source", async () => {
  const c = cache(); let calls = 0, finish;
  const load = () => { calls++; return new Promise(resolve => finish = resolve); };
  const first = c.inspect(1, "doc", candidate, load);
  const second = c.inspect(1, "doc", candidate, load);
  await Promise.resolve();
  assert.equal(calls, 1);
  finish(preview());
  assert.equal((await first).preview_id, (await second).preview_id);
  await c.inspect(1, "doc", candidate, load);
  assert.equal(calls, 1);
});

test("near-expiry previews remain usable while one replacement warms in background", async () => {
  let now = 1000, finish, calls = 0;
  const c = cache(() => now);
  await c.inspect(1, "doc", candidate, async () => preview("old", 20000));
  now = 6000;
  const load = () => { calls++; return new Promise(resolve => finish = resolve); };
  assert.equal((await c.inspect(1, "doc", candidate, load)).preview_id, "old");
  assert.equal((await c.inspect(1, "doc", candidate, load)).preview_id, "old");
  assert.equal(calls, 1);
  now = 20001;
  const waiting = c.inspect(1, "doc", candidate, load);
  finish(preview("fresh", 140000));
  assert.equal((await waiting).preview_id, "fresh");
});

test("navigation discards old previews and late inspection results", async () => {
  const c = cache(); let finish;
  const pending = c.inspect(1, "old", candidate, () => new Promise(resolve => finish = resolve));
  await Promise.resolve(); c.clearTab(1); finish(preview("old"));
  assert.equal((await pending).status, "declined");
  let calls = 0;
  await c.inspect(1, "new", candidate, async () => { calls++; return preview("new"); });
  assert.equal(calls, 1);
  await c.inspect(2, "new", candidate, async () => { calls++; return preview("other-tab"); });
  assert.equal(calls, 2);
});

test("speculative work is bounded to two native inspections and sixty-four cached sources", async () => {
  const c = cache(); const finishes = [];
  const load = () => new Promise(resolve => finishes.push(resolve));
  const first = c.inspect(1, "doc", candidate, load);
  const second = c.inspect(2, "doc", candidate, load);
  await Promise.resolve();
  assert.equal((await c.inspect(3, "doc", candidate, load)).code, "busy");
  finishes.forEach(resolve => resolve(preview())); await Promise.all([first, second]);
  for (let tab = 3; tab < 80; tab++) await c.inspect(tab, "doc", candidate, async () => preview());
  assert.equal(c.entries.size, 64);
});

test("failed inspections can be retried and clear invalidates an in-flight refresh", async () => {
  const c = cache();
  await assert.rejects(c.inspect(1, "doc", candidate, async () => { throw Error("offline"); }));
  assert.equal((await c.inspect(1, "doc", candidate, async () => preview())).status, "media_preview");
  c.clear();
  assert.equal(c.entries.size, 0);
  let finish;
  const pending = c.inspect(1, "doc", candidate, () => new Promise(resolve => finish = resolve));
  await Promise.resolve(); c.clear(); finish(preview());
  assert.equal((await pending).code, "expired");
  assert.equal(c.entries.size, 0);
});

test("explicit refresh bypasses an unexpired cache entry", async () => {
  const c = cache(); let calls = 0;
  const load = async () => preview(String(++calls));
  await c.inspect(1, "doc", candidate, load);
  const fresh = await c.inspect(1, "doc", candidate, load, { refresh: true });
  assert.equal(calls, 2);
  assert.equal(fresh.preview_id, "2");
});
