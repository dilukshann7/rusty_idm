// Isolated extension world; pages cannot provide modifier evidence or native messages.
(() => {
  const api = globalThis.browser || globalThis.chrome;
  const token = crypto.randomUUID();
  let policy = {},
    stopped = false;
  const panels = new Map();
  const dismissed = new WeakSet();
  async function refresh() {
    try {
      const next =
        (await api.runtime.sendMessage({
          type: "page-policy",
          documentKey: token,
        })) || {};
      if (stopped) return;
      if (JSON.stringify(next.panels) !== JSON.stringify(policy.panels)) {
        for (const entry of panels.values()) entry.dispose();
        panels.clear();
      }
      policy = next;
    } catch {
      policy = {};
    }
    if (!stopped) render();
  }
  function intent(event) {
    if (!event.isTrusted || !policy.documentKey) return;
    const keys =
      (event.ctrlKey ? 1 : 0) |
      (event.altKey ? 2 : 0) |
      (event.shiftKey ? 4 : 0);
    void api.runtime
      .sendMessage({
        type: "modifier-intent",
        keys,
        at: Date.now(),
        documentKey: token,
      })
      .catch(() => {});
  }
  for (const name of ["keydown", "keyup", "pointerdown", "click"])
    addEventListener(name, intent, true);
  function render() {
    const settings = policy.panels;
    for (const [element, entry] of panels)
      if (
        !element.isConnected ||
        !entry.root.isConnected ||
        !settings?.enabled
      ) {
        entry.dispose();
        panels.delete(element);
      }
    if (!settings?.enabled) return;
    for (const media of document.querySelectorAll("video,audio")) {
      if (
        panels.size >= 32 ||
        panels.has(media) ||
        dismissed.has(media) ||
        (media.tagName === "VIDEO" ? !settings.video : !settings.audio)
      )
        continue;
      const sourceUrl = () => media.currentSrc || media.src || media.querySelector("source")?.src || "";
      const mediaKey = () => `${globalThis.location?.href || ""}\n${sourceUrl()}`;
      const src = sourceUrl();
      let extension;
      try {
        extension = new URL(src).pathname.split(".").pop().toLowerCase();
      } catch {
        extension = "";
      }
      const supported =
        media.tagName === "VIDEO"
          ? ["mp4", "mkv", "webm", "mov", "avi", "m4v"]
          : ["mp3", "wav", "ogg", "m4a", "flac", "aac", "opus"];
      const direct = supported.includes(extension) && /^https?:/u.test(src || "");
      if (!direct && media.tagName !== "VIDEO") continue;
      const toolbar = globalThis.RustyMediaToolbar.create(media, {
        hoverOnly: settings.hover_only,
        onDismiss: () => { dismissed.add(media); panels.get(media)?.dispose(); panels.delete(media); },
      });
      const { root, button, sourceChoice, detail } = toolbar;
      let candidates = [], preview = null, previewKey = "", previewSource = "";
      let contextKey = mediaKey(), sequence = 0, disposed = false, submitting = false;
      let pending = null, nextPoll = 0;
      const receipts = new Map(), accepted = new Set(), listeners = [];
      const listen = (target, name, handler) => {
        target.addEventListener(name, handler);
        listeners.push(() => target.removeEventListener(name, handler));
      };
      const isDirect = () => {
        try {
          const url = new URL(sourceUrl());
          return /^https?:$/u.test(url.protocol) && supported.includes(url.pathname.split(".").pop().toLowerCase());
        } catch { return false; }
      };
      const valid = () => preview && preview.expires_at_ms > Date.now() && previewKey === mediaKey();
      const candidateKey = candidate => JSON.stringify([candidate.kind, candidate.url]);
      const clearFormats = (invalidate = true) => {
        if (invalidate) sequence++;
        preview = null;
        previewKey = previewSource = "";
        receipts.clear(); accepted.clear();
        toolbar.setFormats("", [], () => {});
        button.title = "Download with Rusty. Drag to move this bar.";
      };
      const eligible = () => {
        const rect = media.getBoundingClientRect();
        return document.visibilityState !== "hidden" && rect.width > 0 && rect.height > 0 &&
          rect.bottom > 0 && rect.top < innerHeight && rect.right > 0 && rect.left < innerWidth &&
          (media.readyState >= 1 || media.paused === false);
      };
      const showFailure = text => {
        detail.textContent = text;
        toolbar.retryButton.hidden = false;
      };
      const markAcceptedRows = () => {
        for (let index = 0; index < (preview?.variants.length || 0); index++)
          if (accepted.has(preview.variants[index].id) && toolbar.formatList.children[index])
            toolbar.formatList.children[index].disabled = true;
      };
      async function submitVariant(variantId, id) {
        if (disposed || submitting) return;
        if (receipts.size && !valid()) {
          detail.textContent = "Rusty could not confirm this download. Check Rusty's main list before retrying.";
          toolbar.retryButton.hidden = true;
          return;
        }
        if (!valid() || preview.preview_id !== id || !preview.variants.some(variant => variant.id === variantId)) {
          clearFormats();
          detail.textContent = "Video or format preview changed. Preparing formats again...";
          void prepare({ attention: true, force: true });
          return;
        }
        if (accepted.has(variantId)) return;
        toolbar.closePopover();
        submitting = true;
        toolbar.setBusy(true);
        if (!receipts.has(variantId)) receipts.set(variantId, crypto.randomUUID());
        detail.textContent = "Waiting for Rusty destination choice...";
        try {
          const reply = await api.runtime.sendMessage({ type: "media-submit", documentKey: token,
            previewId: id, variantId, submitId: receipts.get(variantId) });
          if (disposed || preview?.preview_id !== id) return;
          if (reply?.status === "accepted" || reply?.status === "already_accepted") {
            receipts.delete(variantId);
            accepted.add(variantId);
            detail.textContent = "Added to Rusty";
          } else if (reply?.status === "declined") {
            receipts.delete(variantId);
            showFailure("Not added. Choose a quality to retry, or refresh formats.");
          } else {
            detail.textContent = "Rusty did not confirm this download. Choose the same quality to retry.";
            toolbar.retryButton.hidden = true;
          }
        } catch {
          if (!disposed) {
            detail.textContent = "Rusty is unavailable. Choose the same quality to retry.";
            toolbar.retryButton.hidden = true;
          }
        } finally {
          submitting = false;
          toolbar.setBusy(false);
          markAcceptedRows();
        }
      }
      function prepare({ attention = false, force = false, refresh = false } = {}) {
        if (disposed || submitting || isDirect() || (!attention && !eligible())) return;
        const key = mediaKey();
        if (contextKey !== key) {
          clearFormats(); candidates = []; contextKey = key; nextPoll = 0;
        }
        // Preserve the original payload/receipt while acceptance is uncertain.
        if (receipts.size) return;
        if (pending?.key === key && pending.sequence === sequence) return pending.promise;
        if (!force && Date.now() < nextPoll) return;
        nextPoll = Date.now() + 5000;
        const serial = ++sequence;
        const current = () => !disposed && serial === sequence && key === mediaKey();
        if (!valid()) {
          toolbar.setFormats("", [], () => {});
          detail.textContent = "Preparing video formats...";
        }
        toolbar.retryButton.hidden = true;
        const flight = { key, sequence: serial, promise: null };
        flight.promise = (async () => {
          try {
            const reply = await api.runtime.sendMessage({ type: "media-candidates", documentKey: token });
            if (!current() || submitting) return;
            const found = reply?.candidates || [];
            if (!found.length) {
              if (!valid()) showFailure("Play the video to discover public video sources.");
              return;
            }
            const oldChoice = candidates[Number(sourceChoice.value)];
            const selected = found.findIndex(candidate => oldChoice && candidateKey(candidate) === candidateKey(oldChoice));
            const index = selected < 0 ? 0 : selected;
            if (JSON.stringify(found) !== JSON.stringify(candidates)) {
              candidates = found;
              sourceChoice.replaceChildren();
              candidates.forEach((candidate, index) => {
                const option = document.createElement("option");
                option.value = String(index);
                const url = new URL(candidate.url);
                option.textContent = `${candidate.kind.toUpperCase()} - ${url.hostname}${url.pathname}`;
                sourceChoice.append(option);
              });
            }
            sourceChoice.value = String(index);
            sourceChoice.style.display = candidates.length > 1 ? "block" : "none";
            const chosen = candidates[index];
            const chosenKey = candidateKey(chosen);
            if (previewSource && previewSource !== chosenKey) clearFormats(false);
            if (!refresh && valid() && previewSource === chosenKey && preview.expires_at_ms - Date.now() > 15000) return;
            const inspected = await api.runtime.sendMessage({ type: "media-inspect", documentKey: token, url: chosen.url, kind: chosen.kind, refresh });
            if (!current() || submitting || receipts.size) return;
            if (inspected?.status !== "media_preview" || inspected.expires_at_ms <= Date.now() || !inspected.variants?.length) {
              if (!valid()) showFailure(inspected?.code === "busy" ? "Preparing other videos. Formats will retry shortly." : "No supported public formats found. Refresh to try again.");
              nextPoll = Date.now() + 15000;
              return;
            }
            if (preview?.preview_id !== inspected.preview_id) {
              receipts.clear();
              preview = inspected;
              toolbar.setFormats(inspected.title, inspected.variants, id => submitVariant(id, inspected.preview_id));
              markAcceptedRows();
            }
            previewKey = key; previewSource = chosenKey;
            detail.textContent = "Choose a quality to download.";
            toolbar.retryButton.hidden = true;
            button.title = "Video formats ready. Click to choose a quality, or drag to move.";
          } catch {
            if (current() && !valid()) showFailure("Rusty is unavailable. Refresh formats to try again.");
            nextPoll = Date.now() + 15000;
          } finally {
            if (pending === flight) pending = null;
          }
        })();
        pending = flight;
        return flight.promise;
      }
      listen(sourceChoice, "change", () => {
        clearFormats(); nextPoll = 0;
        void prepare({ attention: true, force: true });
      });
      listen(toolbar.retryButton, "click", event => {
        if (event.isTrusted && !submitting) { nextPoll = 0; void prepare({ attention: true, force: true, refresh: true }); }
      });
      listen(button, "click", async event => {
        if (!event.isTrusted || disposed || submitting || button.disabled || toolbar.consumeDragClick()) return;
        if (!isDirect()) {
          if (valid() && candidates.length === 1 && preview.variants.length === 1) {
            await submitVariant(preview.variants[0].id, preview.preview_id);
            return;
          }
          toolbar.openPicker();
          if (!valid()) {
            const key = mediaKey();
            const lookup = prepare({ attention: true, force: true });
            const requestedSequence = sequence;
            void Promise.resolve(lookup).then(() => {
              if (!disposed && !toolbar.picker.hidden && requestedSequence === sequence && key === mediaKey() && valid() && candidates.length === 1 && preview.variants.length === 1)
                return submitVariant(preview.variants[0].id, preview.preview_id);
            });
          }
          return;
        }
        toolbar.closePopover();
        clearFormats();
        sourceChoice.style.display = "none";
        const url = sourceUrl();
        button.disabled = true;
        detail.textContent = "Sending to Rusty...";
        try {
          const reply = await api.runtime.sendMessage({ type: "panel-download", url });
          if (!disposed) detail.textContent = reply?.outcome === "accepted" ? "Added to Rusty" : "Not added - check Rusty extension";
        } catch {
          if (!disposed) detail.textContent = "Rusty is unavailable";
        } finally { button.disabled = false; }
      });
      const sourceChanged = () => {
        clearFormats(); contextKey = ""; nextPoll = 0;
        detail.textContent = "Preparing video formats...";
      };
      for (const name of ["emptied", "loadstart"]) listen(media, name, sourceChanged);
      for (const name of ["loadedmetadata", "play", "playing"]) listen(media, name, () => { void prepare({ force: true }); });
      for (const target of [media, root]) listen(target, "mouseenter", () => { void prepare({ attention: true }); });
      listen(root, "focusin", () => { void prepare({ attention: true }); });
      panels.set(media, {
        root,
        warm: () => { void prepare(); },
        dispose: () => {
          disposed = true; clearFormats();
          for (const remove of listeners) remove();
          toolbar.dispose();
        },
      });
      void prepare();
    }
  }
  const timer = setInterval(refresh, 30000);
  const warmTimer = setInterval(() => { for (const entry of panels.values()) entry.warm(); }, 2000);
  const observer = new MutationObserver(() => render());
  function ready() {
    observer.observe(document.documentElement, {
      childList: true,
      subtree: true,
    });
    void refresh();
  }
  if (document.documentElement) ready();
  else addEventListener("DOMContentLoaded", ready, { once: true });
  addEventListener(
    "pagehide",
    () => {
      stopped = true;
      for (const name of ["keydown", "keyup", "pointerdown", "click"])
        removeEventListener(name, intent, true);
      clearInterval(timer);
      clearInterval(warmTimer);
      observer.disconnect();
      for (const entry of panels.values()) entry.dispose();
      panels.clear();
    },
    { once: true },
  );
})();
