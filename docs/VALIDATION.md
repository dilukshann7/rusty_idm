# Validation

The v0.1.0 release uses Windows x64 optimized binaries and an unsigned current-user
Inno Setup installer. Source verification, native resource/version inspection,
static-CRT dependency inspection, archive integrity and SHA-256 checks establish
build/package results. They do not establish live desktop/browser behavior.

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
