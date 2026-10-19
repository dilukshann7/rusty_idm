import test from 'node:test';
import assert from 'node:assert/strict';
import { classifyDownload } from '../capture-policy.js';
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
