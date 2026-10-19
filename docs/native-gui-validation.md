# Native GUI validation

Implementation date: 2026-10-19. Host environment: Windows / PowerShell.

The seven-task native milestone has source and tests. No compilation-producing
commands, Rust test execution, application launch, fixture launch or manual GUI
validation were initiated by the agent. Registry resolution generated Cargo.lock.
An existing VS Code rust-analyzer process independently ran a background check;
see the captured-diagnostics note below. Every manual case
below is **Not run**. Formatting only checks Rust parsing/layout, not type checking,
linking, resource compilation or behavior.

### Editor-originated diagnostics

An existing VS Code rust-analyzer wrote compiler output to `target/flycheck0`.
Its observed check failed with missing EM_SETLIMITTEXT imports and missing BitOr
implementations for the native ListView state wrapper. The source now imports
the message from Controls and combines the wrappers' underlying flags explicitly.
Unused imports were also removed. These corrections have not been recompiled by
an agent command. Workspace `.vscode/settings.json` disables check-on-save,
build-script loading and procedural-macro loading to respect the no-build policy.
No passing final compiler result is claimed. Editor-generated target artifacts
are ignored; they are not a product deliverable.

## Source checks

- Rustfmt: all Rust sources parsed and formatted; final check recorded in the
  execution record.
- Manifests: exact direct versions, targeted Windows features, engine/GUI split.
  Cargo.lock generated successfully by Cargo, without compilation.
- Resources: single embedded application manifest, Common Controls v6,
  PerMonitorV2, long-path declaration, menu/accelerator/dialog IDs.
- Ownership: HWNDs accessed on UI thread; engine wake posts carry no pointers.
  Window callback data stays boxed until after destruction. Borrowed stock icons
  and brushes are never destroyed. Owned fonts are detached before deletion.
- Lifecycle: cancellation and output commit share a gate; cancellation wins until
  the final no-replace operation, then a late Stop cannot override completion.
  UI shutdown waits for ShutdownComplete; runtime join occurs outside callbacks.
- Sensitive data: original URLs remain inside requests; routine errors use fixed
  text/status codes, and snapshots expose source host only. No history/URLs are
  persisted in settings.

These are source-review observations, not runtime acceptance results.

## Engine policies

- Two active transfers, a 64-command queue and at most 1,000 retained/pending
  removed job records. Progress samples coalesce to one latest sample per active
  job, published at 100 ms intervals. Terminal snapshots/removals remain pending
  until consumed. Service-error notices coalesce to the latest message.
- HTTP redirects: limit 10; connection/read inactivity limit 30 seconds; TLS
  verification enabled; reqwest system proxy discovery enabled. No application
  retries or implicit protocol retry policy; Accept-Encoding is identity and
  encoded representations are rejected. These are Rusty policies, not recovered
  IDM defaults. A 206 response is rejected for a new full-file download.
- Finalize after body/length checks and flush/sync. On Windows the partial keeps
  its original handle, with external write/delete sharing disabled. Rename and
  deletion use that handle; a changed partial pathname cannot substitute another
  file. Rename requests set ReplaceIfExists=false. Failed finalization retains
  the downloaded partial. Existing-file replacement is disabled. Native contract:
  [FILE_RENAME_INFO](https://learn.microsoft.com/en-us/windows/win32/api/winbase/ns-winbase-file_rename_info)
  and [SetFileInformationByHandle](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-setfileinformationbyhandle).
- Retry starts at zero with Deny replacement policy. When a partial is retained,
  retry is rejected by the engine and disabled in the GUI to avoid hiding its recovery path. Use Add URL
  with a new filename; no restart/resume of the partial exists yet.
- If an active entry's deletion encounters a cleanup failure, its failed record
  remains visible. At shutdown, a native scrolling, selectable text dialog lists
  retained paths before the main window is destroyed.
- The default and remembered category save folders are stored at
  `%LOCALAPPDATA%\RustyIDM\settings.json`, using a uniquely reserved temporary and
  atomic replacement. Corrupt settings are preserved until an explicit Options
  save. Native shell opening requires an explicit user action and an existing
  eligible file/folder.

## Automated checks to run only when authorized

| Suite | Coverage written | Execution |
| --- | --- | --- |
| Engine model unit tests | URL/name rules, Unicode, transitions, redacted Debug | Not run |
| Engine output integration/unit tests | unique partials, handle ownership/substitution, cleanup/sync failures, destination collisions, disabled replacement | Not run |
| Engine transfer integration tests | position-dependent bytes, empty/unknown length, redirects/loop, 404/503, truncation, encoding, 206, cancellation | Not run |
| Engine service tests | controllable executor, two-job admission/cleanup gate, waiting cancellation, cleanup failures retained through removal/shutdown, fresh retry, 1,000-job bound | Not run |
| Event and commit-gate unit tests | concurrent drain/publish, shutdown delivery, cancel/commit ownership | Not run |
| Presenter unit tests | category/identity mapping, unknown/empty progress, stale commands | Not run |
| Settings/progress/lifecycle/bridge unit tests | corruption/version handling, Unicode, save failure, scaling, shutdown state, disabled wake target | Not run |

Focused commands appear in the implementation plan. Finish with one workspace
check/test pass after authorization; launch separately only when requested.

## Manual acceptance matrix

Use disposable folders and local test data; never use ordinary IDM state or its
browser profile. Verify output bytes against independent expected data.

| Scenario | Expected | Result |
| --- | --- | --- |
| Empty startup | Native layout, empty list, zero jobs/rate | Not run |
| Submit three jobs | Two Running, one Waiting; waiting job admits after cleanup | Not run |
| Known-length transfer | Truthful received/total, bounded percentage and ETA | Not run |
| Unknown-length transfer | Marquee in details; no invented percentage or ETA | Not run |
| Zero-byte response | Completed, 0 B; empty final file | Not run |
| HTTP 404/503 / partial 206 | Clear failure, no final file | Not run |
| Disconnect / stalled body | Failure after inactivity timeout, partial cleanup or visible retained path | Not run |
| Redirect / redirect loop | Bounded success/failure, no leaked URL secrets | Not run |
| Stop running / waiting / all | Cancellation completes cleanup; waiting job never opens a socket | Not run |
| Cancel versus final move | One terminal outcome; cancellation before commit prevents final file | Not run |
| Delete inactive entry | Record disappears; completed file remains | Not run |
| Delete active entry | Cancellation completes before record disappears | Not run |
| Retry failed / cancelled | Fresh zero-byte start, fresh destination validation | Not run |
| Existing destination | Explain replacement unavailable; choose new filename or cancel | Not run |
| Destination appears during transfer | Preserve external file and completed partial; show recovery path | Not run |
| Duplicate target submissions | One can finalize; no replacement by the other | Not run |
| Partial path renamed/replaced externally | Reservation denies changes; handle-based cleanup/rename never touches a substitute | Not run |
| Cleanup failure during Delete or shutdown | Failed record survives Delete; shutdown shows copyable retained paths | Not run |
| Locked/unwritable/full disk | No false completion; useful error/recovery path | Not run |
| Settings missing / corrupt / wrong version | Downloads default and warning where relevant; old file preserved | Not run |
| Save settings failure | Previous settings intact; visible error | Not run |
| Unicode / long filenames and folders | Paths preserved, list scrolls, actionable OS errors | Not run |
| Extended drive/UNC paths | Accept supported filesystem prefixes; reject device namespaces and ADS | Not run |
| Browse/Add/Options keyboard cancellation | Native picker and dialog focus/tab order/mnemonics work | Not run |
| Rapid details open/close | At most one window per JobId; closing leaves transfer active | Not run |
| Explicit Open File / Open Folder | Only eligible existing targets open; shell failures are shown | Not run |
| Close with jobs / decline | Prompt; decline preserves work | Not run |
| Close accepted / repeated close | Responsive cleanup, no duplicate shutdown, no late HWND access | Not run |
| Close during Add/Options/picker | Modal ownership safe; queued close handled after dialog closes | Not run |
| Resize / minimize / long list | No overlap; minimum layout and horizontal scrolling | Not run |
| DPI 100/150/200 percent / monitor move | Fonts, layout, columns and dialog controls remain usable | Not run |
| Keyboard only | Menus, toolbar, tree, table, details, dialogs and accelerators usable | Not run |
| Screen reader smoke test | Controls have useful names/focus feedback | Not run |

## Handoff limits

No binary has been produced. Source review cannot establish whether the GUI
renders, controls are accessible, resource tooling links correctly, or downloads
pass the written tests. Existing-file replacement is deliberately unavailable;
all other manual results remain pending above.

Two minor review items are deferred: settings input is read before its 64-KiB
size check, and the ordinary transfer-details static text does not scroll for
very long paths. The shutdown recovery dialog does scroll and supports copying.

## Toolbar icon update

Added five original outline glyphs for Add URL, Stop, Stop All, Delete Entry and
Options. Native toolbar buttons place the icon above its text label. Each icon
has transparent 32/40/48/64/96-pixel frames and a separate disabled variant.
Image lists are rebuilt for the window's DPI and detached before destruction.
Resources live in `crates/win32/resources/icons`; the reproducible Windows drawing
script is `scripts/generate-toolbar-icons.ps1`. The asset-only preview is
`docs/design/rusty-toolbar-icons.png` and was visually inspected. Resource file
existence, ICO frame bounds and Rust formatting are checked by the source script.
The application was not built or launched to validate toolbar rendering.

## Two-step Add Download update

The address dialog now has a native editable address box, optional authorization
checkbox, login/password fields, and OK/Cancel. The second Download File Info
dialog contains a read-only selectable URL, editable category chooser, Save As
and native Browse, category folder preference, description and three actions.
Filename suggestions use the decoded URL path, sanitize invalid filename syntax
and ignore query/fragment values. No request is made just to open File Info.

Start Download admits normally. Download Later retains a held Waiting entry
outside the admission queue, shown as Download later. Downloads → Start Download
releases it into the normal two-transfer queue. Cancel/remove/shutdown still work,
and deferred jobs are session-only. There is no scheduler or restart persistence.

Category names may be typed in the chooser; built-in type categories remain in
the tree and all entries appear under All Downloads. Explicit category metadata
controls built-in filters; description appears in the table. Remembering a folder
saves only that category's preference after the command queue accepts submission.
Legacy settings lacking category preferences still load. Options preserves them.
Authorization uses HTTP Basic only when enabled and stays private to the retained
engine request, excluded from Debug, snapshots and settings.

Written checks cover safe Unicode filenames, category folder selection and legacy
settings, held admission/explicit start, authorization redaction and a loopback
fixture requiring the expected Authorization header. They were not executed.
Focused independent source review found no blocking issue; its Options-copy
correction is included. Source checks and rustfmt are the permitted verification.

Additional manual cases, all Not run: cancellation at either stage; invalid address
returns to stage one; invalid destination preserves File Info values; optional
login/password enablement and masking; selected category and remembered path;
descriptions in the table; Download Later opens no connection; explicit Start
observes the two-transfer limit; Cancel/Remove of held jobs; shutdown with held
jobs; settings write failure after submission does not duplicate that download.

## Compact reference / automatic status dialog update

The current compact sizing and status-window layout are recorded in
[the reference pass](compact-ui-reference.md). This supersedes earlier typography,
toolbar and window dimensions in this record. Status details are now a selectable,
scrolling expanded panel, addressing the previously deferred long-path clipping
item. The settings whole-file read remains deferred.

The status window opens once per attempt, stays closed after dismissal and reopens
for a retry. Closing the main app suppresses new automatic windows. Native tabs,
accurate status fields, progress, blue values, expandable details and compact
footer follow the supplied layout. The subsequent transfer-controls change below
enables pause/resume and replaces the informational limiter tab with real controls.
The completion checkbox closes successful dialogs only when explicitly enabled.
Original source URL credentials/query/fragment are excluded from the display URL.

Written source tests cover these state/format/privacy contracts but have not run.
Actual pixel comparison, DPI transitions, tab keyboard/focus and simultaneous
dialogs remain Not run. A source-only check is not verification of a 1:1 render.

## Pause/resume and speed limiter update

[Transfer-control behavior and checks](transfer-controls.md) records the current
engine and dialog implementation. Source now includes acknowledged pause,
slot-releasing paused jobs, queued resume, safe conditional range requests,
owned-handle restart for full responses, and live interruptible pacing.
The existing Stop/Stop All, removal and shutdown paths also cancel paused jobs.

Written tests cover actual loopback HTTP range headers and final bytes, ignored
ranges/changed files/weak validators/unknown lengths, invalid ranges, cancelled
paused transfers, live throttle removal, and three-job admission/shutdown. Pure
tests cover validation, pacing interrupts, state transitions and presenter actions.
None were executed; compilation, test execution and GUI interaction remain Not run.


## Chrome capture source implementation, 2026-10-19

[Chrome integration](chrome-integration.md) records the new extension, native host,
secured transport, actor receipts, compact dialog integration and session recovery.
All eight plan components are implemented in source. Authored Rust/JavaScript/Pester
tests remain unexecuted. The source verifier covers all four exact workspace
packages, Rust formatting/parsing, native assets, extension JSON/imports/icons,
JavaScript syntax and PowerShell parsing. `git diff --check` is also required.
Compilation, runtime tests, live GUI/Chrome behavior, wrong-user transport attempts
and registration/unregistration remain Not run; no working binary is claimed.
