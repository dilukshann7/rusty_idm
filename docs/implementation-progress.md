# Native milestone implementation record

Plan: `docs/superpowers/plans/2026-10-18-rusty-win32-implementation.md`
Spec: `docs/superpowers/specs/2026-10-18-rusty-win32-design.md`

Execution: inline, 2026-10-19. Existing investigation evidence is preserved.
The source implementation was completed before this local Git repository was initialized.
The user authorized implementation; the earlier documentation-only restriction has
therefore expired. Builds, compilation-producing tests and app/fixture launches
remain prohibited. Tests are written before their corresponding implementation;
RED/GREEN execution is deferred, not claimed.

Pre-flight interfaces:

| Producers → consumers | Decision |
| --- | --- |
| 1 → 2, 3, 4, 6: models and validation | One shared engine model; no HWND in engine contracts. |
| 2 → 3, 6: output identities and transfers | Identity approval never permits unsafe overwrite. |
| 3 → 5, 7: commands, snapshots, shutdown | Owned bounded events; join only after the UI loop. |
| 4 → 5, 6: IDs, categories and actions | All selections use JobId, never a filtered row index. |
| 5 → 6, 7: HWND ownership and wake messages | Window procedures borrow state only for the current call; dialogs run outside those borrows. |

Ruling at implementation time: perform source implementation in the non-Git
workspace, retaining this record instead of Git-dependent skill scripts. Git
history was initialized after the source implementation.

Ruling: replacement approval cannot currently be verified with a race-proof native
transaction without runtime tests. Disable replacement and ask for a new filename;
preserve completed partials if finalization encounters a changed destination.
Cost: existing-file replacement is unavailable in this milestone.

Task status will distinguish implemented source from verified runtime behavior.

| Task | Source status | Verification |
| --- | --- | --- |
| 1 Model/workspace | Implemented; validation/state tests written first | Compilation/tests deferred |
| 2 Output/transfers | Implemented; disposable output/HTTP tests written first | Compilation/fixtures/tests deferred |
| 3 Service/events | Implemented; controllable executor tests written first | Runtime/race tests deferred |
| 4 Presenter/commands | Implemented; presenter tests written first | Compilation/tests deferred |
| 5 Native shell/bridge | Implemented; manifest/resources included | Formatting only; native rendering deferred |
| 6 Dialogs/settings/details | Implemented; settings/progress tests included | Compilation/picker/settings tests deferred |
| 7 Shutdown/handoff | Implemented; lifecycle tests and manual matrix included | Source review only; all manual cases Not run |

Ruling: finalization and cancellation share a commit gate. Once native no-replace
commit wins, a later Stop leaves the job Running until its Completed event. This
avoids reporting Cancelled for an already committed file. Cost: a click after the
commit boundary cannot cancel that completed transfer.

Ruling: retain recovery paths by disabling GUI Retry for finalization/cleanup
failures with retained partials; use Add URL with a new name for a fresh transfer.
Cost: no one-click retry for those failures and no resume of retained bytes.

The single final source review is requested by the executing-plans and
requesting-code-review skills. It cannot establish compile/runtime correctness.

Allowed checks performed: rustfmt successfully parsed/formatted all Rust source;
`cargo generate-lockfile` resolved 176 packages without compiling or downloading
crate sources; `cargo metadata --offline --no-deps --format-version 1` recognized
both workspace members. Cargo.lock is included. No build/test/app/server launch.

## Independent final source review and one fix pass

The reviewer inspected the source and locally cached Windows bindings without
compilation. No Critical findings; three Important findings received source fixes:

- Partial-file identity: reservation denies external write/delete sharing;
  native no-replace rename and deletion use the original handle. Added both
  blocked-substitution integration coverage and deliberately substituted-path
  unit cases, plus a real no-DELETE-handle cleanup failure case.
- Cleanup-error visibility: active removal preserves retained failures; unseen
  retained failures cannot be dismissed by another queued Remove. Retained
  failures cannot be retried. Shutdown displays paths in a native scrolling,
  copyable report. Added actor and pure presenter checks for this lifecycle.
- Verbatim paths: parse drive/UNC prefixes separately, then validate filenames.
  Added valid extended drive/UNC and invalid device/ADS cases.

These fixes are source-reviewed and parsed by rustfmt; RED/GREEN and runtime
verification remain deferred under the user's explicit restriction.

Final: minor (deferred): settings reads the complete file before enforcing 64 KiB.
Final: minor (deferred): ordinary details text can clip long wrapped paths; the
shutdown recovery report uses a scrolling read-only Edit.

Final: Ruling: preserve the approved milestone boundary and disabled replacement.
The later [transfer-controls follow-up](transfer-controls.md) adds active
pause/resume and live per-download limits. Durable history, resume after
application exit, segmentation, named queues and browser integration remain
later work. Cost if wrong: those later capabilities are unavailable in this version.

Final: Ruling: native appearance, accessibility, DPI, filesystem race behavior,
and compilation remain unverified because no compile/launch was authorized.
Cost if wrong: source/API/runtime issues may still exist until those checks run.

No second review or broad test runs: the one fix pass is documented above, with
tests written but unexecuted. No remote Git integration or release artifact was
configured during implementation.

## Editor-originated check discovered during handoff

An existing VS Code rust-analyzer independently generated target/debug and
target/flycheck0 output. Reading that saved output revealed a failed earlier
check (four error occurrences: Controls EM_SETLIMITTEXT not imported in dialogs,
and ListView state flags do not implement BitOr). Source corrections import the
message explicitly and combine wrapper values through their underlying u32.
Unused imports from that snapshot were also removed. No new compiler command was
launched to reproduce or verify these corrections.

Workspace-only `.vscode/settings.json` now disables rust-analyzer check-on-save,
build-script loading and procedural-macro loading. No global editor setting or
other project's process was changed. The editor's background compilation means
the earlier statement that no compilation had occurred globally was incomplete;
the accurate boundary is no compilation-producing command initiated by the agent,
and no clean final compilation/test/GUI result established.

Final: Ruling: disable automatic Rust editor compilation in this workspace to
honor the user's no-build instruction. Cost if wrong: automatic compiler/proc-macro
diagnostics stay disabled until the user re-enables them.

Final allowed verification: `scripts/verify-source.ps1` passed for all 27 Rust
files, workspace manifests/lock, manifest XML, resource IDs and handoff links.
There are 42 written Rust test cases; none were executed by the agent. The final
source passed rustfmt parsing/formatting after the diagnostic corrections. No
compiler/test/app/server command was launched to establish a passing runtime result.

## Pause/resume and speed limits, 2026-10-19

The user's subsequent instruction enables both capabilities in the existing
compact status dialog. See [transfer controls](transfer-controls.md) for the
state, queue, HTTP validation, safe restart and pacing behavior. This supersedes
the earlier pause/resume deferral. Session persistence and resuming retained
failure artifacts remain outside this change.

Rustfmt and `scripts/verify-source.ps1` passed for 34 Rust files. The workspace
now contains 66 written tests, including actual HTTP/control/owned-handle cases;
none were run. An independent source review found no concrete blocking finding.
No compiler, test, app, fixture or server was launched during this change.

## Duplicate links and approved overwrite, 2026-10-19

The user's duplicate-dialog reference supersedes the initial replacement
deferral. [Duplicate handling](duplicate-downloads.md) records the compact dialog,
three real choices, persistent preference, private lookup index, serialized
restart and guarded Windows backup/commit/rollback implementation. Retained
original backups are included in details and shutdown recovery reports.

Formatting and source checks are the permitted verification. Independent
source review found no blocking duplicate-flow issue. New tests are authored
but not executed; native replacement, GUI rendering and final compilation
remain unverified. No build, test, app, fixture or server was launched.


## Chrome capture source implementation, 2026-10-19

[Chrome integration](chrome-integration.md) records the new extension, native host,
secured transport, actor receipts, compact dialog integration and session recovery.
All eight plan components are implemented in source. Authored Rust/JavaScript/Pester
tests remain unexecuted. The source verifier covers all four exact workspace
packages, Rust formatting/parsing, native assets, extension JSON/imports/icons,
JavaScript syntax and PowerShell parsing. `git diff --check` is also required.
Compilation, runtime tests, live GUI/Chrome behavior, wrong-user transport attempts
and registration/unregistration remain Not run; no working binary is claimed.

Final source checks passed for 58 Rust files and all four workspace packages.
The partial independent GPT-6.1 Sol review identified type/concurrency issues;
corrections and regression tests were authored, followed by author source review.
The independent review ended at a usage limit. Runtime RED/GREEN, the full test
suite, compiled binaries and live Chrome/setup acceptance remain Not run.


## Chrome setup preparation, 2026-10-19

User requested installation. `cargo build --locked --offline -p rusty-win32
-p rusty-native-host --target-dir D:\\Programming\\Rust\\rusty_idm\\target`
succeeded for Windows in the debug profile. One dead-code warning names the
unused `show_duplicate` wrapper. No tests or development server were run.
Binaries and unpacked extension files were staged under the current user's
LocalAppData RustyIDM directory; both binary copy hashes matched build outputs.
Chrome's extensions page was opened for the user to load the unpacked folder.
The user supplied the loaded extension ID `ipolekmihgpdbmelheajogmgnjgmenol`.
The registration script ran, its HKCU manifest/origin was read back and verified,
and the installed host returned one valid correlated `ok` ping frame with version
0.1.0, GUI not running and empty stderr. Automatic capture remains off.
Unregister, full test suites and actual Chrome download handoff remain unrun.


## Download completion dialog, 2026-10-19

Added the compact reference-style modeless completion window, exact byte summary,
redacted address, saved path, Open/Open with/Open folder/Close and saved opt-out.
Options can re-enable completion notifications. It triggers only on successful
completion transitions and can show a subsequent successful retry.
Windows debug compilation and source checks (59 Rust files) passed. The installed
GUI was replaced and its hash matched the build output. No test suite, live
completion/button/persistence exercise or visual comparison was run. See
[completion behavior](completion-dialog.md).
