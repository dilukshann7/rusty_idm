import { classifyDownload, Observations, supportedUrl } from './capture-policy.js';
import { NativeClient } from './native-client.js';
import { handoff } from './handoff.js';

class SessionLedger {
  constructor(storage) { this.storage = storage; this.queue = Promise.resolve(); }
  transact(action) { const run = async () => { const data = await this.storage.get('captureLedger'); const records = data.captureLedger || {}; const result = await action(records); await this.storage.set({ captureLedger: records }); return result; }; const result = this.queue.then(run); this.queue = result.catch(() => {}); return result; }
  put(record) { return this.transact(records => { if (!records[record.id] && Object.keys(records).length >= 32) throw Error('Capture queue full'); const previous = records[record.id]; records[record.id] = { ...record, ...(previous?.ownershipLost ? { ownershipLost: true, pausedByUs: false, userPaused: previous.userPaused } : {}) }; }); }
  get(id) { return this.transact(records => records[id] && structuredClone(records[id])); }
  list() { return this.transact(records => Object.values(records).map(r => structuredClone(r))); }
  remove(id) { return this.transact(records => { delete records[id]; }); }
  update(id, action) { return this.transact(records => { if (records[id]) records[id] = action(records[id]); }); }
}
const ledger = new SessionLedger(chrome.storage.session);
const observations = new Observations(chrome.storage.session);
const browser = { get: async id => (await chrome.downloads.search({ id }))[0], pause: id => chrome.downloads.pause(id), cancel: id => chrome.downloads.cancel(id), resume: id => chrome.downloads.resume(id) };
const origins = ['http://*/*', 'https://*/*'];
let preferences = { captureEnabled: false, hostAccess: false };
let recovering = null;
const native = new NativeClient(chrome.runtime, () => { void recover().catch(showFailure); });
function showFailure() { void chrome.action.setBadgeText({ text: '!' }); }
async function showOutcome(outcome) { await chrome.storage.session.set({ lastOutcome: outcome }); await chrome.action.setBadgeText({ text: outcome === 'needs_attention' ? '!' : '' }); }
async function loadPreferences() {
  const local = await chrome.storage.local.get('captureEnabled');
  const hostAccess = await chrome.permissions.contains({ origins });
  preferences = { captureEnabled: local.captureEnabled === true && hostAccess, hostAccess };
  if (!hostAccess && local.captureEnabled) await chrome.storage.local.set({ captureEnabled: false });
  return preferences;
}
async function updateAlarm() {
  const needed = preferences.captureEnabled || (await ledger.list()).length > 0;
  if (needed) { if (!await chrome.alarms.get('rusty-handoff-recovery')) await chrome.alarms.create('rusty-handoff-recovery', { periodInMinutes: 0.5 }); }
  else await chrome.alarms.clear('rusty-handoff-recovery');
}
async function recover() {
  if (recovering) return recovering;
  recovering = (async () => { await observations.clearExpired(); await updateAlarm(); })();
  try { await recovering; } finally { recovering = null; }
}
async function ensureMenu() {
  await chrome.contextMenus.removeAll();
  chrome.contextMenus.create({ id: 'rusty-download-link', title: 'Download with Rusty', contexts: ['link'] }, () => { void chrome.runtime.lastError; });
}
const filter = { urls: origins };
chrome.webRequest.onBeforeRequest.addListener(details => { if (preferences.captureEnabled && preferences.hostAccess) void observations.update(details, 'before').catch(showFailure); }, filter);
chrome.webRequest.onBeforeSendHeaders.addListener(details => { if (preferences.captureEnabled && preferences.hostAccess) void observations.update(details, 'headers').catch(showFailure); }, filter, ['requestHeaders', 'extraHeaders']);
chrome.downloads.onCreated.addListener(item => {
  void (async () => {
    await loadPreferences();
    const result = classifyDownload(item, await observations.list(), preferences, Date.now());
    if (!result.eligible) return;
    await observations.consume(result.observationId);
    const outcome = await handoff(item.id, { ...result.request, id: crypto.randomUUID() }, browser, native, ledger);
    await showOutcome(outcome); await updateAlarm();
  })().catch(showFailure);
});
chrome.downloads.onChanged.addListener(change => {
  if (!change.paused) return;
  void (async () => { for (const record of await ledger.list()) if (record.downloadId === change.id) {
    if (change.paused.current === false) await ledger.update(record.id, r => ({ ...r, pausedByUs: false, ownershipLost: true }));
    else if (record.ownershipLost) await ledger.update(record.id, r => ({ ...r, userPaused: true }));
  } })().catch(showFailure);
});
chrome.contextMenus.onClicked.addListener((info, tab) => {
  if (info.menuItemId !== 'rusty-download-link') return;
  void (async () => {
    if (tab?.incognito || !supportedUrl(info.linkUrl)) { await showOutcome('needs_attention'); return; }
    const outcome = await handoff(null, { id: crypto.randomUUID(), url: info.linkUrl, source: 'context_menu' }, browser, native, ledger);
    await showOutcome(outcome); await updateAlarm();
  })().catch(showFailure);
});
chrome.runtime.onMessage.addListener((message, sender, respond) => {
  if (sender.id !== chrome.runtime.id || sender.url !== chrome.runtime.getURL('popup.html')) return false;
  const action = async () => {
    if (message.type === 'capture-preference') { const access = await chrome.permissions.contains({ origins }); await chrome.storage.local.set({ captureEnabled: message.enabled === true && access }); await loadPreferences(); await updateAlarm(); return { ok: true }; }
    if (message.type === 'recover') { await recover(); return { ok: true }; }
    if (message.type === 'status') {
      await loadPreferences(); let ping;
      try { ping = await native.send({ op: 'ping' }); } catch {}
      const records = await ledger.list(); const last = await chrome.storage.session.get('lastOutcome');
      return { captureEnabled: preferences.captureEnabled, hostAvailable: ping?.status === 'ok', guiRunning: ping?.gui_running === true, pending: records.length, attention: records.filter(r => r.attention).length || (last.lastOutcome === 'needs_attention' ? 1 : 0) };
    }
    return { ok: false };
  };
  action().then(respond, () => respond({ ok: false })); return true;
});
chrome.permissions.onRemoved.addListener(() => { void loadPreferences().then(updateAlarm).catch(showFailure); });
chrome.runtime.onInstalled.addListener(() => { void ensureMenu().then(loadPreferences).then(recover).catch(showFailure); });
chrome.runtime.onStartup.addListener(() => { void loadPreferences().then(recover).catch(showFailure); });
chrome.alarms.onAlarm.addListener(alarm => { if (alarm.name === 'rusty-handoff-recovery') void recover().catch(showFailure); });
void loadPreferences().then(recover).catch(showFailure);
