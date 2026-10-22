# Validation

## Automatic recovery source change (2026-10-22)

The [automatic recovery](automatic-recovery.md) change has source-only validation.
New tests cover policy defaults/bounds, actor retry deadlines and history,
queue continuation/stop, pause/resume, explicit start, cancellation/removal/exit,
checkpoint storage failure, HTTP/browser recovery, interrupted segmented transfers,
changed/ignored ranges, and native settings/status presentation.
Compilation, executable tests, fixture servers, application launch, and visual
acceptance remain deferred by request. Earlier results below do not validate
these new changes.

## Permanent history source change (2026-10-22)

Schema 7 retains completed/failed ordinary downloads, dates, byte counts,
sanitized failure explanations, categories and existing request metadata.
Schemas 1–6 migrate on the next successful save. The store supports 10,000
entries within the existing 16 MiB limit; capacity errors preserve records.
History deletion is persisted before publishing removal. Successful final file
publication remains successful when its final history save needs retry.

Regression sources cover lifecycle/restart metadata, duplicate lookup, missing
files, same-record retries, queued segmented checkpoints, media byte counts,
protected/quarantined admissions, schema migration, capacity, failed saves,
deletion/file preservation and suppression of restored progress windows.
Tests use isolated in-memory/temp stores and a fake executor; they do not
inspect or modify the user's saved downloads.

Rust formatting/syntax, manifest/resource/document-link and JavaScript/PowerShell
source checks are the verification boundary for this change. No compilation,
Rust tests, fixture/dev server, app/browser launch, installer run or publishing
was performed. The newly added Rust tests have not been compiled or executed.

`scripts/verify-source.ps1` passed for 98 Rust files, workspace manifests/lock,
resources/assets/document links and browser source syntax. `git diff --check`
passed. Independent source review found and corrected Cancel → Retry storage
reattachment and the final retry during atomic shutdown; both have regression
sources. These results do not establish type checking or runtime behavior.

Deferred acceptance: actual encrypted restart after successful/failed transfers;
partial recovery and fresh browser capture; completed/missing-file actions;
local dates and category filters; installer upgrade with schema 1–6 data;
write failure/retry/shutdown and deletion failures; large-history UI performance.
Previously discarded history cannot be reconstructed.

## Broader browser capture and video source checks (2026-10-21)

- `node --test extensions/chrome/tests/*.test.js`: 42 extension tests passed,
  including protected context, redirect, fallback, media discovery and version 2
  native reply cases, picker races/choices and direct file panel actions.
- `python tools/test_media_package.py`: 3 package source checks passed for
  executable hashes, paths, PE architecture, runtime hash consistency and notices.
- `python -m unittest discover -s tools/tests -p test_browser_packages.py`: the
  four browser profiles staged valid archives containing the media observation
  and panel sources.
- `cargo metadata --locked --offline --no-deps --format-version 1`: workspace
  metadata resolved without building.
- `scripts/verify-source.ps1`: Rust formatting and source checks passed for 94
  Rust files, workspace manifests/lock, resources, browser assets and scripts.
- The pinned yt-dlp executable accepted the restricted runtime flags and
  reported version 2026.08.19.

No Rust build, Rust tests, installer/portable package generation, dev server or
live browser/media acceptance was run for this source change. In particular,
protected file handoff in each browser, HLS/DASH quality selection, YouTube
video/audio merging, restart and tool-failure paths remain unverified on the
running app.

Rust regression sources cover protected redirect progression, media format
rechecks, unknown-codec manifests, completed stream validation, retained media
retry/removal, destination collisions, process-tree shutdown, stale previews,
idempotent receipts and capacity after completed submissions. These new Rust
tests have not been compiled or executed.
The protected restart regression also checks that restart confirmation cannot
bypass the requirement for a new browser capture.

## v0.2.1 multi-connection verification (2026-10-21)

The engine now has a schema-6 range map, per-range owned files, exact range
probes and worker validation, dynamic splitting after endpoint acknowledgement,
aggregate speed control, Last-Modified overlap rechecks, ordered assembly and
cleanup journaling. Native Options exposes 1–16 connections, default 8, for
newly admitted downloads.

- `cargo test --workspace --locked --offline --quiet`: 125 Rust tests passed.
  Eight new HTTP fixture cases cover concurrent ranges, ignored-range fallback,
  malformed range and changed-validator rejection, dynamic splitting,
  Last-Modified verification, aggregate speed limiting and pause/resume with
  saved sidecars.
- `node --test extensions/chrome/tests/*.test.js`: 31 extension tests passed.
- `python -m unittest discover -s tools/tests -p test_browser_packages.py`:
  one browser-package test passed.
- `scripts/verify-source.ps1`: 92 Rust files, workspace versions, native
  resources, document links, browser source and PowerShell syntax passed.
- An optimized static-CRT Windows build of the app and native host passed;
  both binaries report product version 0.2.1.
- The Inno Setup installer and five archives were produced. All six release
  artifacts matched their recorded SHA-256 hashes; all five ZIPs passed archive
  integrity checks.

The first workspace run exposed an old fixture that rejected a validator-free
`bytes=0-0` probe. A later run exposed a pause-before-probe acknowledgement
race; both were corrected before the passing run above. One refresh fixture
network error was intermittent and passed on rerun.

Desktop transfer, forced-process restart with sidecars, checkpoint corruption,
interrupted assembly, two simultaneous large files, and address refresh with
segmented partials still need Windows acceptance.
Installer upgrade/uninstall and live browser handoff remain unverified.

## v0.2.0 preparation (2026-10-21)

- `cargo test --workspace --locked --offline --quiet`: 111 Rust tests passed. The
  local HTTP fixture now uses blocking accepted sockets, and browser-controller
  tests use an in-memory engine rather than the user's saved queue.
- `node --test extensions/chrome/tests/*.test.js`: 31 extension tests passed.
- `python -m unittest discover -s tools/tests -p test_browser_packages.py`:
  browser archive staging passed for Chrome, Brave, Edge and Firefox.
- `scripts/verify-source.ps1` and `git diff --check` passed after formatting.
- Debug Win32 app and native host built with the v0.2.0 version resources. The
  desktop app launched, showed the main window, Options pages, Scheduler and
  Add URL dialog, and exited through its menu. No live transfer was started.

The optimized release binaries and installer have not been built or packaged
for v0.2.0. Installer upgrade/uninstall, live browser handoff, authenticated
address refresh and transfer behavior in the desktop UI still need Windows
acceptance. This is release candidate validation, not publication.

## Manual acceptance still required

| Area | Cases |
| --- | --- |
| Setup | Fresh install, existing registration, running-process guard, uninstall preserves user data |
| Background | Hidden sign-in startup, second instance, Close/minimize to tray, explicit Exit |
| Windows lifecycle | Explorer restart, sign-out during modal prompts, active-transfer shutdown |
| Capture | Supported public GET, unsupported/authenticated downloads remain in Chrome, lost acknowledgements |
| File Info | Add/held/cancel, duplicate choices, missing folders, Unicode/long paths |
| Transfers | Two simultaneous jobs, pause/resume, cancellation, limit changes, retry and guarded replacement |
| Completion | Progress closes on success, compact dialog actions, saved opt-out and explicit Properties |
| Context menu | Right-click row targeting, keyboard menu, state-dependent actions and double-click preference |
| Layout | 100/125/150/200 percent DPI, monitor changes, compact controls and keyboard access |

## Earlier validation records

The v0.1.0 release uses Windows x64 optimized binaries and an unsigned current-user
Inno Setup installer. Source verification, native resource/version inspection,
static-CRT dependency inspection, archive integrity and SHA-256 checks establish
build/package results. They do not establish live desktop/browser behavior. The
source-only notes below describe earlier checkpoints before the v0.2.0 test pass.

## Unreleased address refresh source (2026-10-21)

The [address refresh guide](refresh-download-address.md) describes the new flow. The extension's JavaScript suite passed 31/31 tests, including secret handling, short-lived redirect headers, browser fallback and user pause preservation. Offline Cargo manifest inspection, changed-file Rust parsing with `rustfmt --emit stdout`, the Chrome source script, duplicate dialog-ID inspection and `git diff --check` also passed. Existing source files were already outside `rustfmt --check` formatting, so the full `verify-source.ps1` formatting gate is not claimed here.

Rust compilation, Rust tests and live Windows/browser checks were not run under the workspace's no-build instruction. Authenticated refresh, restart-held recovery, storage failure and exact-range behavior still need runtime acceptance on Windows.

No test suite or live acceptance is claimed by packaging the release. The exact
artifact hashes and sizes are in the published SHA256SUMS.txt and release manifest.
The v0.1.0 build record is available in
[the v0.1.0 validation record](v0.1.0-release-validation.md); freshly published
artifacts use the hashes attached to their GitHub release.

## Unreleased compact configuration source

The configuration implementation was reviewed once against the approved
[design](superpowers/specs/2026-10-20-configuration-pages-design.md), covering
`fbac56a..c5c1399`. A fresh GPT-6.1 Sol reviewer found nine Important issues and
three minor issues. The following were addressed in one source fix pass:

| Finding | Source correction |
| --- | --- |
| Firefox callback namespace selected by promise-based code | Prefer `browser` in the shared adapter and isolated content script; retain callback-compatible message listeners |
| Saved Overwrite bypassed fresh approval | Always prompt for replacement; pass the exact approved identity to duplicate restart admission and reject changed destinations |
| Expired durable admission could reappear after restart | Persist external admissions as pending, remove expired entries and membership, commit only accepted admissions before scheduling/receipt; failed compensation remains quarantined and ineligible for scheduling |
| Initial details expansion could leave dangling window data | Destroy the HWND before dropping its owner when expansion fails |
| Quiet validation could loop without an editable form | Reopen editable File Info for invalid addresses/paths/folders, retaining Cancel |
| Disabled queue-only checkbox still held Start requests | Gate and normalize queue-only against the start-dialog preference |
| Old scanner success could authorize a replaced file | Retire conflicting destination records, gate by current generation, cancel/mark unfinished removed-entry scans unverified |
| Passive policy refresh restarted Rusty after Exit | Policy reads return unavailable without starting the desktop process; explicit capture still owns its launch path |
| Unregister left an executable that blocked re-registration | Include the verified owned browser host copy in unregister and rollback snapshots |

The media-panel finding was regraded from minor because the visible button could
reject the displayed source without feedback. The fix filters unsupported media
extensions before drawing a panel and reports the handoff outcome on its button.

Two minor findings remain deferred: browser status establishes registration and
detection rather than live extension connection telemetry; an initially closed
queue-query receiver leaves the cancellable empty picker instead of displaying
an engine-closure error.

Recovery storage now writes schema 4. Readers accept schemas 1–4 and migrate in
memory; older binaries reject the new schema rather than ignoring admission
quarantine/confirmation metadata. The storage filename stays unchanged.

Allowed verification: `cargo fmt --all`, `scripts/verify-source.ps1` (85 Rust
files, resource IDs/assets/manifests, JS and PowerShell syntax) and
`git diff --check`. Python packaging helpers were also parsed as source.
No compilation, test additions/execution, app/browser launch, fixture server,
installation, live registry/setup changes or publishing was performed. The
review corrections have source inspection/parser evidence only, with no
red/green or runtime claim. See the [configuration guide](configuration.md) for
the deferred runtime acceptance matrix.
