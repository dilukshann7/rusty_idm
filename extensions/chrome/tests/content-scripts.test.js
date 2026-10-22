import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { runInNewContext } from "node:vm";

// Exercise the actual worker registration routine without starting its native bridge.
const worker = readFileSync(new URL("../service-worker.js", import.meta.url), "utf8");
const routine = worker.slice(worker.indexOf("async function syncScripts()"), worker.indexOf("const native =", worker.indexOf("async function syncScripts()")));
async function sync(existing, permitted = true) {
  const calls = [];
  await runInNewContext(`${routine}\nsyncScripts()`, {
    origins: ["http://*/*", "https://*/*"],
    chrome: {
      permissions: { contains: async () => permitted },
      scripting: {
        getRegisteredContentScripts: async () => existing,
        registerContentScripts: async entries => calls.push(["register", entries]),
        updateContentScripts: async entries => calls.push(["update", entries]),
        unregisterContentScripts: async entries => calls.push(["unregister", entries]),
      },
    },
  });
  return JSON.parse(JSON.stringify(calls));
}
test("new content registration loads toolbar before content in the isolated world", async () => {
  const calls = await sync([]);
  assert.deepEqual(calls[0][1][0].js, ["media-toolbar.js", "content.js"]);
  assert.notEqual(calls[0][1][0].world, "MAIN");
});
test("persisted old registrations are upgraded and permission loss unregisters", async () => {
  const calls = await sync([{ id: "rusty-controls", js: ["content.js"] }]);
  assert.equal(calls[0]?.[0], "update");
  assert.deepEqual(calls[0][1][0].js, ["media-toolbar.js", "content.js"]);
  assert.deepEqual(await sync([{ id: "rusty-controls", js: ["media-toolbar.js", "content.js"] }]), []);
  assert.equal((await sync([{ id: "rusty-controls" }], false))[0][0], "unregister");
});
