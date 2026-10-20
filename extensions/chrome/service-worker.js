import { api as chrome, BROWSER } from './browser-api.js';
import { allowed, excluded } from './capture-rules.js';
import { classifyDownload, Observations, supportedUrl, fingerprintReferrer } from './capture-policy.js';
import { NativeClient } from './native-client.js';
import { handoff } from './handoff.js';

import { SessionLedger, recoverPending } from './recovery.js';

const ledger = new SessionLedger(chrome.storage.session, () => updateAlarm().catch(showFailure));
const activeIds = new Set();
const observations = new Observations(chrome.storage.session);
const browser = { get: async id => (await chrome.downloads.search({ id }))[0], pause: id => chrome.downloads.pause(id), cancel: id => chrome.downloads.cancel(id), resume: id => chrome.downloads.resume(id) };
const origins = ['http://*/*', 'https://*/*'];
let preferences = { captureEnabled: false, hostAccess: false };
let recovering = null;
let policy = null, policyAt = 0, policyFlight = null;
const documents = new Map(), intents = new Map();
async function desktopPolicy(force = false) {
  if (!force && policy && Date.now() - policyAt < 2000) return policy;
  if (policyFlight) return policyFlight;
  policyFlight = (async () => {
    try { const reply = await native.send({ op: 'policy', payload: { browser: BROWSER } });
      if (reply.status !== 'policy' || reply.browser !== BROWSER) { policy = null; return null; }
      const changed = !policy || policy.revision !== reply.policy.revision;
      const reconnect = policy && policy.restart_generation !== reply.policy.restart_generation;
      policy = reply.policy; policyAt = Date.now();
      if (changed) await ensureMenu(); if (reconnect) native.failPort(); return policy;
    } catch { policy = null; return null; }
    finally { policyFlight = null; }
  })(); return policyFlight;
}
function documentDetails(details) {
  const entry = documents.get(details.tabId);
  return { ...details, rustyDocumentKey: entry && details.timeStamp >= entry.at ? entry.key : null };
}
function modifiers(candidate) {
  const history = intents.get(candidate?.tabId) || [];
  const intent = [...history].reverse().find(i => i.documentKey === candidate.documentKey && i.at <= candidate.startedAt && Date.now() - i.at <= 2000);
  if (!intent || !candidate.documentKey) return { prevented: false, forced: false };
  const matches = mask => mask !== 0 && (intent.keys & mask) === mask;
  return { prevented: matches(policy.keys.prevent), forced: matches(policy.keys.force) };
}
chrome.webNavigation.onCommitted.addListener(details => { if (details.frameId === 0) { documents.delete(details.tabId); intents.delete(details.tabId); } });
chrome.tabs.onRemoved.addListener(id => { documents.delete(id); intents.delete(id); });
async function syncScripts() {
  const permitted = await chrome.permissions.contains({ origins });
  const scripts = await chrome.scripting.getRegisteredContentScripts({ ids: ['rusty-controls'] });
  if (permitted && scripts.length === 0) await chrome.scripting.registerContentScripts([{ id: 'rusty-controls', matches: origins, js: ['content.js'], runAt: 'document_start', allFrames: false, persistAcrossSessions: true }]);
  if (!permitted && scripts.length) await chrome.scripting.unregisterContentScripts({ ids: ['rusty-controls'] });
}
const native = new NativeClient(chrome.runtime, () => { void recover().catch(showFailure); });
function showFailure() { void chrome.action.setBadgeText({ text: '!' }); }
async function showOutcome(outcome) { await chrome.storage.session.set({ lastOutcome: outcome }); await chrome.action.setBadgeText({ text: outcome === 'needs_attention' ? '!' : '' }); }
async function loadPreferences() {
  const local = await chrome.storage.local.get('captureEnabled');
  const hostAccess = await chrome.permissions.contains({ origins });
  preferences = { captureEnabled: local.captureEnabled === true && hostAccess, hostAccess };
  if (!hostAccess && local.captureEnabled) await chrome.storage.local.set({ captureEnabled: false });
  await syncScripts();
  return preferences;
}
async function updateAlarm() {
  const needed = preferences.captureEnabled || (await ledger.list()).length > 0;
  if (needed) { if (!await chrome.alarms.get('rusty-handoff-recovery')) await chrome.alarms.create('rusty-handoff-recovery', { periodInMinutes: 0.5 }); }
  else await chrome.alarms.clear('rusty-handoff-recovery');
}
async function recover() {
  if (recovering) return recovering;
  recovering = (async () => { await observations.clearExpired(); const report = await recoverPending(browser, native, ledger, Date.now(), { activeIds }); if (report.needsAttention.length) await showOutcome('needs_attention'); else if (report.resolved.length || report.continued.length) await showOutcome('recovered'); await updateAlarm(); })();
  try { await recovering; } finally { recovering = null; }
}
async function ensureMenu() {
  await chrome.contextMenus.removeAll();
  const menus = policy?.menus || { links: true, selection: false, media: false };
  for (const [id, title, contexts, enabled] of [
    ['rusty-download-link', 'Download with Rusty', ['link'], menus.links],
    ['rusty-download-selection', 'Download selected URL with Rusty', ['selection'], menus.selection],
    ['rusty-download-media', 'Download media with Rusty', ['audio', 'video'], menus.media]]) {
    if (enabled) chrome.contextMenus.create({ id, title, contexts }, () => { void chrome.runtime.lastError; });
  }
}
const filter = { urls: origins };
chrome.webRequest.onBeforeRequest.addListener(details => { if (preferences.captureEnabled && preferences.hostAccess) void observations.update(documentDetails(details), 'before').catch(showFailure); }, filter);
chrome.webRequest.onBeforeSendHeaders.addListener(details => { if (preferences.captureEnabled && preferences.hostAccess) void observations.update(documentDetails(details), 'headers').catch(showFailure); }, filter, BROWSER === 'firefox' ? ['requestHeaders'] : ['requestHeaders', 'extraHeaders']);
chrome.downloads.onCreated.addListener(item => {
  void (async () => {
    await loadPreferences();
    const referrerDigest = item.referrer ? await fingerprintReferrer(item.referrer) : null;
    if (!await desktopPolicy()) return;
    const evidence = await observations.list();
    const result = classifyDownload(item, evidence, { ...preferences, referrerDigest }, Date.now());
    if (!result.eligible) return;
    const intent = modifiers(evidence.find(o => o.requestId === result.observationId));
    if (intent.prevented || !allowed(policy, BROWSER, item, intent.forced)) return;
    Object.assign(result.request, { forced: intent.forced, original_url: item.url, policy_revision: policy.revision, browser: BROWSER });
    await observations.consume(result.observationId);
    const id = crypto.randomUUID(); activeIds.add(id); let outcome;
    try { outcome = await handoff(item.id, { ...result.request, id }, browser, native, ledger); } finally { activeIds.delete(id); }
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
async function explicitDownload(url, tab) {
  if (tab?.incognito || !supportedUrl(url)) { await showOutcome('needs_attention'); return; }
  const id = crypto.randomUUID(); activeIds.add(id);
  try { await showOutcome(await handoff(null, { id, url, source: 'context_menu', browser: BROWSER }, browser, native, ledger)); }
  finally { activeIds.delete(id); }
  await updateAlarm();
}
chrome.contextMenus.onClicked.addListener((info, tab) => {
  const url = info.menuItemId === 'rusty-download-link' ? info.linkUrl : info.menuItemId === 'rusty-download-selection' ? info.selectionText?.trim() : info.menuItemId === 'rusty-download-media' ? info.srcUrl : null;
  if (url) void explicitDownload(url, tab).catch(showFailure);
});
chrome.runtime.onMessage.addListener((message, sender, respond) => {
  if (sender.id !== chrome.runtime.id) return false;
  if (sender.tab && sender.frameId === 0 && supportedUrl(sender.url) && !sender.tab.incognito) {
    const action = async () => {
      if (message.type === 'page-policy' && typeof message.documentKey === 'string' && message.documentKey.length <= 80) {
        if (documents.size >= 128 && !documents.has(sender.tab.id)) return {};
        const key = sender.documentId || message.documentKey;
        const current = documents.get(sender.tab.id);
        if (!current || current.key !== key) { documents.set(sender.tab.id, { key, at: Date.now() }); intents.delete(sender.tab.id); }
        const p = await desktopPolicy();
        return p && !excluded(p, sender.url) ? { keys: p.keys, panels: p.panels, documentKey: key } : {};
      }
      if (message.type === 'modifier-intent' && Number.isInteger(message.keys) && message.keys >= 0 && message.keys <= 7) {
        const document = documents.get(sender.tab.id);
        if (document?.key === (sender.documentId || message.documentKey) && Number.isFinite(message.at) && Math.abs(Date.now() - message.at) <= 2000) {
          const history = intents.get(sender.tab.id) || [];
          history.push({ documentKey: document.key, keys: message.keys, at: message.at }); intents.set(sender.tab.id, history.slice(-16));
        }
        return {};
      }
      if (message.type === 'panel-download' && policy?.panels.enabled && !excluded(policy, sender.url)) {
        const url = message.url;
        const extension = supportedUrl(url) ? new URL(url).pathname.split('.').pop().toLowerCase() : '';
        if ((policy.panels.video && ['mp4','mkv','webm','mov','avi','m4v'].includes(extension)) || (policy.panels.audio && ['mp3','wav','ogg','m4a','flac','aac','opus'].includes(extension))) await explicitDownload(url, sender.tab);
        return {};
      }
      return {};
    };
    action().then(respond, () => respond({})); return true;
  }
  if (sender.url !== chrome.runtime.getURL('popup.html')) return false;
  const action = async () => {
    if (message.type === 'capture-preference') { const access = await chrome.permissions.contains({ origins }); await chrome.storage.local.set({ captureEnabled: message.enabled === true && access }); await loadPreferences(); if (!preferences.captureEnabled) await observations.clear(); await updateAlarm(); return { ok: true }; }
    if (message.type === 'recover') { await recover(); return { ok: true }; }
    if (message.type === 'status') {
      await loadPreferences(); await desktopPolicy(true); let ping;
      try { ping = await native.send({ op: 'ping' }); } catch {}
      const records = await ledger.list(); const last = await chrome.storage.session.get('lastOutcome');
      return { captureEnabled: preferences.captureEnabled, hostAvailable: ping?.status === 'ok', guiRunning: ping?.gui_running === true, pending: records.length, attention: records.filter(r => r.attention).length || (last.lastOutcome === 'needs_attention' ? 1 : 0) };
    }
    return { ok: false };
  };
  action().then(respond, () => respond({ ok: false })); return true;
});
chrome.permissions.onRemoved.addListener(() => { void loadPreferences().then(async () => { if (!preferences.captureEnabled) await observations.clear(); await updateAlarm(); }).catch(showFailure); });
chrome.runtime.onInstalled.addListener(() => { void ensureMenu().then(loadPreferences).then(recover).catch(showFailure); });
chrome.runtime.onStartup.addListener(() => { void loadPreferences().then(recover).catch(showFailure); });
chrome.alarms.onAlarm.addListener(alarm => { if (alarm.name === 'rusty-handoff-recovery') void recover().catch(showFailure); });
void loadPreferences().then(recover).catch(showFailure);
