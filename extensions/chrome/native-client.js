import { HOST } from './browser-config.js';
export { HOST };
const uuid = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/u;
const codes = new Set(['invalid_request', 'unsupported', 'busy', 'user_cancelled', 'browser_fallback', 'expired', 'app_closing', 'engine_unavailable', 'recovery_required']);
export function validReply(reply) {
  if (!reply || reply.v !== 1 || !uuid.test(reply.id) || new TextEncoder().encode(JSON.stringify(reply)).length > 65536) return false;
  const fields = { pending: [], accepted: ['job_id', 'created_new'], declined: ['code'], unknown: [], aborted: [], already_accepted: ['job_id', 'created_new'], ok: ['host_version', 'gui_running'], policy: ['browser', 'policy'] }[reply.status];
  if (!fields || Object.keys(reply).some(k => !['v', 'id', 'status', ...fields].includes(k)) || fields.some(k => !(k in reply))) return false;
  if (['accepted', 'already_accepted'].includes(reply.status)) return Number.isSafeInteger(reply.job_id) && reply.job_id > 0 && typeof reply.created_new === 'boolean';
  if (reply.status === 'declined') return codes.has(reply.code);
  if (reply.status === 'policy') return ['chrome', 'brave', 'edge', 'firefox'].includes(reply.browser) && reply.policy && Number.isSafeInteger(reply.policy.revision) && Array.isArray(reply.policy.browsers) && Array.isArray(reply.policy.extensions) && Array.isArray(reply.policy.excluded_sites) && Array.isArray(reply.policy.excluded_addresses);
  if (reply.status === 'ok') return typeof reply.host_version === 'string' && typeof reply.gui_running === 'boolean';
  return true;
}
export class NativeClient {
  constructor(runtime, onDisconnect = () => {}) { this.runtime = runtime; this.onDisconnect = onDisconnect; this.pending = new Map(); this.port = null; }
  connect() {
    if (this.port) return this.port;
    const port = this.runtime.connectNative(HOST); this.port = port;
    port.onMessage.addListener(reply => {
      if (!validReply(reply)) { this.failPort(); return; }
      const call = this.pending.get(reply.id); if (!call) return;
      if (reply.status === 'pending') { clearTimeout(call.connectTimer); call.onPending?.(reply); return; }
      call.finish(null, reply);
    });
    port.onDisconnect.addListener(() => { void this.runtime.lastError; if (this.port === port) this.port = null; for (const call of [...this.pending.values()]) call.finish(Error('Native connection unavailable')); this.onDisconnect(); });
    return port;
  }
  failPort() { const port = this.port; this.port = null; try { port?.disconnect(); } catch {} for (const call of [...this.pending.values()]) call.finish(Error('Invalid native response')); }
  send(operation, { onPending, signal } = {}) {
    const message = { v: 1, id: operation.id || crypto.randomUUID(), op: operation.op, payload: operation.payload || {} };
    if (!uuid.test(message.id) || new TextEncoder().encode(JSON.stringify(message)).length > 65536) return Promise.reject(Error('Invalid native request'));
    const previous = this.pending.get(message.id);
    if (previous) return previous.message === JSON.stringify(message) ? previous.promise : Promise.reject(Error('Capture identifier conflict'));
    if (this.pending.size >= 64 || signal?.aborted) return Promise.reject(Error('Native request unavailable'));
    let resolve, reject; const promise = new Promise((a, b) => { resolve = a; reject = b; });
    const call = { promise, message: JSON.stringify(message), onPending, finish: (error, reply) => { if (!this.pending.delete(message.id)) return; clearTimeout(call.connectTimer); clearTimeout(call.leaseTimer); signal?.removeEventListener('abort', abort); error ? reject(error) : resolve(reply); } };
    const abort = () => call.finish(Error('Native request expired'));
    call.connectTimer = setTimeout(abort, 10000);
    const remaining = message.op === 'capture' ? Math.max(0, Math.min(120000, message.payload.expires_at_ms - Date.now())) : 10000;
    call.leaseTimer = setTimeout(abort, remaining);
    this.pending.set(message.id, call); signal?.addEventListener('abort', abort, { once: true });
    try { this.connect().postMessage(message); } catch { call.finish(Error('Native host unavailable')); }
    return promise;
  }
}
