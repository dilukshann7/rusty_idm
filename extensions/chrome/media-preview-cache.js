// Public format metadata stays in this worker's memory, scoped to its tab and document.
export class MediaPreviewCache {
  constructor(now = Date.now) {
    this.now = now;
    this.entries = new Map();
    this.pending = new Map();
    this.active = 0;
  }

  inspect(tabId, documentKey, candidate, load, { refresh = false } = {}) {
    const key = JSON.stringify([tabId, documentKey, candidate.kind, candidate.url]);
    const now = this.now();
    for (const [id, entry] of this.entries)
      if (entry.reply.expires_at_ms <= now) this.entries.delete(id);
    let cached = this.entries.get(key);
    if (refresh) { this.entries.delete(key); cached = null; }
    if (cached) {
      this.entries.delete(key);
      this.entries.set(key, cached);
      if (cached.reply.expires_at_ms - now > 15000 || this.pending.has(key) || this.active >= 2)
        return Promise.resolve(cached.reply);
    }
    const pending = this.pending.get(key);
    if (pending && !pending.discarded) return pending.promise;
    if (this.active >= 2) return Promise.resolve({ status: "declined", code: "busy" });
    const entry = { tabId, discarded: false, promise: null };
    this.active++;
    entry.promise = Promise.resolve().then(load).then(reply => {
      if (entry.discarded) return { status: "declined", code: "expired" };
      if (reply?.status === "media_preview" && reply.expires_at_ms > this.now()) {
        this.entries.delete(key);
        this.entries.set(key, { tabId, reply });
        while (this.entries.size > 64) this.entries.delete(this.entries.keys().next().value);
      }
      return reply;
    }).finally(() => {
      if (this.pending.get(key) === entry) this.pending.delete(key);
      this.active--;
    });
    this.pending.set(key, entry);
    // Keep a valid menu available while refreshing; a cold lookup joins the same flight.
    if (cached) {
      void entry.promise.catch(() => {});
      return Promise.resolve(cached.reply);
    }
    return entry.promise;
  }

  clearTab(tabId) {
    for (const [key, entry] of this.entries) if (entry.tabId === tabId) this.entries.delete(key);
    for (const entry of this.pending.values()) if (entry.tabId === tabId) entry.discarded = true;
  }

  clear() {
    this.entries.clear();
    for (const entry of this.pending.values()) entry.discarded = true;
  }
}
