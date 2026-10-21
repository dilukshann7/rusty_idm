import test from 'node:test';
import assert from 'node:assert/strict';
import { classifyDownload, Observations } from '../capture-policy.js';
const now = 100000;
const item = { id: 1, url: 'https://example.org/a', finalUrl: 'https://example.org/a', state: 'in_progress', danger: 'safe', paused: false, incognito: false, referrer: '', startTime: new Date(now).toISOString(), filename: 'C:\\Downloads\\a.zip' };
const chain = { requestId: 'one', urls: [item.url], startedAt: now, lastAt: now, allGet: true, headersObserved: true, credentials: false, referrer: '' };
const classify = (i = item, observations = [chain]) => classifyDownload(i, observations, { captureEnabled: true, hostAccess: true }, now);
test('keeps_post_blob_incognito_and_ambiguous_requests_in_chrome', () => {
  assert.equal(classify().eligible, true);
  for (const bad of [{ ...chain, allGet: false }, { ...chain, credentials: true }]) assert.equal(classify(item, [bad]).eligible, false);
  assert.equal(classify({ ...item, incognito: true }).eligible, false);
  assert.equal(classify({ ...item, url: 'blob:https://example.org/a' }).eligible, false);
  assert.equal(classify(item, [chain, { ...chain, requestId: 'two', allGet: false, tabId: 2 }]).eligible, false);
});
test('missing_header_observation_is_not_assumed_safe', () => assert.equal(classify(item, [{ ...chain, headersObserved: false }]).eligible, false));
test('queries_are_preserved_and_paths_are_not_filename_hints', () => {
  const url = 'https://example.org/a?signature=private';
  const result = classify({ ...item, url, finalUrl: url }, [{ ...chain, urls: [url] }]);
  assert.equal(result.request.url, url); assert.equal(result.request.filename_hint, 'a.zip');
});
test('observation_backpressure_fails_open_and_never_stores_credential_values', async () => {
  let stored = {}; const storage = { async get() { return structuredClone(stored); }, async set(value) { stored = structuredClone(value); } };
  const observations = new Observations(storage, () => now);
  const jobs = [];
  for (let i = 0; i < 130; i++) jobs.push(observations.update({ requestId: String(i), url: item.url, method: 'GET', timeStamp: now }, 'before'));
  await Promise.all(jobs);
  assert.deepEqual(await observations.list(), []);
  await observations.update({ requestId: '1', requestHeaders: [{ name: 'Cookie', value: 'secret-cookie' }] }, 'headers');
  assert.equal(JSON.stringify(stored).includes('secret-cookie'), false);
});
test('explicit_refresh_can_retrieve_short_lived_headers_without_persisting_values', async () => {
  let stored = {};
  const storage = { async get() { return structuredClone(stored); }, async set(value) { stored = structuredClone(value); } };
  const observations = new Observations(storage, () => now);
  await observations.update({ requestId: 'refresh', url: item.url, method: 'GET', timeStamp: now }, 'before');
  await observations.update({ requestId: 'refresh', url: item.url, method: 'GET', requestHeaders: [
    { name: 'Cookie', value: 'private=one' }, { name: 'Referer', value: 'https://example.org/page' },
  ] }, 'headers');
  assert.equal(JSON.stringify(stored).includes('private=one'), false);
  assert.deepEqual(observations.takeContext('refresh'), [{ url: item.url, headers: [
    { name: 'Cookie', value: 'private=one' }, { name: 'Referer', value: 'https://example.org/page' },
  ] }]);
  assert.deepEqual(observations.takeContext('refresh'), []);
  const protectedChain = { ...chain, credentials: true };
  assert.equal(classifyDownload(item, [protectedChain], { captureEnabled: true, hostAccess: true, refreshArmed: true }, now).eligible, true);
  assert.equal(classify(item, [protectedChain]).eligible, false);
});
test('refresh_headers_expire_and_are kept per observed redirect hop', async () => {
  let current = now; let stored = {};
  const storage = { async get() { return structuredClone(stored); }, async set(value) { stored = structuredClone(value); } };
  const observations = new Observations(storage, () => current);
  const first = item.url, second = 'https://files.example.net/new';
  await observations.update({ requestId: 'redirect', url: first, method: 'GET', timeStamp: now }, 'before');
  await observations.update({ requestId: 'redirect', url: first, method: 'GET', requestHeaders: [{ name: 'Cookie', value: 'first-only' }] }, 'headers');
  await observations.update({ requestId: 'redirect', url: second, method: 'GET', timeStamp: now }, 'before');
  await observations.update({ requestId: 'redirect', url: second, method: 'GET', requestHeaders: [{ name: 'Authorization', value: 'Bearer second-only' }, { name: 'Host', value: 'blocked' }] }, 'headers');
  assert.deepEqual(observations.takeContext('redirect'), [
    { url: first, headers: [{ name: 'Cookie', value: 'first-only' }] },
    { url: second, headers: [{ name: 'Authorization', value: 'Bearer second-only' }] },
  ]);
  assert.equal(JSON.stringify(stored).includes('first-only'), false);
  assert.equal(JSON.stringify(stored).includes('second-only'), false);
  await observations.update({ requestId: 'expired', url: first, method: 'GET', timeStamp: now }, 'before');
  await observations.update({ requestId: 'expired', url: first, method: 'GET', requestHeaders: [{ name: 'Cookie', value: 'short-lived' }] }, 'headers');
  current += 30001;
  assert.deepEqual(observations.takeContext('expired'), []);
});
