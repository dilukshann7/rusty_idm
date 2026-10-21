# Expired-link resume implementation plan

**Goal:** Continue an unfinished HTTP(S) GET download after its address changes, preserving verified partial bytes.

**Architecture:** Durable checkpoints own partial files. A refresh candidate is checked with exact byte-range probes before a browser lease is accepted or a pasted URL is committed. Browser request context is short-lived and origin-scoped.

**Tech stack:** Rust engine, Win32 UI, native messaging bridge, Chrome extension JavaScript.

**Spec:** Approved plan in this task, 2026-10-21.

## Global constraints

- No dev server or Rust build unless the user specifically asks.
- Keep browser secrets out of durable storage, logs, snapshots, and browser storage.
- Keep the original partial intact on verification failure.
- Commit all requested work, including the README and IDM investigation.

## Tasks

- [x] Preserve ordinary failed partials with durable checkpoints and restore unfinished jobs held.
- [x] Capture an optional source page and short-lived origin-scoped browser request context.
- [x] Verify changed-link candidates using exact total and bounded range samples before rebind.
- [x] Wire browser and paste refresh through the Win32 UI, with browser fallback on refusal.
- [x] Document the behavior, add focused tests, and run permitted source/JavaScript checks.
- [x] Commit all changes.

## Review focus

- Invalid ranges, wrong totals, changed local partials, and failed verification must never append.
- A browser capture must remain browser-owned until the refresh is verified and accepted.
- Browser cookies and authorization must not enter persisted state or logs.
- An app restart must hold unfinished work and require recapture for expired session context.
- Original automatic capture behavior must remain unchanged outside an armed refresh.

Rust compilation, Rust tests and live Win32/browser acceptance remain deliberately unrun under the workspace instruction. The new Rust tests cover sample matching/mismatch, totals, ETags, rejected ranges, authorization and unobserved redirects, but their runtime status is unverified.
