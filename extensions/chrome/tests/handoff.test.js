import test from 'node:test';
import assert from 'node:assert/strict';
import { handoff } from '../handoff.js';
export function fixture({ pauseFails = false, nativeFails = false, completed = false, cancelFails = false, createdNew = true } = {}) {
  const records = new Map(), events = [];
  const item = { id: 1, state: 'in_progress', paused: false };
  const ledger = { async put(r) { records.set(r.id, structuredClone(r)); events.push(r.phase); }, async get(id) { return records.get(id); }, async list() { return [...records.values()]; }, async remove(id) { records.delete(id); } };
  const browser = { async get() { return { ...item }; }, async pause() { if (pauseFails) throw Error(); item.paused = true; events.push('pause'); }, async cancel() { events.push('cancel'); if (cancelFails) throw Error(); item.state = 'interrupted'; item.paused = false; }, async resume() { events.push('resume'); item.paused = false; } };
  const native = { async send(m) { events.push(m.op); if (nativeFails) throw Error(); if (m.op === 'abort') return { v: 1, id: m.id, status: 'aborted' }; if (completed && m.op === 'capture') item.state = 'complete'; return { v: 1, id: m.id, status: 'accepted', job_id: 7, created_new: createdNew }; } };
  return { ledger, browser, native, records, events, item };
}
const request = { id: '12345678-1234-4234-8234-123456789abc', url: 'https://example.org/file', source: 'automatic', download_id: 1, expires_at_ms: 120000 };
test('pause_failure_never_contacts_native_host', async () => { const f = fixture({ pauseFails: true }); assert.equal(await handoff(1, request, f.browser, f.native, f.ledger, () => 0), 'continued_in_chrome'); assert.equal(f.events.includes('capture'), false); });
test('actor_accepted_is_recorded_before_browser_cancel', async () => { const f = fixture(); assert.equal(await handoff(1, request, f.browser, f.native, f.ledger, () => 0), 'accepted'); assert.ok(f.events.indexOf('accepted') < f.events.indexOf('cancel')); assert.ok(f.events.indexOf('status') < f.events.indexOf('cancel')); });
test('missing_host_resumes_only_our_pause', async () => { const f = fixture({ nativeFails: true }); assert.equal(await handoff(1, request, f.browser, f.native, f.ledger, () => 0), 'continued_in_chrome'); assert.equal(f.item.paused, false); });
test('completed_chrome_file_is_never_removed', async () => { const f = fixture({ completed: true }); await handoff(1, request, f.browser, f.native, f.ledger, () => 0); assert.equal(f.item.state, 'complete'); assert.equal(f.events.includes('cancel'), false); assert.ok(f.events.includes('abort')); });
test('cancel_failure_retains_actionable_record', async () => { const f = fixture({ cancelFails: true }); assert.equal(await handoff(1, request, f.browser, f.native, f.ledger, () => 0), 'needs_attention'); assert.equal(f.records.size, 1); });
