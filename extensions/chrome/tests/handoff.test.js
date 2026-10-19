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
test('storage_failure_after_successful_cancel_never_aborts_the_accepted_job', async () => {
  const f = fixture(); const save = f.ledger.put;
  f.ledger.put = async record => { if (record.phase === 'browser_cancelled') throw Error('storage unavailable'); return save(record); };
  const cancel = f.browser.cancel; f.browser.cancel = async id => { await cancel(id); f.item.error = 'USER_CANCELED'; };
  assert.equal(await handoff(1, request, f.browser, f.native, f.ledger, () => 0), 'needs_attention');
  assert.equal(f.events.includes('abort'), false);
  assert.equal(f.records.get(request.id).cancelRequested, true);
});
test('host_pending_does_not_cancel_chrome', async () => {
  const f = fixture(); f.native.send = async () => ({ status: 'pending' });
  assert.equal(await handoff(1, request, f.browser, f.native, f.ledger, () => 0), 'needs_attention');
  assert.equal(f.events.includes('cancel'), false); assert.equal(f.item.paused, true);
});
test('user_pause_is_preserved_when_acceptance_arrives', async () => {
  const f = fixture(); const send = f.native.send;
  f.native.send = async operation => { const reply = await send(operation); if (operation.op === 'capture') { const record = f.records.get(request.id); record.pausedByUs = false; record.userPaused = true; } return reply; };
  await handoff(1, request, f.browser, f.native, f.ledger, () => 0);
  assert.equal(f.item.paused, true); assert.equal(f.events.includes('cancel'), false); assert.equal(f.events.includes('resume'), false);
});
test('status_after_owned_abort_cannot_cancel_chrome_using_an_old_receipt', async () => {
  const f = fixture(); const send = f.native.send;
  f.native.send = async operation => operation.op === 'status' ? { status: 'declined', code: 'browser_fallback' } : send(operation);
  await handoff(1, request, f.browser, f.native, f.ledger, () => 0);
  assert.equal(f.events.includes('cancel'), false); assert.equal(f.item.paused, false);
});
test('search_failure_after_pause_still_recovers_our_paused_download', async () => {
  const f = fixture(); let searches = 0; const get = f.browser.get;
  f.browser.get = async id => { if (++searches === 2) throw Error('search unavailable'); return get(id); };
  assert.equal(await handoff(1, request, f.browser, f.native, f.ledger, () => 0), 'continued_in_chrome');
  assert.equal(f.item.paused, false); assert.equal(f.events.includes('capture'), false);
});
test('cancel_rejection_after_the_browser_already_cancelled_keeps_rusty_responsibility', async () => {
  const f = fixture();
  f.browser.cancel = async () => { f.events.push('cancel'); Object.assign(f.item, { state: 'interrupted', paused: false, error: 'USER_CANCELED' }); throw Error('acknowledgement lost'); };
  assert.equal(await handoff(1, request, f.browser, f.native, f.ledger, () => 0), 'needs_attention');
  assert.equal(f.events.includes('abort'), false); assert.equal(f.records.size, 1);
});
test('capture_uuid_is_only_in_the_envelope_and_not_the_payload', async () => {
  const f = fixture(); const send = f.native.send;
  f.native.send = async message => { if (message.op === 'capture') { assert.equal(message.id, request.id); assert.equal(Object.hasOwn(message.payload, 'id'), false); } return send(message); };
  assert.equal(await handoff(1, request, f.browser, f.native, f.ledger, () => 0), 'accepted');
});
