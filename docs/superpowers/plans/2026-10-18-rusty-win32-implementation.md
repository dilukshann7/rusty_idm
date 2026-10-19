# Rusty Native Win32 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. The user chooses the execution method before implementation; do not proactively delegate implementation.

**Goal:** Deliver the first Windows GUI download manager with actual OS controls and working, cancellable single-stream HTTP/HTTPS transfers.

**Architecture:** A reusable engine library owns transfers, output files and admission control on a background Tokio runtime. A Win32 GUI binary owns all windows/controls on its message-loop thread and exchanges owned commands/events with the engine. The UI follows the supplied IDM layout while using Rusty's own identity and system styling.

**Tech Stack:** Rust stable, Microsoft `windows` bindings, Tokio, reqwest with rustls TLS, tokio-util cancellation tokens, serde/serde_json for settings, embed-resource for the Windows manifest/resources. Exact mutually compatible stable crate versions must be recorded in Cargo.toml/Cargo.lock during authorized implementation; do not assume an unreleased API.

**Spec:** [Native Windows desktop design](../specs/2026-10-18-rusty-win32-design.md). Read both documents. This implements milestone 1 only; later milestones require separate plans.

## Global Constraints

- Rust and actual Win32 controls through Microsoft's `windows` crate. No egui, Iced or webview UI.
- Standard OS control appearance, system fonts/colors, Common Controls v6 manifest and per-monitor DPI awareness. No hard-coded orange accent or automatic-dark-mode promise.
- Menu bar, toolbar, category TreeView, report-mode ListView, status bar and separate modeless transfer-details window with a native Progress Bar.
- Fixed admission limit of two simultaneous downloads, with additional submissions Waiting.
- List/history stays in memory; persist the default destination only. No tray/background lifecycle yet.
- No pause/resume, automatic network retries, segmentation, named queues, browser integration or private IDM compatibility in milestone 1.
- Never silently overwrite existing files. Deleting a list entry never deletes a completed file.
- All HWND/control access stays on the UI thread. No network/disk work or blocking task joins in a window procedure.
- The user prohibits builds or dev-server launches unless specifically requested. Do not run cargo build, cargo run, cargo test or another compilation-producing command under that restriction.
- This planning request authorizes documentation only: no application scaffolding, dependency installation or product code now.

## Review Focus

1. Destination collision, including another job or external process creating/changing the target while downloading: preserve existing data and require fresh replacement consent (Tasks 2, 4, 7).
2. Cancel/close while a transfer, dialog or UI wake-up is pending: no stranded tasks, freed-window access or lost cancellation completion (Tasks 3, 5, 7).
3. Empty, unknown-length, truncated or encoded HTTP bodies: truthful progress, bounded redirects and no false completion (Tasks 2, 7).
4. Unicode/long paths, invalid Windows filenames and settings corruption: actionable errors and safe defaults without destructive cleanup (Tasks 1, 2, 6).
5. DPI changes, minimized windows, long filenames and keyboard-only use: controls remain usable and selected jobs remain correctly associated after filtering (Tasks 5–7).

## Execution and verification policy

The repository currently contains investigation documents/evidence, no Cargo workspace or product source. Preserve all analysis files and existing fixtures. Check whether Git exists before discussing commits; do not initialize a repository, create a worktree, stage unrelated files or commit without user authorization. Each task can form one logical commit if commits are authorized later.

All compile/test/launch commands below are **deferred commands**, not authorization to run them. When implementing under the current restriction, write meaningful tests first, implement source, perform allowed source checks and report runtime/compile verification as pending. If the user authorizes tests, follow the indicated red/green cycle task by task. Do not repeatedly broaden testing after the relevant checks pass. GUI launch remains separately subject to the user's instructions. No application dev server is needed.

## File structure

| Files to create during implementation | Responsibility |
| --- | --- |
| `Cargo.toml`, `.gitignore` adjustment | Workspace with `crates/engine`, `crates/win32`; ignore target/build outputs, preserve current exclusions. |
| `crates/engine/Cargo.toml`, `src/lib.rs`, `src/model.rs` | Public request, state, identity, snapshot and error contracts. |
| `crates/engine/src/output.rs`, `src/transfer.rs` | Reserved partial files/finalization and single HTTP transfer. |
| `crates/engine/src/service.rs`, `src/events.rs` | Admission, commands, cancellation, snapshots and bounded event delivery. |
| `crates/engine/tests/output.rs`, `transfer.rs`, `service.rs`, `support/mod.rs` | Output, HTTP and lifecycle tests; test-only loopback fixture. |
| `crates/win32/Cargo.toml`, `build.rs`, `resources/app.rc`, `app.manifest` | Binary dependencies, embedded Common Controls/DPI manifest, version resources. |
| `crates/win32/src/main.rs`, `lib.rs`, `app.rs`, `platform.rs` | Startup, testable module exports, UI state, safe resource ownership and callback/window lifetime. |
| `crates/win32/src/controls.rs`, `commands.rs`, `presenter.rs` | Native shell, stable command IDs, job-to-row mapping, categories and display formatting. |
| `crates/win32/src/bridge.rs`, `dialogs.rs`, `details.rs`, `settings.rs` | Engine notifications, native dialogs, progress details and settings persistence. |
| `docs/native-gui-validation.md` | Manual acceptance matrix and honest verification status. |

Choose targeted `windows` feature flags for APIs actually used, including windows/messages, controls, shell, COM, DPI and resource functions. Both crates may use Windows bindings for Windows-specific output handling, but engine public job/state contracts must not expose HWNDs.

## Shared interfaces

Define these in Task 1 and reuse their names. Rust field syntax below is a contract outline, not implementation source.

```text
JobId(u64)
JobState = Waiting | Running | Cancelling | Completed | Failed | Cancelled
DownloadRequest { url: Url, destination: PathBuf, replacement: ReplacementPolicy }
ReplacementPolicy = Deny | Approved(DestinationIdentity)
JobSnapshot { id, filename, source_host, destination, state, received: u64,
              total: Option<u64>, bytes_per_second: u64,
              eta_seconds: Option<u64>, last_try: Option<SystemTime>,
              error: Option<JobError> }
JobError { kind: ErrorKind, message: String, retained_partial: Option<PathBuf> }
ErrorKind = InvalidRequest | Http | Network | LengthMismatch | DestinationConflict
            | DestinationChanged | Io | Cleanup | ShuttingDown
Command = Submit(DownloadRequest) | Cancel(JobId) | CancelAll
          | Remove(JobId) | Retry(JobId) | Shutdown
Event = JobChanged(JobSnapshot) | JobRemoved(JobId) | ShutdownComplete
```

`DestinationIdentity` is an opaque Windows file identity/version token captured by the destination selection/output layer, not merely a filename. Error messages displayed/logged must redact URL credentials and sensitive query/header values. Original request URLs remain engine-private; source host and sanitized display URL can be exposed deliberately.

`EngineHandle::try_send(&self, command: Command) -> Result<(), CommandError>` is nonblocking and reports a full/closed command queue; CommandError = QueueFull | Closed. A submission beyond the retained-job limit returns a visible non-job service-error event: add Event::CommandRejected { message: String } for actor-side failures. `spawn_engine(wake: Arc<dyn Fn() + Send + Sync>) -> EngineHandle` owns the background thread/runtime. `EngineHandle::drain_events(&self) -> Vec<Event>` retrieves pending owned events. `EngineHandle::join(self) -> Result<(), EngineJoinError>` consumes and joins the runtime owner after ShutdownComplete, outside Win32 callbacks. GUI action handlers never call Tokio block_on.

### Task 1: Model, workspace and input validation

**Files:** Create workspace/Cargo manifests and engine `lib.rs`, `model.rs`; add model unit tests in `model.rs`. Adjust `.gitignore` narrowly.

**Interfaces:** Produces the shared contracts, `validate_request(url: &str, destination: PathBuf, replacement: ReplacementPolicy) -> Result<DownloadRequest, JobError>` and `transition(state: JobState, action: StateAction) -> Result<JobState, TransitionError>`. Define StateAction variants Submit, Admit, Cancel, TransferSucceeded, TransferFailed and CancellationFinished. CommandError and TransitionError are concrete error enums.

- [ ] Write table-driven model tests: reject non-HTTP(S), empty/malformed URLs, missing filename, Windows device names and alternate-data-stream filename syntax; preserve valid Unicode paths; reject Running→Completed after cancellation already owns the terminal outcome. Define explicit transition tests for Waiting cancellation and terminal-job removal/retry eligibility.

  Name the state tests `cancel_waiting_is_terminal`, `running_cancel_waits_for_cleanup` and `completion_cannot_override_cancellation`. Their core assertions are:

  ```rust
  assert_eq!(transition(JobState::Waiting, StateAction::Cancel).unwrap(), JobState::Cancelled);
  assert_eq!(transition(JobState::Running, StateAction::Cancel).unwrap(), JobState::Cancelling);
  assert!(transition(JobState::Cancelling, StateAction::TransferSucceeded).is_err());
  ```
- [ ] When test execution is authorized, run `cargo test -p rusty-engine --lib model`; before implementation expect failing/missing model behavior, afterward all assertions pass. Otherwise record both runs as deferred.
- [ ] Implement the contracts and validator. Validate filename rules without inventing a short arbitrary path-length limit. Destination existence/writability remains an I/O check in Task 2.
- [ ] Verify source imports/exports and manifest feature choices without compiling. Record dependency versions when Cargo is first authorized to resolve them; commit Cargo.lock for the binary workspace then.

### Task 2: Safe output and a single-stream transfer

**Files:** Create engine `output.rs`, `transfer.rs`, `tests/output.rs`, `tests/transfer.rs`, `tests/support/mod.rs`.

**Interfaces:** Consumes DownloadRequest, JobError and ReplacementPolicy. Produces `transfer(request: DownloadRequest, cancel: CancellationToken, progress: ProgressSink) -> Result<TransferOutcome, JobError>`, where TransferOutcome is Completed { bytes: u64 } or Cancelled. ProgressSink is an owned thread-safe callback accepting `TransferProgress { received: u64, total: Option<u64> }`; it may coalesce/drop intermediate progress but not final results. Export `inspect_destination(path: &Path) -> Result<Option<DestinationIdentity>, JobError>` for native replacement approval. Output module owns its temporary paths/handles and exposes prepare/write/finalize/abort internally.

- [ ] Write output tests asserting collision-safe partial creation, original destination unchanged on Deny, cancellation removes only this job's partial, flush/rename failure never reports Completed, and cleanup failure retains the actual partial path. Use unique temporary directories; never operate on the user's IDM test files.
- [ ] Write test-only HTTP cases for exact position-dependent bytes, zero-byte success, unknown length, redirect success, redirect loop, 404/503 failure, interrupted body, cancellation mid-body and Content-Encoding/length consistency. Assert completed bytes/hash, no final output on failure and no invented percentage for unknown total. Disable automatic body decompression/request identity encoding so length checks refer to stored bytes; reject unsupported encoded representations rather than comparing mismatched lengths.
- [ ] Implement streaming with a finite redirect limit of 10, system proxy behavior documented, TLS verification enabled and cancellable request/body operations. Require successful final response status; no app-level automatic retries. Prefer an interruptible 30-second no-body-progress timeout over a total-duration timeout that breaks large downloads. Record this as Rust policy, not an IDM-recovered default.
- [ ] Implement output finalization: Deny uses a no-replace operation. Approved replacement must revalidate destination identity and preserve the old file through a recoverable native replacement/backup transaction; a changed target fails DestinationChanged and retains the completed partial for renewed consent. Never fall back to remove-target-then-rename. Test external-target changes and duplicate submissions. If the chosen Win32 primitive cannot guarantee the required identity/race behavior, disable that replacement path with a clear error until its safe transaction is verified; normal new-name downloads must still work.
- [ ] When authorized, run `cargo test -p rusty-engine --test output --test transfer`. Expected: fixture cases pass with exact bytes and originals preserved. The fixture is loopback-only and exists for each test's lifetime; starting it requires authorization for these runtime tests, not a dev-server workaround.

### Task 3: Admission, cancellation and bounded progress delivery

**Files:** Create engine `service.rs`, `events.rs`, `tests/service.rs`; update engine exports.

**Interfaces:** Produces spawn_engine/EngineHandle and Command/Event from the shared interfaces. Own monotonically allocated JobIds and retain original requests privately for Retry. GUI removal never invokes filesystem deletion of completed outputs.

- [ ] Write tests using a controllable fake transfer executor: three submissions produce two Running and one Waiting; cancellation frees a slot only after transfer cleanup; waiting cancellation never opens a socket; Retry of Failed/Cancelled starts from zero with fresh destination validation; Remove during Running initiates cancellation and emits JobRemoved only after cleanup.
- [ ] Add shutdown tests: stop admission immediately, cancel Waiting/Running, reject later submissions, emit terminal events before ShutdownComplete. Race Cancel against transfer success and assert exactly one terminal state with no duplicated completion/removal.
- [ ] Implement the actor and a command capacity of 64. Limit retained job records to 1,000 with a visible submission error at the limit; users can remove inactive records. Coalesce progress to at most 10 updates/second/job and keep bounded latest snapshots, with terminal transitions durably pending until drained. Never block engine cancellation waiting for the GUI to consume events. Recompute aggregate speed only from active jobs.
- [ ] Ensure wake notifications are coalesced with an atomic pending flag. Drain/clear/recheck must avoid a lost wake-up when an event arrives during UI draining. Verify burst progress and shutdown still deliver the final state when the UI is temporarily not consuming.
- [ ] When authorized, run `cargo test -p rusty-engine --test service`. Expected: deterministic admission, cancellation, backpressure and shutdown tests pass without external network access.

### Task 4: Presenter, categories and native command policy

**Files:** Create GUI `presenter.rs`, `commands.rs` with pure unit tests; create GUI manifest and `src/lib.rs` exports for those tests. The binary's main module calls the library's run entry point.

**Interfaces:** Consumes JobSnapshot/Event. Produces `ViewModel::apply(Event)`, `ViewModel::visible_jobs(Category) -> Vec<JobId>`, `ViewModel::actions(Option<JobId>) -> ActionAvailability`, and `format_snapshot(&JobSnapshot) -> RowText`. RowText owns strings for the seven specified table columns. Define Category = All, Compressed, Documents, Music, Programs, Video, Unfinished, Finished.

- [ ] Write tests asserting filters preserve JobId association when rows reorder; Completed is Finished; Waiting/Running/Cancelling/Failed/Cancelled are Unfinished; unknown total gives no percent/ETA; zero-byte Completed is truthful; remove never means delete-file; actions cannot Retry/Remove a stale or incompatible selection.
- [ ] Implement case-insensitive extension mappings: zip/7z/rar/tar/gz; pdf/txt/doc/docx/xls/xlsx/ppt/pptx; mp3/wav/flac/m4a; exe/msi/msix; mp4/mkv/webm/avi. Unmatched files remain in All and the appropriate status group. This mapping is a Rusty design choice.
- [ ] Map command IDs centrally, including Add URL, Stop, Stop All, Delete Entry, Retry, Open File, Open Folder, Options, Exit, transfer details and About. Keep native menu labels/mnemonics consistent; unsupported Resume/Scheduler commands are absent.
- [ ] Keep Last Try Date empty until an attempt starts, formatted in local time; Description shows sanitized failure text or state detail. Sort/filter presentation must never mutate engine identity or truncate stored paths.
- [ ] When authorized, run `cargo test -p rusty-win32 --lib presenter`. Expected: mapping/format/action tests pass; this command compiles and is deferred under the current restriction.

### Task 5: Native window shell and safe event bridge

**Files:** Create GUI `main.rs`, `app.rs`, `platform.rs`, `controls.rs`, `bridge.rs`, `build.rs`, `resources/app.rc`, `resources/app.manifest`; update manifests.

**Interfaces:** Consumes EngineHandle and ViewModel. Produces `run() -> Result<(), AppError>` called by main; AppState owns engine/view/control state. `bridge::WakeTarget` posts a payload-free WM_APP notification to a stable main HWND and can be disabled before destruction. Define all child IDs and WM_APP values centrally in commands/bridge.

- [ ] Embed Common Controls v6 and PerMonitorV2 DPI declarations. Initialize common-control classes and COM for native pickers on the UI thread, with deterministic cleanup and error reporting. Avoid adding a second application manifest through conflicting linker/build settings.
- [ ] Build genuine menu, Toolbar, TreeView, report ListView and status-bar controls. Initial logical window size 1250×650, navigation width approximately 180. Add all seven table columns, horizontal scrolling, full-row selection, tooltips and system GUI fonts. Use original/simple system icons and enable buttons from ActionAvailability.
- [ ] Implement WM_NCCREATE/WM_NCDESTROY ownership safely, checking return values and avoiding Rust unwinding through callbacks. RAII must distinguish owned versus borrowed/shared GDI/icon handles. Route WM_COMMAND/WM_NOTIFY and ignore unknown/stale control IDs safely.
- [ ] Handle WM_SIZE and WM_DPICHANGED using DPI-scaled logical metrics; accept the suggested DPI rectangle and recreate owned fonts appropriately. Preserve selection by JobId, not row number, across updates/filters. Define a minimum client layout and allow scrolling rather than overlapping controls.
- [ ] Implement payload-free event wake-up, UI-thread event draining and targeted row/status updates. Disable WakeTarget before HWND destruction; no borrowed pointers in WPARAM/LPARAM. Keep AppState alive until window callbacks have finished and never wait for runtime termination inside them.
- [ ] Add bridge tests for concurrent publish/drain and disabled target using a fake notifier. Add the DPI/keyboard/lifetime scenarios to the manual validation matrix in Task 7. Defer real-window checks and `cargo check -p rusty-win32` until explicitly authorized; do not claim the shell renders from source review alone.

### Task 6: Native dialogs, settings and transfer details

**Files:** Create GUI `dialogs.rs`, `details.rs`, `settings.rs`; wire app/controls/commands.

**Interfaces:** Produces `show_add_download(owner: HWND, default_dir: &Path) -> Result<Option<DownloadRequest>, AppError>`, `show_options(owner: HWND, settings: &Settings) -> Result<Option<Settings>, AppError>`, and `DetailsWindows::show_or_update(owner: HWND, snapshot: &JobSnapshot)`. Settings { schema_version: 1, default_destination: PathBuf }; `settings::load() -> Result<Settings, SettingsError>`, `settings::save(settings: &Settings) -> Result<(), SettingsError>` and `settings::defaults() -> Result<Settings, SettingsError>` return explicit errors, allowing the UI to show a recoverable warning and choose defaults.

- [ ] Implement native Add URL and Options dialogs with correctly ordered Edit/Button controls, labels, mnemonics and IsDialogMessage handling. Native Browse uses a Windows picker. Existing destination prompts explain replacement and allow choosing another path/cancelling; capture approval identity through the same output-layer validation used by transfer finalization.
- [ ] If finalization reports DestinationChanged, retain/show the partial path and explain that the target changed; do not silently retry with replacement consent from an older file. Retry from zero requires fresh destination validation. Command queue full/closed errors stay visible without duplicating a submission.
- [ ] Persist settings at `%LOCALAPPDATA%\RustyIDM\settings.json` through a unique temporary file and atomic replacement. Default to the Windows Downloads known folder. Test malformed/unknown-version settings, Unicode destination, unwritable settings directory and preserving the old settings on a failed save. Do not save URLs, credentials or history here.
- [ ] Implement one modeless details window per JobId with native Progress Bar and supported buttons. Unknown-length Running uses marquee; known length uses a bounded scaled progress range without u64 overflow. Display received/total, speed and ETA from snapshots. Closing details leaves the transfer active; Stop sends Cancel. Prevent duplicate windows and clear their handles on destruction.
- [ ] Add explicit Open File/Open Folder commands only for existing eligible paths; check native shell-operation results. Open File uses a user command, never automatic execution upon completion. Stop All cancels Running and Waiting; Delete Entry follows Task 3 removal lifecycle.
- [ ] Add settings and progress-scaling unit tests. When authorized, run `cargo test -p rusty-win32 --lib`; document manual tests for pickers, focus, dialog cancellation and opening files without claiming automated shell execution tests were performed.

### Task 7: Shutdown, acceptance and handoff

**Files:** Update GUI app/bridge/details; create `docs/native-gui-validation.md`; update the root brief with implementation/verification status only when warranted.

**Interfaces:** WM_CLOSE initiates ConfirmStop or exit according to engine state. On acceptance, AppState enters Closing, disables submission, sends Shutdown and continues pumping messages. ShutdownComplete closes detail windows and destroys the main window; the runtime owner joins after the message loop returns. Repeated close requests cannot initiate duplicate shutdowns.

- [ ] Test shutdown state handling with fake engine events: reject further submissions, remain responsive while cancellation is pending, handle failure/terminal races, disable notifications before destroying windows, and exit only after cleanup completes. Declining the prompt leaves active work unchanged.
- [ ] Write the manual acceptance matrix: empty startup; three downloads with two admitted; known/unknown/zero-length transfer; HTTP error; disconnect/timeout; cancel one/all/waiting; remove entry preserving file; approved/rejected/changed destination; settings corruption; Unicode/long filename; rapid details open/close; repeated main-window close; resize/minimize; 100/150/200 percent DPI and monitor change; keyboard-only menus/dialogs; screen-reader smoke check. Mark every case Not run initially.
- [ ] Run allowed source checks first: Rust formatting checks when available, review manifest/resource paths, verify documentation links and inspect unsafe ownership boundaries. Do not use cargo fmt as a pretext for compiling or fetching/building dependencies.
- [ ] Only with explicit test/build authorization, run the focused suites above followed by one workspace check/test pass justified by integration changes: `cargo check --workspace` and `cargo test --workspace`. Record actual outputs and platform; fix failures before claiming success. Do not run a release build or repeated broad passes without need.
- [ ] Only with explicit launch authorization, run `cargo run -p rusty-win32` and execute the manual matrix against disposable fixture/files. No ordinary IDM state or browser profiles involved. Verify hashes against position-dependent expected bytes; no success claim based on a progress bar reaching 100 percent.
- [ ] Record implemented versus source-reviewed versus runtime-verified capabilities and all remaining failures. If compilation/launch was not authorized, deliver source with those checks explicitly pending. Hand over the GUI/engine milestone; do not automatically start pause/resume or other deferred work.

## Self-review and execution handoff

Spec coverage: native layout/controls and DPI → Tasks 4–6; transfer integrity and collision handling → Task 2; two-job admission/cancel/remove/retry → Task 3; dialogs/settings/progress → Task 6; lifecycle and acceptance → Task 7. All five Review Focus cases have owning tests/checks. Shared Command/Event/JobSnapshot names and EngineHandle operations are consistent across tasks.

No tests, builds, fixture server, application launch or dependency installation were performed while writing this plan. No product code is included. Recommend native execution in this chat because the seven tasks share contracts and Win32 lifetime assumptions; one implementer can keep them consistent without repeated handoffs, followed by an independent final review if authorized. The alternative is a fresh implementing agent and reviewer per task, which adds review boundaries and context overhead. User reviews this plan and chooses the execution method before work starts.
