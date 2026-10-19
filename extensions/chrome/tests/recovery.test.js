import test from 'node:test';
import assert from 'node:assert/strict';
import { recoverPending, SessionLedger } from '../recovery.js';
function fixture({ status = 'accepted', createdNew = true, unreachable = false, cancelFails = false, userPaused = false, complete = false } = {}) {
  const calls = [], data = new Map();
  const record = { id: '12345678-1234-4234-8234-123456789abc', downloadId: 1, phase: 'pending', pausedByUs: !userPaused, userPaused, expiresAtMs: 120000 };
  data.set(record.id, record);
  const ledger = { async put(r) { data.set(r.id, structuredClone(r)); }, async get(id) { return data.get(id); }, async list() { return [...data.values()]; }, async remove(id) { data.delete(id); } };
  const item = { state: complete ? 'complete' : 'in_progress', paused: true };
  const browser = { async get() { return { ...item }; }, async cancel() { calls.push('cancel'); if (cancelFails) throw Error(); item.state = 'interrupted'; }, async resume() { calls.push('resume'); item.paused = false; } };
  const native = { async send(m) { calls.push(m.op); if (unreachable) throw Error(); return m.op === 'abort' ? { status: createdNew ? 'aborted' : 'already_accepted' } : { status, job_id: 4, created_new: createdNew }; } };
  return { native, browser, ledger, data, calls, record, item };
}
test('worker_restart_uses_same_id_and_receipt', async () => { const f = fixture(); const report = await recoverPending(f.browser, f.native, f.ledger, 100); assert.deepEqual(report.resolved, [f.record.id]); assert.equal(f.calls.includes('capture'), false); assert.equal(f.calls.includes('cancel'), true); });
test('accepted_reply_lost_never_resubmits', async () => { const f = fixture(); await recoverPending(f.browser, f.native, f.ledger, 100); assert.deepEqual(f.calls, ['status', 'status', 'cancel']); });
test('unknown_unreachable_status_fails_open', async () => { for (const config of [{ unreachable: true }, { status: 'unknown' }]) { const f = fixture(config); const r = await recoverPending(f.browser, f.native, f.ledger, 100); assert.deepEqual(r.continued, [f.record.id]); assert.equal(f.item.paused, false); assert.equal(f.calls.includes('cancel'), false); } });
test('chrome_cancel_failure_retains_actionable_record', async () => { const f = fixture({ cancelFails: true }); const r = await recoverPending(f.browser, f.native, f.ledger, 100); assert.deepEqual(r.needsAttention, [f.record.id]); assert.equal(f.data.size, 1); });
test('user_pause_is_not_resumed', async () => { const f = fixture({ status: 'unknown', userPaused: true }); await recoverPending(f.browser, f.native, f.ledger, 100); assert.equal(f.calls.includes('resume'), false); });
test('expired_prompt_cannot_submit_later', async () => { const f = fixture({ status: 'pending' }); await recoverPending(f.browser, f.native, f.ledger, 120001); assert.ok(f.calls.includes('abort')); assert.equal(f.calls.includes('capture'), false); assert.equal(f.item.paused, false); });
test('two_simultaneous_requests_remain_distinct', async () => { const f = fixture({ status: 'unknown' }); const id = '12345678-1234-4234-8234-123456789abd'; f.data.set(id, { ...f.record, id, downloadId: 2 }); const r = await recoverPending(f.browser, f.native, f.ledger, 100); assert.equal(new Set(r.continued).size, 2); });
test('completed_browser_files_are_kept_and_existing_jobs_are_not_aborted', async () => { const f = fixture({ complete: true, createdNew: false }); await recoverPending(f.browser, f.native, f.ledger, 100); assert.equal(f.calls.includes('cancel'), false); assert.equal(f.calls.includes('abort'), false); });
test('session_ledger_serializes_simultaneous_writes', async () => { let value = {}; const ledger = new SessionLedger({ async get() { return structuredClone(value); }, async set(v) { value = structuredClone(v); } }); await Promise.all([ledger.put({ id: 'one' }), ledger.put({ id: 'two' })]); assert.equal((await ledger.list()).length, 2); });
