const safe = (value) => {
  try {
    const url = new URL(value);
    return ["http:", "https:"].includes(url.protocol) && !url.username && !url.password && value.length <= 8192;
  } catch { return false; }
};

export function youtubePage(value) {
  if (!safe(value)) return null;
  const url = new URL(value);
  if (!["youtube.com", "www.youtube.com", "m.youtube.com"].includes(url.hostname)) return null;
  if (url.pathname === "/watch") {
    const id = url.searchParams.get("v");
    if (/^[A-Za-z0-9_-]{11}$/u.test(id || "")) return `https://www.youtube.com/watch?v=${id}`;
  }
  const match = /^\/shorts\/([A-Za-z0-9_-]{11})\/?$/u.exec(url.pathname);
  return match ? `https://www.youtube.com/shorts/${match[1]}` : null;
}

export function manifestKind(details) {
  if (!safe(details.url) || details.incognito || details.tabId < 0 || details.method !== "GET") return null;
  if (details.statusCode != null && (details.statusCode < 200 || details.statusCode >= 300)) return null;
  const path = new URL(details.url).pathname.toLowerCase();
  const type = (details.responseHeaders || []).find((header) => header.name?.toLowerCase() === "content-type")?.value?.toLowerCase() || "";
  if (path.endsWith(".m3u8") || type.includes("mpegurl")) return "hls";
  if (path.endsWith(".mpd") || type.includes("dash+xml")) return "dash";
  return null;
}

export function canSubmitPreview(owner, tabId, documentKey, variantId, now = Date.now()) {
  return Boolean(owner && owner.tabId === tabId && owner.key === documentKey &&
    owner.expires > now && owner.variants.has(variantId));
}

export class MediaRequestEvidence {
  constructor(now = Date.now) { this.now = now; this.requests = new Map(); this.uncertainUntil = 0; }
  observe(details) {
    const now = this.now();
    for (const [id, value] of this.requests) if (now - value.at > 60000) this.requests.delete(id);
    if (this.requests.size >= 512) { this.requests.clear(); this.uncertainUntil = now + 60000; }
    const credentialed = (details.requestHeaders || []).some(header =>
      ["cookie", "authorization", "proxy-authorization"].includes(header.name?.toLowerCase()));
    const previous = this.requests.get(details.requestId);
    this.requests.set(details.requestId, { at: now, credentialed: credentialed || previous?.credentialed === true });
  }
  takePublic(requestId) {
    const value = this.requests.get(requestId);
    this.requests.delete(requestId);
    return Boolean(value && this.now() >= this.uncertainUntil && this.now() - value.at <= 60000 && !value.credentialed);
  }
}

// Public metadata only. No request headers, cookies, or stream URLs are persisted.
export class MediaObservations {
  constructor(now = Date.now) { this.now = now; this.tabs = new Map(); }
  add(details, document, credentialed = false) {
    const kind = manifestKind(details);
    if (!kind || credentialed || !document || details.timeStamp < document.at || this.now() - details.timeStamp > 60000) return;
    const list = (this.tabs.get(details.tabId) || []).filter((item) => this.now() - item.at <= 60000 && item.documentKey === document.key);
    if (!list.some((item) => item.url === details.url)) list.push({ url: details.url, kind, at: this.now(), documentKey: document.key });
    this.tabs.set(details.tabId, list.slice(-16));
  }
  list(tabId, document, pageUrl) {
    if (!document) return [];
    const items = (this.tabs.get(tabId) || []).filter((item) => this.now() - item.at <= 60000 && item.documentKey === document.key).map(({ url, kind }) => ({ url, kind }));
    const youtube = youtubePage(pageUrl);
    if (youtube) items.unshift({ url: youtube, kind: "youtube" });
    return items;
  }
  clear(tabId) { this.tabs.delete(tabId); }
}
