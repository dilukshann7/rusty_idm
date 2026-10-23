// Loaded before content.js in the isolated extension world. No page scripts or storage.
(() => {
  const WIDTH = 225, HEIGHT = 26, MARGIN = 4;
  const clamp = (value, min, max) => Math.min(Math.max(min, max), Math.max(min, value));

  function create(media, { hoverOnly = false, onDismiss = () => {} } = {}) {
    const root = document.createElement("span");
    root.style.cssText = "all:initial!important;position:fixed!important;inset:auto!important;margin:0!important;padding:0!important;border:0!important;width:225px!important;height:26px!important;z-index:2147483647!important;overflow:visible!important;background:transparent!important;color-scheme:light!important;";
    const shadow = root.attachShadow({ mode: "closed" });
    const style = document.createElement("style");
    style.textContent = `
      :host { font:12px Tahoma,Arial,sans-serif; color:#203f61; }
      * { box-sizing:border-box; }
      [hidden] { display:none!important; }
      .bar { display:flex; align-items:center; gap:2px; width:100%; height:26px; padding:3px;
        border:1px solid #6f8ca9; border-radius:3px; background:linear-gradient(#f7fbff,#c5d3e1 48%,#a1b6ce 51%,#dce6ef);
        box-shadow:inset 1px 1px #fff,inset -1px -1px #829bb4,0 1px 3px #0005; user-select:none; }
      button { font:12px Tahoma,Arial,sans-serif; color:#203f61; cursor:pointer; }
      .download { position:relative; flex:1; min-width:0; height:20px; padding:0 3px 1px 19px; text-align:left;
        white-space:nowrap; font-weight:bold; letter-spacing:.1px; touch-action:none; cursor:grab;
        border:1px solid #7a98b8; border-radius:4px; background:linear-gradient(#f7fbff,#cedded 46%,#afc5df 50%,#e2edf7);
        box-shadow:inset 1px 1px #fff; }
      .download::before { content:""; position:absolute; left:4px; top:3px; border-top:6px solid transparent;
        border-bottom:6px solid transparent; border-left:11px solid #21983d; filter:drop-shadow(1px 1px 0 #b8e9bb); }
      .download:active { cursor:grabbing; }
      .small { flex:none; width:15px; height:16px; line-height:12px; padding:0; border:1px solid #8ba1b7;
        border-radius:4px; font-weight:bold; background:linear-gradient(#fcfdff,#a7bed5); box-shadow:inset 1px 1px #fff; }
      button:hover { filter:brightness(1.06); } button:disabled { opacity:.6; cursor:wait; }
      button:focus-visible,select:focus-visible { outline:2px solid #1b64a7; outline-offset:1px; }
      .popover { position:absolute; width:380px; max-width:calc(100vw - 8px); max-height:calc(100vh - 38px);
        overflow:auto; padding:6px; border:1px solid #4b4b4b; border-radius:4px; background:#292929;
        box-shadow:0 4px 14px #0006; font:13px/1.5 Tahoma,Arial,sans-serif; color:#fff; }
      .picker { width:640px; }
      select { display:block; width:100%; margin:0 0 6px; padding:4px; font:12px Tahoma,Arial,sans-serif; background:#333; color:#fff; border:1px solid #555; }
      .format { display:block; width:100%; padding:5px 9px; text-align:left; border:0; border-radius:2px;
        background:transparent; color:#fff; font:13px/1.5 Tahoma,Arial,sans-serif; overflow-wrap:anywhere; }
      .format:hover,.format:focus-visible { background:#454545; filter:none; }
      .retry { margin:6px 8px; padding:4px 8px; border:1px solid #666; border-radius:3px; background:#414141; color:#fff; }
      .detail { display:block; padding:5px 9px; overflow-wrap:anywhere; font-size:12px; color:#ddd; }
      .heading { display:block; margin:0 8px 4px; padding:2px 0 7px; border-bottom:1px solid #414141; }
    `;
    const make = (tag, className, text) => {
      const element = document.createElement(tag);
      element.className = className;
      if (text) element.textContent = text;
      return element;
    };
    const bar = make("div", "bar");
    const button = make("button", "download", media.tagName === "AUDIO" ? "Download this audio" : "Download this video");
    const helpButton = make("button", "small", "?");
    const closeButton = make("button", "small", "×");
    helpButton.setAttribute("aria-label", "Help with Rusty video downloads");
    closeButton.setAttribute("aria-label", "Dismiss download bar until reload");
    button.title = "Download with Rusty. Drag to move this bar.";
    bar.setAttribute("role", "group");
    bar.setAttribute("aria-label", "Rusty media download bar");
    button.setAttribute("aria-expanded", "false");
    const picker = make("div", "popover picker");
    picker.hidden = true;
    picker.setAttribute("role", "group");
    picker.setAttribute("aria-label", "Video sources and formats");
    const heading = make("span", "heading", "Choose video quality");
    const sourceChoice = make("select");
    sourceChoice.setAttribute("aria-label", "Video source");
    const formatList = make("div");
    formatList.setAttribute("role", "menu");
    formatList.setAttribute("aria-label", "Available video formats");
    const retryButton = make("button", "retry", "Refresh formats");
    retryButton.hidden = true;
    const detail = make("span", "detail");
    detail.setAttribute("role", "status");
    sourceChoice.style.display = "none";
    picker.append(heading, sourceChoice, formatList, detail, retryButton);
    const help = make("div", "popover", "Drag the main button to move this bar. Click it to download a direct file or choose formats for a public, on-demand video. Play the video first if no source appears. Live, DRM, and account-required videos are unsupported. × hides this bar until reload.");
    help.hidden = true;
    help.setAttribute("role", "note");
    bar.append(button, helpButton, closeButton);
    shadow.append(style, bar, picker, help);
    document.documentElement.append(root);

    let disposed = false, frame, timer, drag = null, suppressClick = false;
    let offsetX = 0, offsetY = 0, mediaHover = false, toolbarHover = false, focused = false, hoverGrace = false;
    let currentPosition = { left: 0, top: 0 };
    const listeners = [];
    const listen = (target, name, handler, options) => {
      target.addEventListener(name, handler, options);
      listeners.push(() => target.removeEventListener(name, handler, options));
    };
    const promote = () => {
      if (!root.showPopover) return;
      try {
        root.hidePopover();
        root.showPopover();
      } catch {
        // A closed popover has display:none in the UA sheet; remove it for the fallback.
        root.removeAttribute("popover");
      }
    };
    if (root.showPopover) {
      root.setAttribute("popover", "manual");
      promote();
    }
    function position() {
      if (disposed) return;
      const rect = media.getBoundingClientRect();
      const viewport = globalThis.visualViewport;
      const viewLeft = viewport?.offsetLeft || 0, viewTop = viewport?.offsetTop || 0;
      const viewWidth = viewport?.width || innerWidth, viewHeight = viewport?.height || innerHeight;
      const width = Math.min(WIDTH, Math.max(0, viewWidth - MARGIN * 2));
      const left = clamp(rect.right - WIDTH - MARGIN + offsetX, viewLeft + MARGIN, viewLeft + viewWidth - width - MARGIN);
      const top = clamp(rect.top + MARGIN + offsetY, viewTop + MARGIN, viewTop + viewHeight - HEIGHT - MARGIN);
      currentPosition = { left, top };
      const hostStyle = (name, value) => {
        if (root.style[name] !== value) root.style.setProperty(name, value, "important");
      };
      hostStyle("left", `${left}px`);
      hostStyle("top", `${top}px`);
      hostStyle("width", `${width}px`);
      const full = document.fullscreenElement;
      const nativeFullscreen = full === media || media.webkitDisplayingFullscreen;
      const matchesFullscreen = !full || full.contains(media);
      const onscreen = media.isConnected && rect.width > 0 && rect.height > 0 &&
        rect.bottom > viewTop && rect.top < viewTop + viewHeight && rect.right > viewLeft && rect.left < viewLeft + viewWidth;
      const interacting = mediaHover || toolbarHover || focused || hoverGrace || drag || !picker.hidden || !help.hidden;
      const eligible = onscreen && matchesFullscreen && !nativeFullscreen;
      const visible = eligible && (!hoverOnly || interacting);
      // Idle hover-only controls remain in the tab order so keyboard focus can reveal them.
      hostStyle("visibility", eligible ? "visible" : "hidden");
      hostStyle("opacity", visible ? "1" : "0");
      if (root.style.pointerEvents !== (visible ? "auto" : "none"))
        root.style.setProperty("pointer-events", visible ? "auto" : "none", "important");
      for (const popover of [picker, help]) {
        const popWidth = Math.min(popover === picker ? 640 : 380, Math.max(0, viewWidth - MARGIN * 2));
        popover.style.left = `${clamp(left + width - popWidth, viewLeft + MARGIN, viewLeft + viewWidth - popWidth - MARGIN) - left}px`;
        const below = viewTop + viewHeight - top - HEIGHT - MARGIN;
        const above = top - viewTop - MARGIN;
        const useAbove = below < 200 && above > below;
        popover.style.top = useAbove ? "auto" : "30px";
        popover.style.bottom = useAbove ? "30px" : "auto";
        popover.style.maxHeight = `${Math.max(0, (useAbove ? above : below) - MARGIN)}px`;
      }
    }
    function track() {
      position();
      // Layout shifts and transforms can move a player without scroll or resize events.
      if (!disposed) frame = requestAnimationFrame(track);
    }
    function closePopover(restoreFocus = false) {
      const wasOpen = !picker.hidden || !help.hidden;
      picker.hidden = help.hidden = true;
      button.setAttribute("aria-expanded", "false");
      helpButton.setAttribute("aria-expanded", "false");
      if (wasOpen && restoreFocus) button.focus();
      position();
    }
    function openPicker() {
      help.hidden = true;
      picker.hidden = false;
      button.setAttribute("aria-expanded", "true");
      helpButton.setAttribute("aria-expanded", "false");
      position();
    }
    const delayedPosition = () => {
      clearTimeout(timer);
      hoverGrace = true;
      timer = setTimeout(() => { hoverGrace = false; position(); }, 200);
    };
    listen(media, "mouseenter", () => { mediaHover = true; hoverGrace = false; clearTimeout(timer); position(); });
    listen(media, "mouseleave", () => { mediaHover = false; delayedPosition(); });
    listen(root, "mouseenter", () => { toolbarHover = true; hoverGrace = false; clearTimeout(timer); position(); });
    listen(root, "mouseleave", () => { toolbarHover = false; delayedPosition(); });
    listen(root, "focusin", () => { focused = true; position(); });
    listen(root, "focusout", event => {
      focused = !!event.relatedTarget && (event.relatedTarget === root || shadow.contains(event.relatedTarget));
      delayedPosition();
    });
    listen(helpButton, "click", event => {
      if (!event.isTrusted) return;
      const open = help.hidden;
      closePopover();
      help.hidden = !open;
      helpButton.setAttribute("aria-expanded", String(open));
      position();
    });
    listen(closeButton, "click", event => {
      if (!event.isTrusted) return;
      dispose(); onDismiss();
    });
    listen(document, "pointerdown", event => {
      if (!event.composedPath().includes(root)) closePopover();
    }, true);
    listen(document, "keydown", event => {
      if (event.key === "Escape" && (!picker.hidden || !help.hidden)) {
        event.preventDefault(); event.stopPropagation(); closePopover(true);
      }
    }, true);
    // Keep overlay interactions from reaching the site's player controls.
    for (const name of ["click", "dblclick", "pointerdown", "pointerup", "keydown", "keyup"])
      listen(root, name, event => event.stopPropagation());
    listen(button, "pointerdown", event => {
      if (!event.isTrusted || event.button !== 0 || button.disabled) return;
      suppressClick = false;
      drag = { id: event.pointerId, x: event.clientX, y: event.clientY, left: currentPosition.left, top: currentPosition.top, moved: false };
      button.setPointerCapture(event.pointerId);
    });
    listen(button, "pointermove", event => {
      if (!drag || event.pointerId !== drag.id) return;
      const dx = event.clientX - drag.x, dy = event.clientY - drag.y;
      if (!drag.moved && Math.hypot(dx, dy) < 5) return;
      drag.moved = true;
      event.preventDefault(); event.stopPropagation();
      const rect = media.getBoundingClientRect();
      offsetX = drag.left + dx - (rect.right - WIDTH - MARGIN);
      offsetY = drag.top + dy - (rect.top + MARGIN);
      position();
    });
    const endDrag = event => {
      if (!drag || event.pointerId !== drag.id) return;
      suppressClick = drag.moved && event.type === "pointerup";
      drag = null;
      try { button.releasePointerCapture(event.pointerId); } catch { /* Capture may already be released. */ }
      position();
    };
    for (const name of ["pointerup", "pointercancel", "lostpointercapture"]) listen(button, name, endDrag);
    listen(button, "dragstart", event => event.preventDefault());
    listen(button, "keydown", event => {
      if (event.key === "ArrowDown") {
        event.preventDefault(); openPicker(); formatList.children[0]?.focus();
      }
    });
    listen(document, "fullscreenchange", () => { promote(); position(); });
    listen(media, "webkitbeginfullscreen", position);
    listen(media, "webkitendfullscreen", () => { promote(); position(); });
    listen(globalThis, "scroll", position, true);
    listen(globalThis, "resize", position);
    if (globalThis.visualViewport) {
      listen(visualViewport, "resize", position);
      listen(visualViewport, "scroll", position);
    }
    const resize = typeof ResizeObserver === "function" ? new ResizeObserver(position) : null;
    resize?.observe(media);
    const intersection = typeof IntersectionObserver === "function" ? new IntersectionObserver(position) : null;
    intersection?.observe(media);
    function dispose() {
      if (disposed) return;
      disposed = true; drag = null;
      clearTimeout(timer); cancelAnimationFrame(frame);
      resize?.disconnect(); intersection?.disconnect();
      for (const remove of listeners) remove();
      root.remove();
    }
    position();
    frame = requestAnimationFrame(track);
    function setBusy(busy) {
      sourceChoice.disabled = retryButton.disabled = busy;
      for (const row of formatList.children) row.disabled = busy;
    }
    function setFormats(title, variants, onSelect) {
      formatList.replaceChildren();
      if (variants.length > 128) throw new RangeError("Too many video quality choices are available.");
      variants.forEach((variant, index) => {
        const size = variant.size == null ? "size unknown" : `${(variant.size / 1048576).toFixed(1)} MB`;
        const row = make("button", "format", `${index + 1}. ${variant.quality} — ${variant.container} file, audio: ${variant.audio}, ${size} — ${title}`);
        row.title = row.textContent;
        row.setAttribute("role", "menuitem");
        row.addEventListener("click", event => {
          if (event.isTrusted && !row.disabled && !disposed) return onSelect(variant.id);
        });
        row.addEventListener("keydown", event => {
          const length = formatList.children.length;
          const next = event.key === "ArrowDown" ? (index + 1) % length
            : event.key === "ArrowUp" ? (index + length - 1) % length
            : event.key === "Home" ? 0 : event.key === "End" ? length - 1 : null;
          if (next !== null) { event.preventDefault(); formatList.children[next]?.focus(); }
        });
        formatList.append(row);
      });
    }
    return { root, button, helpButton, closeButton, picker, help, sourceChoice, formatList, retryButton, detail,
      setFormats, setBusy,
      openPicker, closePopover, dispose,
      consumeDragClick() { const consumed = suppressClick; suppressClick = false; return consumed; },
    };
  }
  globalThis.RustyMediaToolbar = Object.freeze({ create });
})();
