import test from "node:test";
import assert from "node:assert/strict";
import { MediaObservations, MediaRequestEvidence, canSubmitPreview, manifestKind, youtubePage } from "../media-discovery.js";

test("recognizes only public single YouTube page URLs", () => {
  assert.equal(youtubePage("https://www.youtube.com/watch?v=abcdefghijk&list=playlist"), "https://www.youtube.com/watch?v=abcdefghijk");
  assert.equal(youtubePage("https://www.youtube.com/shorts/abcdefghijk"), "https://www.youtube.com/shorts/abcdefghijk");
  for (const url of ["https://evil.youtube.com/watch?v=abcdefghijk", "https://www.youtube.com/playlist?list=x", "https://www.youtube.com/live/abcdefghijk", "https://www.youtube.com/watch?v=bad"]) assert.equal(youtubePage(url), null);
});

test("manifest requests belong to their observed tab document and expire", () => {
  let now = 100000;
  const media = new MediaObservations(() => now);
  const document = { key: "first", at: 99000 };
  const hls = { url: "https://cdn.example/movie/master.m3u8?token=public", method: "GET", tabId: 8, timeStamp: now, responseHeaders: [] };
  assert.equal(manifestKind(hls), "hls");
  media.add(hls, document);
  media.add({ ...hls, url: "https://cdn.example/movie/stream", responseHeaders: [{ name: "Content-Type", value: "application/dash+xml" }] }, document);
  media.add({ ...hls, url: "https://cdn.example/private.mpd" }, document, true);
  assert.deepEqual(media.list(8, document, "https://example.com/watch" ).map((item) => item.kind), ["hls", "dash"]);
  assert.deepEqual(media.list(8, { key: "second", at: now }, "https://example.com/watch"), []);
  now += 60001;
  assert.deepEqual(media.list(8, document, "https://example.com/watch"), []);
});

test("a preview can only be submitted by its tab and document before expiry", () => {
  const owner = { tabId: 8, key: "first", expires: 120000, variants: new Set(["137+140"]) };
  assert.equal(canSubmitPreview(owner, 8, "first", "137+140", 119999), true);
  assert.equal(canSubmitPreview(owner, 9, "first", "137+140", 119999), false);
  assert.equal(canSubmitPreview(owner, 8, "second", "137+140", 119999), false);
  assert.equal(canSubmitPreview(owner, 8, "first", "18", 119999), false);
  assert.equal(canSubmitPreview(owner, 8, "first", "137+140", 120000), false);
});

test("public media evidence rejects credentials, missing headers, expiry and overload", () => {
  let now = 100000;
  const evidence = new MediaRequestEvidence(() => now);
  assert.equal(evidence.takePublic("worker-restarted"), false);
  evidence.observe({ requestId: "public", requestHeaders: [] });
  assert.equal(evidence.takePublic("public"), true);
  evidence.observe({ requestId: "redirect", requestHeaders: [{ name: "Cookie", value: "private-cookie" }] });
  evidence.observe({ requestId: "redirect", requestHeaders: [] });
  assert.equal(JSON.stringify([...evidence.requests.values()]).includes("private-cookie"), false);
  assert.equal(evidence.takePublic("redirect"), false);
  evidence.observe({ requestId: "expired", requestHeaders: [] });
  now += 60001;
  assert.equal(evidence.takePublic("expired"), false);
  for (let index = 0; index < 513; index++) evidence.observe({ requestId: String(index), requestHeaders: [] });
  assert.ok(evidence.requests.size <= 512);
  assert.equal(evidence.takePublic("512"), false);
  assert.equal(manifestKind({ url: "https://example.com/master.m3u8", method: "GET", tabId: 8, statusCode: 401 }), null);
});
