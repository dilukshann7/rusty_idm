export function supportedUrl(value) {
  try { const url = new URL(value); return ['http:', 'https:'].includes(url.protocol) && !url.username && !url.password && !/[\u0000-\u001f\u007f]/u.test(value) && new TextEncoder().encode(value).length <= 8192; } catch { return false; }
}
export function classifyDownload(item, observations, preferences, nowMs) {
  const reject = reason => ({ eligible: false, reason });
  if (!preferences.captureEnabled || !preferences.hostAccess) return reject('capture_off');
  if (item.incognito || item.byExtensionId || item.state !== 'in_progress' || item.paused || item.danger !== 'safe') return reject('browser_owned');
  if (!supportedUrl(item.url) || !supportedUrl(item.finalUrl || item.url)) return reject('unsupported');
  const started = Date.parse(item.startTime);
  if (!Number.isFinite(started)) return reject('missing_metadata');
  // Count unsafe candidates too: a simultaneous POST must make the GET ambiguous.
  const candidates = observations.filter(o => nowMs - o.lastAt <= 30000 && Math.abs(o.startedAt - started) <= 10000 &&
    o.urls?.includes(item.url) && o.urls?.includes(item.finalUrl || item.url) &&
    (!item.referrer || (preferences.referrerDigest && o.referrerDigest === preferences.referrerDigest)));
  if (candidates.length !== 1) return reject('ambiguous_metadata');
  const candidate = candidates[0];
  if (!candidate.allGet || !candidate.headersObserved || candidate.credentials || candidate.incognito || candidate.consumed) return reject('unsafe_request');
  const hint = (item.filename || '').split(/[\\/]/u).pop();
  return { eligible: true, reason: 'eligible', request: { url: item.finalUrl || item.url, source: 'automatic', download_id: item.id,
    ...(hint && hint.length <= 255 && !/[\u0000-\u001f\u007f]/u.test(hint) ? { filename_hint: hint } : {}) }, observationId: candidate.requestId };
}

// Header values are never copied. Each redirect hop needs its own observed headers.
export class Observations {
  constructor(storage, now = Date.now) { this.storage = storage; this.now = now; this.queue = Promise.resolve(); this.pending = 0; this.uncertainUntil = 0; }
  update(details, stage) {
    if (this.pending >= 128) { this.uncertainUntil = this.now() + 30000; return Promise.resolve(); }
    // Reduce headers immediately: the queued operation must not retain raw values.
    const summary = { requestId: details.requestId, url: supportedUrl(details.url) ? details.url : '', method: details.method, timeStamp: details.timeStamp, incognito: !!details.incognito };
    summary.credentials = (details.requestHeaders || []).some(h => ['cookie', 'authorization', 'proxy-authorization'].includes(h.name.toLowerCase()));
    const referrer = (details.requestHeaders || []).find(h => h.name.toLowerCase() === 'referer')?.value;
    const digest = referrer ? fingerprintReferrer(referrer).catch(() => null) : Promise.resolve(null);
    details = null;
    this.pending++;
    const operation = async () => {
      const now = this.now(); const result = await this.storage.get('requestObservations');
      const all = (result.requestObservations || []).filter(o => now - o.lastAt <= 30000);
      let chain = all.find(o => o.requestId === summary.requestId);
      if (!chain && stage !== 'before') return;
      if (!chain) { chain = { requestId: summary.requestId, urls: [], startedAt: summary.timeStamp, lastAt: now, allGet: true, headersObserved: true, credentials: false, incognito: summary.incognito, referrerDigest: null, hopHeaders: false }; all.push(chain); }
      if (stage === 'before') {
        if (chain.urls.length && !chain.hopHeaders) chain.headersObserved = false;
        chain.hopHeaders = false;
        chain.allGet &&= summary.method === 'GET';
        if (!chain.urls.includes(summary.url)) chain.urls.push(summary.url);
        // Reject pathological redirect chains without retaining unbounded URL metadata.
        if (chain.urls.length > 16 || !summary.url) { chain.allGet = false; chain.urls = chain.urls.slice(0, 16); }
      }
      if (stage === 'headers') {
        chain.hopHeaders = true;
        chain.credentials ||= summary.credentials;
        // Referrer is only an exact correlation signal, never sent to the desktop.
        if (chain.urls.length === 1) chain.referrerDigest = await digest;
      }
      chain.lastAt = now;
      await this.storage.set({ requestObservations: all.slice(-64) });
    };
    const result = this.queue.then(operation).finally(() => { this.pending--; }); this.queue = result.catch(() => { this.uncertainUntil = this.now() + 30000; }); return result;
  }
  async list() { await this.queue; if (this.now() < this.uncertainUntil) return []; const r = await this.storage.get('requestObservations'); return (r.requestObservations || []).filter(o => this.now() - o.lastAt <= 30000).map(o => ({ ...o, headersObserved: o.headersObserved && o.hopHeaders })); }
  consume(id) { const operation = async () => { const r = await this.storage.get('requestObservations'); await this.storage.set({ requestObservations: (r.requestObservations || []).map(o => o.requestId === id ? { ...o, consumed: true } : o) }); }; const result = this.queue.then(operation); this.queue = result.catch(() => {}); return result; }
  clear() { const result = this.queue.then(() => this.storage.remove('requestObservations')); this.queue = result.catch(() => {}); return result; }
  clearExpired() { const operation = async () => { const r = await this.storage.get('requestObservations'); await this.storage.set({ requestObservations: (r.requestObservations || []).filter(o => this.now() - o.lastAt <= 30000) }); }; const result = this.queue.then(operation); this.queue = result.catch(() => {}); return result; }
}

export async function fingerprintReferrer(value) { const bytes = await crypto.subtle.digest('SHA-256', new TextEncoder().encode(value)); return Array.from(new Uint8Array(bytes), b => b.toString(16).padStart(2, '0')).join(''); }
