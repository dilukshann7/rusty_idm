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
      if (!src || !/^https?:/u.test(src)) continue;
      let extension;
      try {
        extension = new URL(src).pathname.split(".").pop().toLowerCase();
      } catch {
        continue;
      }
      const supported =
        media.tagName === "VIDEO"
          ? ["mp4", "mkv", "webm", "mov", "avi", "m4v"]
          : ["mp3", "wav", "ogg", "m4a", "flac", "aac", "opus"];
      if (!supported.includes(extension)) continue;
      const root = document.createElement("span");
      const shadow = root.attachShadow({ mode: "closed" });
      const button = document.createElement("button");
      button.textContent = "Download with Rusty";
      button.style.cssText =
        "font:12px Tahoma,sans-serif;padding:3px 8px;color:#111;background:#f4f4f4;border:1px solid #aaa;border-radius:3px;cursor:pointer;";
      button.addEventListener("click", async (event) => {
        if (!event.isTrusted || button.disabled) return;
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
      shadow.append(button);
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
