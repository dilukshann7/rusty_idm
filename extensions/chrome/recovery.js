import { nativeStatus, fallback, finishAccepted } from './handoff.js';

export class SessionLedger {
  constructor(storage, onChange = () => {}) { this.storage = storage; this.onChange = onChange; this.queue = Promise.resolve(); }
  transact(action, write = false) {
    const run = async () => {
      const data = await this.storage.get('captureLedger'); const records = data.captureLedger || {};
      const result = await action(records);
      if (write) await this.storage.set({ captureLedger: records });
      return result;
    };
    const result = this.queue.then(run); this.queue = result.catch(() => {});
    if (write) void result.then(() => this.onChange()).catch(() => {});
    return result;
  }
  put(record) {
    return this.transact(records => {
      if (!records[record.id] && Object.keys(records).length >= 32) throw Error('Capture queue full');
      const previous = records[record.id];
      records[record.id] = { ...record, ...(previous?.ownershipLost ? { ownershipLost: true, pausedByUs: false, userPaused: previous.userPaused } : {}) };
    }, true);
  }
  get(id) { return this.transact(records => records[id] && structuredClone(records[id])); }
  list() { return this.transact(records => Object.values(records).map(r => structuredClone(r))); }
  remove(id) { return this.transact(records => { delete records[id]; }, true); }
  update(id, action) { return this.transact(records => { if (records[id]) records[id] = action(records[id]); }, true); }
}

export async function recoverPending(browser, native, ledger, nowMs, { activeIds = new Set() } = {}) {
  const report = { resolved: [], continued: [], needsAttention: [] };
  for (let record of await ledger.list()) {
    if (activeIds.has(record.id)) continue;
    try {
      // No URL is required for status reconciliation. Strip old versions' payloads too.
      if (nowMs >= record.expiresAtMs) {
        const { url, request, filename_hint, ...metadata } = record;
        record = metadata; await ledger.put(record);
      }
      if (record.phase === 'browser_cancelled') { await ledger.remove(record.id); report.resolved.push(record.id); continue; }
      if (record.phase === 'done') {
        const item = record.downloadId == null ? null : await browser.get(record.downloadId);
        if (!item || item.state !== 'in_progress' || !item.paused) { await ledger.remove(record.id); report.resolved.push(record.id); }
        else report.needsAttention.push(record.id);
        continue;
      }
      const status = await nativeStatus(native, record.id);
      if (status.status === 'pending' && nowMs < record.expiresAtMs) continue;
      const outcome = status.status === 'accepted'
        ? await finishAccepted(record, status, browser, native, ledger)
        : await fallback(record, browser, native, ledger);
      report[outcome === 'accepted' ? 'resolved' : outcome === 'continued_in_chrome' ? 'continued' : 'needsAttention'].push(record.id);
    } catch {
      await ledger.put({ ...record, attention: 'recovery_unavailable' }); report.needsAttention.push(record.id);
    }
  }
  return report;
}
