import test from "node:test";
import assert from "node:assert/strict";
import { NativeClient, validReply } from "../native-client.js";

const id = "11111111-1111-4111-8111-111111111111";
const preview = { v: 2, id, status: "media_preview", preview_id: id, expires_at_ms: Date.now() + 120000,
  title: "Sample", variants: [{ id: "137+140", quality: "1080p", container: "mp4", audio: "mp4a", size: 1000 }] };

test("version 2 format previews are bounded and version 1 capture stays valid", () => {
  assert.equal(validReply(preview), true);
  assert.equal(validReply({ ...preview, v: 1 }), false);
  assert.equal(validReply({ ...preview, variants: Array(25).fill(preview.variants[0]) }), false);
  assert.equal(validReply({ ...preview, variants: [{ ...preview.variants[0], size: -1 }] }), false);
  assert.equal(validReply({ v: 1, id, status: "accepted", job_id: 3, created_new: true }), true);
});

test("native media inspect uses protocol version 2 and keeps its request id", async () => {
  let receive;
  const sent = [];
  const port = {
    onMessage: { addListener(callback) { receive = callback; } },
    onDisconnect: { addListener() {} },
    postMessage(message) { sent.push(message); queueMicrotask(() => receive({ ...preview, id: message.id, preview_id: message.id })); },
  };
  const client = new NativeClient({ connectNative: () => port });
  const result = await client.send({ id, op: "media_inspect", payload: { kind: "youtube", url: "https://www.youtube.com/watch?v=abcdefghijk" } });
  assert.equal(sent[0].v, 2);
  assert.equal(sent[0].id, id);
  assert.equal(result.status, "media_preview");
});
