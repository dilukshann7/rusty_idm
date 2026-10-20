# Compact Configuration Pages Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan inline, task by task. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** Implement the approved four compact native Options pages and their working download/browser preferences.

**Architecture:** Shared validated policy types feed versioned settings, the engine and extensions. Native child pages edit a draft; a single application boundary applies ownership-checked system changes and settings. Existing engine scheduling, guarded file finalization and browser leases remain authoritative.

**Tech Stack:** Rust, existing Tokio/serde/windows dependencies, native Win32 dialogs, browser JavaScript and per-user Windows integration scripts.

**Spec:** ../specs/2026-10-20-configuration-pages-design.md (user approved 2026-10-20).

## Global Constraints

- Existing codex/queues-and-system-icons checkout in D:/Programming/Rust/rusty_idm; inline implementation, one final GPT-6.1 Sol source reviewer.
- Four pictured tabs, Rusty branding, compact Tahoma 8 child pages; no inert additional tabs.
- Preserve existing settings, Chrome extension identity, queues, leases and recovery. New automation defaults off.
- No builds, tests/test additions, fixture servers, GUI/browser launches, live setup/registry changes, installs or publishing. Existing fixture literals may be adjusted for interface compatibility. Formatting/source/resource checks are allowed.

## Review Focus

- Hidden-page visibility, tab clipping and repeated no-change redraw must remain stable at different DPIs.
- Stale browser policy, disconnected hosts and partial apply failures must preserve browser ownership and previous preferences.
- Same/cross-volume partials, interruption, original replacements and unconfirmed transfers must preserve file identities and recovery.
- Clipboard/modifier/media/URL rules must not turn arbitrary text, expired intent or authenticated/unsafe requests into automatic downloads.
- Settings migration, category edits and scan failures must not change active destinations, leak credentials or falsely report success.

## Task 1: Shared settings and capture/category policy

Files: browser-bridge policy.rs/lib.rs; engine categories.rs/model.rs; win32 settings.rs and new preferences.rs.
Interfaces: validated CapturePolicy and BrowserKind; CategoryRule and DownloadOptions; Settings retains old fields and adds defaulted Configuration. Pure parsing/matching routines shared with admission, private address rules encrypted.
- [x] Add versioned types, bounded validation, factory extension/site lists and category defaults; preserve schema-1 settings on load.
- [x] Implement host/URL/extension matching, normalized category conflicts and validated UA/scanner options. Add private-rule DPAPI storage using existing store primitives.
- [x] Inspect compatibility and run source formatting/checks. Commit coherent settings model.

## Task 2: Engine request options and guarded transfer behavior

Files: engine model.rs, service.rs, service_queues.rs, storage.rs, transfer.rs, output.rs; focused helper modules as necessary.
Interfaces: request-level DownloadOptions, provisional confirmation gate and commands; persistent option/confirmation records; journaled staging for a final-directory copy.
- [x] Carry immutable admission options through queued recovery, set User-Agent and optional server creation time, and support modification-time tolerance only after identity/content verification.
- [x] Support configured partial directories and cancellation/recovery-safe same/cross-volume publication using unique final-directory staging and journal phases.
- [x] Gate provisional publication on explicit confirmation; cancellation and Download Later use the existing actor, durable admission and pause acknowledgement. Restore provisional records held.
- [x] Source-inspect copy/commit/cancel/restart/validator cases. Format/check and commit.

## Task 3: Four-browser integration and policy enforcement

Files: browser-bridge protocol/policy; native-host config/main/router; win32 browser_integration; extension shared modules/content script/manifests; owned registration/package scripts and installer.
Interfaces: browser-attributed policy exchange and revision; per-browser caller allowlist/config; shared extension adapter.
- [x] Preserve Chrome registration while adding owned Chrome/Brave/Edge/Firefox host and package configurations; implement Firefox invocation/background adaptation.
- [x] Apply extension/host/URL policy before pausing and recheck desktop policy before admission. Preserve existing accepted receipts, recovery and safe-request restrictions.
- [x] Add expiring per-document modifier intent, configured context menus and opt-in direct-media panels with no credential forwarding.
- [x] Source/manifest/script checks and commit; no live installation or browser action.

## Task 4: Compact native Options and working editors

Files: win32 new options.rs/options_pages.rs/options_editors.rs; resource dialogs and control IDs; dialogs.rs and app entry.
Interfaces: options::show(owner,&Settings)->Result<Option<Settings>> with stable context, child pages, validation and shared draft.
- [x] Build General/File types/Save to/Downloads child pages, shared icon/header/footer and keyboard/DPI behavior.
- [x] Connect browser setup/status, key/menu/panel editors, defaults/exclusion list, category New/Edit/Browse, progress options and scanner editor.
- [x] Keep edits across tabs, avoid unchanged writes/redraw, retain draft after failed apply and show local Help. Inspect every resource/control mapping. Format/check and commit.

## Task 5: Application workflows and transactional preference application

Files: win32 app.rs/dialogs.rs/download_form.rs, new configuration_apply.rs/clipboard.rs/batch.rs/scanner.rs; engine command integration.
Interfaces: accepted settings revision; clipboard owned actions; held queue selection; provisional File Info sessions; bounded scanner completions.
- [x] Stage/validate preferences and compensate owned startup/host mutations on failed save; update the app/engine/extensions only after acceptance.
- [x] Connect clipboard baseline/dedup, dialog bypass/queue-only, queue chooser and minimal URL batch workflow to existing duplicate/lease/scheduler rules.
- [x] Connect provisional File Info confirm/cancel/Later, read-only immutable fields, progress preferences, and scanner-before-Open behavior without blocking the UI.
- [x] Preserve manual paths and remember-folder switches. Source-inspect shutdown, modal/lease races, queue creation, scan and partial-apply failures; format/check and commit.

## Task 6: Whole-change source review and documentation

Files: README, feature guide/spec status, source-check scripts as necessary.
- [x] Document usage, four-browser setup, defaults and deferred runtime matrix.
- [ ] Run cargo fmt --all, scripts/verify-source.ps1 and git diff --check; these are not compilation/runtime verification.
- [ ] One fresh GPT-6.1 Sol source review of complete implementation against the spec and Review Focus; fix important findings inline and rerun allowed checks.
- [ ] Commit, leave feature checkout available and report actual verification boundaries. No merge/push/release without request.
