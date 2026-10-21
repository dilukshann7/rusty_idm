// Isolated extension world; pages cannot provide modifier evidence or native messages.
(() => {
  const api = globalThis.browser || globalThis.chrome;
  const token = crypto.randomUUID();
  let policy = {},
    stopped = false;
  const panels = new Map();
  async function refresh() {
    try {
      const next =
        (await api.runtime.sendMessage({
          type: "page-policy",
          documentKey: token,
        })) || {};
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
        (media.tagName === "VIDEO" ? !settings.video : !settings.audio)
      )
        continue;
      const src =
        media.currentSrc || media.src || media.querySelector("source")?.src;
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
      const root = document.createElement("span");
      const shadow = root.attachShadow({ mode: "closed" });
      const button = document.createElement("button");
      button.textContent = direct ? "Download with Rusty" : "Choose video format in Rusty";
      button.style.cssText =
        "font:12px Tahoma,sans-serif;padding:3px 8px;color:#111;background:#f4f4f4;border:1px solid #aaa;border-radius:3px;cursor:pointer;";
      const sourceChoice = document.createElement("select");
      const formatChoice = document.createElement("select");
      const submit = document.createElement("button");
      const detail = document.createElement("span");
      for (const control of [sourceChoice, formatChoice, submit]) control.style.display = "none";
      for (const control of [sourceChoice, formatChoice, submit, detail]) control.style.font = "12px Tahoma,sans-serif";
      submit.textContent = "Add selected format";
      let candidates = [], preview = null, submissionId = null, inspectSequence = 0;
      const inspect = async () => {
        const chosen = candidates[Number(sourceChoice.value)];
        if (!chosen) return;
        const sequence = ++inspectSequence;
        preview = null;
        submissionId = null;
        formatChoice.style.display = "none";
        submit.style.display = "none";
        detail.textContent = "Checking available formats...";
        try {
          const reply = await api.runtime.sendMessage({ type: "media-inspect", documentKey: token, url: chosen.url, kind: chosen.kind });
          if (sequence !== inspectSequence) return;
          if (reply?.status !== "media_preview" || reply.expires_at_ms <= Date.now()) {
            detail.textContent = "No supported public formats found.";
            return;
          }
          preview = reply;
          submissionId = null;
          formatChoice.replaceChildren();
          for (const variant of reply.variants) {
            const option = document.createElement("option");
            option.value = variant.id;
            option.textContent = `${variant.quality} • ${variant.container} • audio: ${variant.audio} • ${variant.size == null ? "size unknown" : `${(variant.size / 1048576).toFixed(1)} MB`}`;
            formatChoice.append(option);
          }
          detail.textContent = reply.title;
          formatChoice.style.display = "inline-block";
          submit.style.display = "inline-block";
        } catch { if (sequence === inspectSequence) detail.textContent = "Rusty is unavailable."; }
      };
      sourceChoice.addEventListener("change", () => { void inspect(); });
      formatChoice.addEventListener("change", () => { submissionId = null; });
      submit.addEventListener("click", async (event) => {
        if (!event.isTrusted || submit.disabled || !preview || preview.expires_at_ms <= Date.now()) {
          detail.textContent = "Format preview expired. Inspect again.";
          return;
        }
        submit.disabled = true;
        button.disabled = sourceChoice.disabled = formatChoice.disabled = true;
        submissionId ||= crypto.randomUUID();
        detail.textContent = "Waiting for Rusty destination choice...";
        try {
          const reply = await api.runtime.sendMessage({ type: "media-submit", documentKey: token, previewId: preview.preview_id, variantId: formatChoice.value, submitId: submissionId });
          detail.textContent = reply?.status === "accepted" || reply?.status === "already_accepted"
            ? "Added to Rusty" : "Not added. Inspect the video again if it changed.";
          if (reply?.status === "accepted" || reply?.status === "already_accepted") submit.style.display = "none";
        } catch { detail.textContent = "Rusty is unavailable. Try again."; }
        finally { button.disabled = sourceChoice.disabled = formatChoice.disabled = submit.disabled = false; }
      });
      button.addEventListener("click", async (event) => {
        if (!event.isTrusted || button.disabled) return;
        if (!direct) {
          button.disabled = true;
          inspectSequence++;
          preview = null;
          submit.style.display = formatChoice.style.display = "none";
          detail.textContent = "Finding public video sources...";
          try {
            const reply = await api.runtime.sendMessage({ type: "media-candidates", documentKey: token });
            candidates = reply?.candidates || [];
            sourceChoice.replaceChildren();
            candidates.forEach((candidate, index) => {
              const option = document.createElement("option");
              option.value = String(index);
              const url = new URL(candidate.url);
              option.textContent = `${candidate.kind.toUpperCase()} • ${url.hostname}${url.pathname}`;
              sourceChoice.append(option);
            });
            if (!candidates.length) detail.textContent = "No public video source observed on this page.";
            else { sourceChoice.style.display = "inline-block"; await inspect(); }
          } catch { detail.textContent = "Rusty is unavailable."; }
          finally { button.disabled = false; }
          return;
        }
        const url = media.currentSrc || src;
        let extension;
        try {
          extension = new URL(url).pathname.split(".").pop().toLowerCase();
        } catch {}
        if (!supported.includes(extension)) {
          button.textContent = "Unsupported media URL";
          return;
        }
        button.disabled = true;
        button.textContent = "Sending to Rusty...";
        try {
          const reply = await api.runtime.sendMessage({
            type: "panel-download",
            url,
          });
          button.textContent =
            reply?.outcome === "accepted"
              ? "Added to Rusty"
              : "Not added - check Rusty extension";
        } catch {
          button.textContent = "Rusty is unavailable";
        } finally {
          button.disabled = false;
        }
      });
      shadow.append(button, sourceChoice, formatChoice, submit, detail);
      media.insertAdjacentElement("afterend", root);
      let timer;
      const show = () => {
        clearTimeout(timer);
        root.style.opacity = "1";
      };
      const hide = () => {
        timer = setTimeout(() => (root.style.opacity = "0"), 200);
      };
      panels.set(media, {
        root,
        dispose: () => {
          clearTimeout(timer);
          media.removeEventListener("mouseenter", show);
          media.removeEventListener("mouseleave", hide);
          root.remove();
        },
      });
      if (settings.hover_only) {
        root.style.opacity = "0";
        media.addEventListener("mouseenter", show);
        media.addEventListener("mouseleave", hide);
        root.addEventListener("mouseenter", show);
        root.addEventListener("mouseleave", hide);
      }
    }
  }
  const timer = setInterval(refresh, 30000);
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
      clearInterval(timer);
      observer.disconnect();
      for (const entry of panels.values()) entry.dispose();
      panels.clear();
    },
    { once: true },
  );
})();
