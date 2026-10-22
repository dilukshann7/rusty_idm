import { readFileSync, existsSync } from "node:fs";
import { runInContext, createContext } from "node:vm";
import { randomUUID } from "node:crypto";

export class Element {
  constructor(tag) {
    this.tagName = tag.toUpperCase();
    this.style = { setProperty(name, value) { this[name] = value; } };
    this.children = []; this.handlers = new Map(); this.attributes = {};
    this.value = ""; this.isConnected = true;
    this.once = new WeakSet();
    this.rect = { left: 100, top: 100, right: 740, bottom: 460, width: 640, height: 360 };
  }
  append(...children) {
    this.children.push(...children);
    for (const child of children) child.parentNode = this;
    if (this.tagName === "SELECT" && !this.value) this.value = this.children[0]?.value || "";
  }
  replaceChildren(...children) { this.children = []; this.value = ""; this.append(...children); }
  addEventListener(name, handler, options) {
    if (!this.handlers.has(name)) this.handlers.set(name, new Set());
    this.handlers.get(name).add(handler);
    if (options?.once) this.once.add(handler);
  }
  removeEventListener(name, handler) { this.handlers.get(name)?.delete(handler); }
  setAttribute(name, value) { this.attributes[name] = value; }
  getAttribute(name) { return this.attributes[name] ?? null; }
  removeAttribute(name) { delete this.attributes[name]; }
  querySelector() { return null; }
  attachShadow() { const shadow = new Element("shadow"); shadow.host = this; return this.shadow = shadow; }
  insertAdjacentElement(position, element) { this.panel = element; }
  contains(other) { return this === other || this.children.some(child => child.contains(other)); }
  getBoundingClientRect() { return this.rect; }
  remove() { this.isConnected = false; if (this.parentNode) this.parentNode.children = this.parentNode.children.filter(child => child !== this); }
  focus() { this.focused = true; }
  setPointerCapture(id) { this.captured = id; }
  releasePointerCapture() { this.captured = null; }
  showPopover() { this.popoverOpen = true; }
  hidePopover() { this.popoverOpen = false; }
  fire(name, extra = {}) {
    const event = { type: name, isTrusted: true, target: this, button: 0, pointerId: 1, clientX: 600, clientY: 115,
      preventDefault() { this.defaultPrevented = true; }, stopPropagation() { this.propagationStopped = true; },
      composedPath: () => [this], ...extra };
    const pending = [...(this.handlers.get(name) || [])].map(handler => {
      if (this.once.has(handler)) this.handlers.get(name).delete(handler);
      return handler(event);
    });
    return Promise.all(pending);
  }
  get listenerCount() { return [...this.handlers.values()].reduce((count, items) => count + items.size, 0); }
}

export function dom({ hover = false, topLayer = true } = {}) {
  const document = new Element("document");
  const window = new Element("window");
  const video = new Element("video");
  document.documentElement = new Element("html");
  document.createElement = tag => {
    const element = new Element(tag);
    if (!topLayer) element.showPopover = element.hidePopover = undefined;
    return element;
  };
  document.querySelectorAll = () => video.isConnected ? [video] : [];
  const observers = [], frames = new Map(), timers = new Map();
  let nextId = 1;
  class Observer {
    constructor(callback) { this.callback = callback; this.disconnected = false; observers.push(this); }
    observe() {} disconnect() { this.disconnected = true; }
  }
  const context = createContext({ document, innerWidth: 1000, innerHeight: 700,
    visualViewport: window, crypto: { randomUUID }, URL,
    MutationObserver: Observer, ResizeObserver: Observer, IntersectionObserver: Observer,
    addEventListener: window.addEventListener.bind(window), removeEventListener: window.removeEventListener.bind(window),
    requestAnimationFrame: fn => { const id = nextId++; frames.set(id, fn); return id; },
    cancelAnimationFrame: id => frames.delete(id),
    setInterval: fn => { const id = nextId++; timers.set(id, fn); return id; }, clearInterval: id => timers.delete(id),
    setTimeout: fn => { const id = nextId++; timers.set(id, fn); return id; }, clearTimeout: id => timers.delete(id),
  });
  const script = new URL("../../media-toolbar.js", import.meta.url);
  if (existsSync(script)) runInContext(readFileSync(script, "utf8"), context);
  const frame = () => { const pending = [...frames.values()]; frames.clear(); pending.forEach(fn => fn()); };
  const tick = () => { const pending = [...timers.values()]; timers.clear(); pending.forEach(fn => fn()); };
  return { document, window, video, context, observers, frames, timers, frame, tick,
    create: onDismiss => context.RustyMediaToolbar?.create(video, { hoverOnly: hover, onDismiss }),
    runContent() { runInContext(readFileSync(new URL("../../content.js", import.meta.url), "utf8"), context); },
  };
}
