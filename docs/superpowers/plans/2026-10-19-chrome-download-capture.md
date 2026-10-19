# Chrome Download Capture Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. The user chooses the execution method before implementation; do not proactively delegate implementation.

**Goal:** Add Chrome download capture and a Download with Rusty context menu, backed by a Windows native host and the existing compact desktop workflow.

**Architecture:** A Manifest V3 service worker pauses eligible Chrome downloads and submits correlated capture requests through a native messaging host. A user/session-scoped named pipe delivers requests to the single GUI instance; the GUI gathers File Info/duplicate decisions and obtains an actor acknowledgement before the extension cancels Chrome's copy. Bounded leases, idempotent receipts and a session recovery ledger handle cancellation, disconnects and worker restarts.

**Tech Stack:** Existing Rust 2021 workspace and pinned Tokio 1.48.0, tokio-util 0.7.16, windows 0.62.2, serde 1.0.228, serde_json 1.0.145 and url 2.5.7; Chrome Manifest V3/JavaScript ES modules; Node built-in test runner for authored JS tests; PowerShell setup scripts. No JS framework, bundler or HTTP server.

**Spec:** [Chrome capture design](../specs/2026-10-19-chrome-download-capture-design.md). Read both. This is one integrated feature with sequential, independently testable component tasks; the extension and desktop bridge share one protocol.

## Global Constraints

- Windows 10/11 and Chrome 120+; Manifest V3, plain JavaScript modules, no bundler.
- Capture is an explicit extension preference, off initially.
- Current user instruction: no builds, Rust tests, application, fixture, browser or dev-server launches unless explicitly requested.
- Native host name: `dev.rusty_idm.chrome`.
- Application ceiling: **65,536 payload bytes** in either direction, checked before allocation.
- Limits: URL **8,192 UTF-8 bytes**, filename hint **255 UTF-16 units**, **32** pending captures, **256** completed receipt records with **10-minute** retention.
- Native startup/connect timeout: **10 seconds**. Entire capture lease: **120 seconds**, including dialogs and queues.
- Preserve URL queries internally; redact them, userinfo and fragments in UI/logs. Filenames are hints, never save paths.
- All HWND/control operations stay on the GUI thread; callbacks only queue owned signals.
- No Chrome profile edits, extension auto-install, registry writes or host registration during implementation without a separate explicit setup request.
- Existing two-active-download admission, pause/resume, duplicate choices and guarded file replacement remain authoritative. Never cancel a reused existing job on browser fallback.
- Keep feature-scoped commits. Do not push unless requested.

## Review Focus

1. Lost acknowledgement after actor acceptance must reuse the same ID, expose status and avoid a second submission — Tasks 2 and 7.
2. A download that completes or is paused by the user during handoff must keep its Chrome file/pause state — Tasks 5 and 7.
3. Expired File Info or duplicate prompts must not submit after Chrome has resumed — Tasks 1 and 4.
4. Identical URLs from concurrent GET/POST/redirect requests must not be guessed into an automatic capture — Task 5.
5. Another Windows user, spoofed native origin or spaced/Unicode executable path must not hijack or break delivery — Tasks 3 and 6.

## Verification convention

Each task includes RED/GREEN commands for a later authorized verification run.
**Writing this plan or approving implementation does not authorize those commands.**
Under the current restriction, author the tests, inspect source, run only permitted
source checks and mark execution Not run. Never replace this boundary with a claim
that formatting proves compilation. JS test commands are deferred too, to keep a
single explicit execution boundary for this integration.

During implementation, `scripts/verify-source.ps1`, rustfmt, offline
`cargo metadata --locked --offline --no-deps`, JSON parsing, PowerShell parser
checks, `node --check` and `git diff --check` are permitted source checks. They
must not load the extension or execute registry/setup actions.

## File map and dependencies

| Files | Responsibility |
| --- | --- |
| `crates/browser-bridge/src/{lib,protocol,framing,lease}.rs` | Shared validated messages, bounded codec, cancellation/acceptance lease |
| `crates/browser-bridge/src/windows/{mod,identity,pipe}.rs` | SID/session identity, endpoint ownership and pipe I/O |
| `crates/browser-bridge/tests/{protocol,framing,lease,windows_pipe}.rs` | Protocol and transport contracts |
| `crates/engine/src/submissions.rs`, `crates/engine/tests/submissions.rs` | Correlated actor receipts and idempotent actions |
| `crates/native-host/src/{main,host,config}.rs`, `crates/native-host/tests/host.rs` | Chrome console host, validated configuration, framed relay |
| `crates/win32/src/{browser_integration,download_intent}.rs` and corresponding `_tests.rs` | Capture inbox/controller and submission intent separation |
| `crates/win32/src/{app,dialogs,bridge,main}.rs` | Single-instance lifecycle, prefilled File Info, deadline-aware dialogs and wake signal |
| `extensions/chrome/{manifest.json,service-worker.js,capture-policy.js,handoff.js,recovery.js,native-client.js}` | Browser policy, orchestration, recovery and native connection |
| `extensions/chrome/{popup.html,popup.js,popup.css,icons/*}` | Compact controls/status and local Rusty branding |
| `extensions/chrome/tests/{capture-policy,handoff,recovery}.test.js` | Browser-adapter state tests without real Chrome |
| `scripts/{register,unregister}-chrome-host.ps1`, `scripts/verify-chrome-source.ps1` | Per-user setup/cleanup and static validation |
| `docs/chrome-integration.md`, `README.md`, `scripts/verify-source.ps1` | Handoff/setup instructions and updated source checks |

New package names are `rusty-browser-bridge` and `rusty-native-host`; the host
binary is `rusty-native-host.exe`. Add both workspace members. `browser-bridge`
has no dependency on engine or GUI; engine imports its lease/ID types, native-host
and GUI use its transport. No dependency cycle. Reuse pinned workspace packages;
add only required Windows feature flags after checking cached declarations.

---

### Task 1: Define the shared protocol, bounded codec and capture lease

**Files:** Create `crates/browser-bridge/Cargo.toml`, `src/{lib,protocol,framing,lease}.rs`, `tests/{protocol,framing,lease}.rs`. Modify `Cargo.toml`, `Cargo.lock`, `scripts/verify-source.ps1`.

**Interfaces:**
- `RequestId`: UUID newtype, parsed with `RequestId::parse(&str) -> Result<Self, ProtocolError>`; exact equality and redacted Debug.
- `CaptureRequest { id: RequestId, url: String, filename_hint: Option<String>, source: CaptureSource, download_id: Option<u64>, expires_at_ms: u64 }`; custom Debug omits URL/hint.
- CaptureRequest is the validated internal shape: the envelope ID becomes its ID; the wire capture payload has no second ID. Generate IDs with browser `crypto.randomUUID()` and validate canonical UUID text in Rust without a new UUID dependency.
- `Operation::{Ping,Activate,Capture(CaptureRequest),Status{capture_id:RequestId},Abort{capture_id:RequestId}}` and `Reply` with the spec's version, statuses, receipt fields and decline codes.
- `read_frame<R: Read>(&mut R) -> Result<Option<Vec<u8>>, FrameError>`; clean EOF is None, partial prefix/body is an error. `write_frame<W: Write>(&mut W, &[u8]) -> Result<(), FrameError>` flushes complete frames.
- `CaptureLease::new(deadline: Instant)`, `revoke() -> LeaseState`, `state() -> LeaseState`; `try_accept<T,E>(action: impl FnOnce() -> Result<T,E>) -> Result<T, LeaseFailure<E>>` serializes revocation/deadline checks with acceptance. `LeaseState::{Pending,Accepted,Revoked,Expired}`; errors distinguish already accepted and action failure.

- [ ] **Step 1 — Write contract tests:** `frame_uses_utf8_byte_length_and_handles_fragmented_reads` asserts the prefix for a Tamil URL is its encoded byte count; `oversize_prefix_is_rejected_before_reading_body` passes 65,537 and a reader that panics on body read. Test clean EOF/truncated prefix/truncated body/multiple frames. `capture_validation_rejects_unsafe_fields` covers unknown version/op/fields, oversized URL/hint, userinfo and control characters. `lease_expiry_and_revoke_prevent_acceptance` asserts the action closure is never called; `accept_and_revoke_have_one_order` asserts only one state wins.
- [ ] **Step 2 — Deferred RED:** `cargo test -p rusty-browser-bridge --test framing --test protocol --test lease`; expect missing API failures before implementation. Under the current restriction record Not run.
- [ ] **Step 3 — Implement those exact interfaces:** use serde `deny_unknown_fields`, explicit tagged wire variants and the spec limits. Preserve queries; never serialize errors with input values. A successful action transitions the lease to Accepted; action errors create no acceptance. Keep this task portable; declare the Windows transport module only when Task 3 creates its files. Update source-check workspace validation by exact package-name set, ultimately `{rusty-engine,rusty-win32,rusty-browser-bridge,rusty-native-host}`, rather than silently accepting arbitrary members. During Task 1 allow the defined three members until Task 3 adds the fourth.
- [ ] **Step 4 — Deferred GREEN:** repeat Step 2; expect all cases pass. Run allowed source checks and update the lock without compiling or changing existing version pins. If dependencies are absent from the cache, report the specific resolution limitation.
- [ ] **Step 5 — Commit:** stage only shared crate, workspace/lock and source-check changes; `feat(browser): define capture protocol and bounded handoff leases`.

### Task 2: Add actor-confirmed, idempotent submission receipts

**Files:** Create `crates/engine/src/submissions.rs`, `crates/engine/tests/submissions.rs`. Modify `crates/engine/{Cargo.toml,src/lib.rs,src/service.rs}`.

**Interfaces:**
- Consume Task 1 `RequestId` and `CaptureLease`.
- Export `SubmissionAction::{Submit(DownloadRequest),Show(JobId),Activate(JobId),Restart(JobId)}`, `SubmissionReceipt { id: RequestId, job_id: JobId, created_new: bool }`, `SubmissionStatus::{Pending,Accepted(SubmissionReceipt),Declined(SubmissionError),Unknown}`.
- Export `ExternalCommand::{Apply{id,action,lease,reply},Status{id,reply},CancelOwned{id,reply}}`; each `reply` is a Tokio oneshot sender of its corresponding result. `SubmissionError::{Expired,Revoked,Busy,Closing,NotFound,InvalidAction,RecoveryRequired}` and `CancelOwnedResult::{Cancelled,NotOwned,AlreadyFinished,Unknown}`.
- `EngineHandle::client() -> EngineClient` returns a clonable command-only client without thread/join ownership. `EngineClient::try_external(ExternalCommand) -> Result<(), CommandError>` feeds the existing bounded command channel through `Command::External`; EngineHandle delegates to it for tests. This lets the receipt/control relay run without borrowing GUI AppState.

- [ ] **Step 1 — Write actor tests:** `queued_command_is_not_an_acknowledgement` asserts no receipt before actor processing; `same_id_returns_same_job_once` asserts one executor admission and one JobId; `different_payload_for_same_id_is_rejected` cannot change destination/action; `revoked_or_expired_request_never_admits` asserts zero file/network preparation. `status_survives_event_drain` obtains the same receipt afterward. `fallback_cancels_only_new_owned_job` cannot cancel an existing reused entry. Apply, status and cancellation are keyed by the original capture ID; replay of the capture keeps its original receipt, while status can report subsequent cancellation. Test held jobs, duplicate/recovery rejection, full queues, closing and dropped result receivers.
- [ ] **Step 2 — Deferred RED:** `cargo test -p rusty-engine --test submissions`; expect missing command/receipt APIs. Record Not run until authorized.
- [ ] **Step 3 — Implement the actor path:** refactor reusable admission/activate/restart logic without changing manual commands. Apply through `CaptureLease::try_accept`, record the receipt before replying and admit only once. Store at most 256 completed receipts for 10 minutes; reject new tracked requests if capacity is still occupied instead of evicting live receipts. Retain result identity even if the reply receiver disappears. CancelOwned maps only a newly created tracked job; reused jobs never become abortable by the extension. This layer does not show dialogs or log URLs.
- [ ] **Step 4 — Deferred GREEN:** repeat Step 2 and the existing `cargo test -p rusty-engine --test service --test controls`; expect old admission/cancellation contracts unchanged. Perform allowed formatting/source checks now.
- [ ] **Step 5 — Commit:** `feat(engine): acknowledge and deduplicate browser submissions` with these source/tests only.

### Task 3: Implement Windows IPC and the Chrome native host

**Files:** Create `crates/browser-bridge/src/windows/{mod,identity,pipe}.rs`, `tests/windows_pipe.rs`; `crates/native-host/{Cargo.toml,src/main.rs,src/host.rs,src/config.rs,tests/host.rs}`. Modify workspace manifests/lock and source-check package set.

**Interfaces:**
- `UserSession::current() -> io::Result<UserSession>`, `.pipe_name() -> String`, `.mutex_name() -> String` use user SID and Windows session ID.
- `RequestRouter: Send+Sync` exposes `dispatch(operation: Operation, replies: SyncSender<Reply>) -> Result<(),BridgeError>`. `PipeServer::start(session: UserSession, router: Arc<dyn RequestRouter>) -> io::Result<PipeServer>` routes without touching windows. `IncomingCapture { capture: CaptureRequest, lease: CaptureLease, reply: SyncSender<Reply> }` represents GUI work only. Pending/final replies use the same envelope ID.
- `PipeClient::connect(session, timeout: Duration) -> io::Result<PipeClient>`; `.request(Operation) -> Result<ReplyStream, BridgeError>`. Shutdown wakes bounded outstanding callers and joins outside window callbacks.
- `HostConfig::load(path: &Path) -> Result<HostConfig, HostError>`, `.validate_origin(&str) -> Result<(),HostError>`; `run_host<R: Read+Send,W: Write+Send>(input: &mut R, output: &mut W, config: &HostConfig, caller_origin: &str) -> Result<(),HostError>`. Main loads `%LOCALAPPDATA%\RustyIDM\browser\host-config.json`; do not guess a sibling configuration file.

- [ ] **Step 1 — Write transport/host tests:** wrong user/logon and remote client are refused; an occupied unexpected endpoint fails closed; fragmented reads and disconnects produce bounded errors; 33rd pending request declines busy. `status_and_abort_are_processed_while_capture_pending` interleaves IDs and verifies independent replies without waiting for a modal dialog. `unicode_spaced_gui_path_is_not_shell_expanded` inspects launch executable/argv and proves URLs are absent. `stdout_contains_only_frames` decodes every output byte. Spoofed origin/config paths fail before GUI launch. Test ping with GUI absent and missing native configuration.
- [ ] **Step 2 — Deferred RED:** `cargo test -p rusty-browser-bridge --test windows_pipe` and `cargo test -p rusty-native-host --test host`; expect missing transport/host APIs. These tests start processes/pipe fixtures only after authorization.
- [ ] **Step 3 — Implement transport and host:** verify cached windows/Tokio declarations for token identity, security descriptors, named-pipe security attributes and remote rejection; add only needed Windows features. Use native pipe security/current-logon ACL and verify peer token identities. Keep frame lengths bounded on async pipe I/O too. Host stdin reading and one framed stdout writer must continue while GUI requests are pending; bounded per-ID routing cannot put status/abort behind a modal capture. The host has console/stdin/stdout entry point, never the GUI subsystem. Validate one exact extension origin. Load absolute installed binary paths from generated host config; launch `Command::new(gui_exe).arg("--browser-integration")`, never a shell. Retry GUI connection within 10 seconds, without unbounded process spawning. Ping does not launch GUI.
- [ ] **Step 4 — Deferred GREEN:** repeat Step 2; all framing/origin/ownership/path cases pass. Inspect stdout/stderr separation and perform permitted source checks.
- [ ] **Step 5 — Commit:** `feat(browser): add Windows native messaging host and pipe transport`.

### Task 4: Integrate the capture inbox with compact native dialogs

**Files:** Create `crates/win32/src/{browser_integration,download_intent}.rs`, `browser_integration_tests.rs`, `download_intent_tests.rs`. Modify `crates/win32/{Cargo.toml,src/lib.rs,src/app.rs,src/dialogs.rs,src/bridge.rs,src/main.rs}` and `download_form.rs` only for shared filename sanitization.

**Interfaces:**
- `DownloadIntent::{New(dialogs::AddedDownload),Existing{job_id:JobId,action:ExistingAction},Cancelled}`, `ExistingAction::{Show,Activate,Restart}`; decisions contain no engine side effects.
- `dialogs::show_browser_download(owner: HWND, settings: &Settings, capture: &CaptureRequest, lease: &CaptureLease) -> Result<Option<AddedDownload>, AppError>` opens prefilled File Info, bypassing Address.
- Replace `AppState::resolve_duplicate(...) -> Result<bool,AppError>` with `resolve_download_intent(...) -> Result<DownloadIntent,AppError>`; manual ADD executes that intent through existing commands, browser capture translates it into Task 2 SubmissionAction.
- `BrowserController::new(engine: EngineClient, wake: Arc<dyn Fn()+Send+Sync>) -> Self`, `.router() -> Arc<dyn RequestRouter>`, `.drain() -> Vec<IncomingCapture>` and `.submit(id: RequestId, action: SubmissionAction, lease: CaptureLease, reply: SyncSender<Reply>) -> Result<(),CommandError>`. Its bounded worker relays actor receipts and serves status/abort independently of the GUI; it records immutable replay replies plus current cancellation outcome. Add `Action::Browser` and `BROWSER_WAKE = WM_APP + 3`; use the existing 250-ms housekeeping timer as fallback.
- `AppInstance::acquire_or_activate(&UserSession) -> Result<InstanceOutcome,AppError>`, `InstanceOutcome::{Primary(AppInstance),Forwarded}`; hold Primary for the app lifetime. A second process sends protocol Activate to the owned endpoint; the GUI queues an activation signal without creating an engine job.

- [ ] **Step 1 — Write controller/intent tests:** a prepared intent sends no engine command; `same_capture_id_shows_one_dialog` asserts duplicate envelopes reuse one pending entry; expired/cancelled prompts never become Submit; a full inbox produces busy, not lost work. Seeded filename hints containing paths/device names/control characters cannot escape the selected folder. File Info cancel, duplicate cancel, all duplicate actions, remembered choice and Download Later map to the correct action. `closing_declines_pending_captures_before_hwnd_destroy`, `exceptional_exit_drops_browser_engine_clients_before_join` and `new_instance_only_activates_existing_gui` pin lifecycle ownership. A lost actor reply remains queryable by original ID. `actor_receipt_and_abort_are_served_while_another_modal_is_open` verifies the relay never waits on UI polling.
- [ ] **Step 2 — Deferred RED:** `cargo test -p rusty-win32 browser_integration` and `cargo test -p rusty-win32 download_intent`; expect absent controller/intent interfaces. Record Not run until authorized.
- [ ] **Step 3 — Implement GUI integration:** acquire the instance guard before creating a new GUI; run pipe transport off-thread. Callbacks queue signals and never borrow AppState across nested dialog APIs. File Info and duplicate modal contexts check the lease on a 250-ms dialog timer and before accepting; expiration ends the dialog on its own UI thread. Full URLs stay private in the draft while visible browser-origin URL fields are sanitized. Send prepared actions to the off-thread receipt relay; it records and forwards acceptance only after actor success, even when another capture dialog is open. Route status/abort through that worker and shared leases, without waiting on modal UI. Hold no RefCell borrow during native calls. Preserve settings/category/duplicate behavior for normal Add URL. During close, reject new inbox work, revoke pending leases, stop IPC, drop command-client clones and join relay workers before EngineHandle::join, including exceptional message-loop exits. No joins occur inside callbacks.
- [ ] **Step 4 — Deferred GREEN:** repeat Step 2 plus existing download_form/settings/presenter tests. After a separately authorized launch, one context-menu request opens exactly one compact File Info dialog, then the existing status window; cancelling it submits nothing.
- [ ] **Step 5 — Commit:** `feat(win32): receive browser downloads through native dialogs`.

### Task 5: Build the Chrome capture policy and normal handoff

**Files:** Create `extensions/chrome/{package.json,capture-policy.js,handoff.js,native-client.js,tests/capture-policy.test.js,tests/handoff.test.js}`.

**Interfaces:**
- `extensions/chrome/package.json` is private, has `type:"module"` and no dependencies; the built-in Node tests import the same `.js` ES modules as Chrome.
- `classifyDownload(item, observations, preferences, nowMs) -> {eligible:boolean,reason:string,request?:CapturePayload}` is pure.
- `NativeClient.send(operation, {onPending, signal}) -> Promise<Reply>` validates version/ID/status and reuses a native port for pending handoffs; no eval or dynamic code.
- `handoff(downloadId, request, browserAdapter, nativeClient, ledger, now) -> Promise<HandoffOutcome>` uses injected Chrome APIs/time for tests. `HandoffOutcome` is `accepted`, `continued_in_chrome` or `needs_attention`.
- Ledger record `{id,downloadId,phase,pausedByUs,expiresAtMs,receipt?}`; phases `offered`, `paused`, `pending`, `accepted`, `browser_cancelled`, `fallback_pending`, `done`.

- [ ] **Step 1 — Write JS tests:** `keeps_post_blob_incognito_and_ambiguous_requests_in_chrome` includes simultaneous identical GET/POST URLs, redirects and separate tabs. Fixtures use real DownloadItem fields without invented tabId/requestId; `missing_header_observation_is_not_assumed_safe` and Cookie/Authorization flags reject capture. `pause_failure_never_contacts_native_host`; `host_pending_does_not_cancel_chrome`; `actor_accepted_is_recorded_before_browser_cancel`; `user_cancel_and_missing_host_resume_only_our_pause`; `completed_chrome_file_is_never_removed`; `cancel_failure_only_aborts_new_owned_job`. Assert no `removeFile`, history erase or dangerous-download acceptance API is called.
- [ ] **Step 2 — Deferred RED:** `node --test extensions/chrome/tests/capture-policy.test.js extensions/chrome/tests/handoff.test.js`; expect missing modules/exports before implementation. Current restriction: author only, Not run.
- [ ] **Step 3 — Implement policy and handshake:** correlate uniquely by exact original/final HTTP URL and a 10-second window, checking DownloadItem.referrer when available. Candidates across different tabs still count as ambiguity: DownloadItem exposes no tabId/webRequest requestId. Retain only GET/redirect/credential booleans from observed headers, never values. Bound observations to 64 entries/30-second TTL and session records to 32. Extract only the basename from Chrome's filename for filename_hint; never forward an absolute save path. Create the ID/session record before pause; honor native pending vs final statuses and 120-second lease. Before cancelling Chrome, query current native status even if an earlier capture reply was accepted, so a replayed receipt cannot cancel Chrome after owned-job abortion. Before cancel/resume, re-read the item and follow the spec state rules. Context-menu handoff has no Chrome download ID, so failure shows an error without manufacturing a second browser download.
- [ ] **Step 4 — Deferred GREEN:** repeat Step 2; assertions cover ordering and side effects, not mock call counts alone. Permitted now: `node --check` for syntax only.
- [ ] **Step 5 — Commit:** `feat(chrome): hand eligible downloads to Rusty with browser fallback`.

### Task 6: Wire the extension UI and reversible per-user host setup

**Files:** Create `extensions/chrome/{manifest.json,service-worker.js,popup.html,popup.js,popup.css,icons/*,README.md}`, `scripts/{register,unregister}-chrome-host.ps1`, `scripts/verify-chrome-source.ps1`. Modify `scripts/verify-source.ps1` to call only the static verifier.

**Interfaces:**
- `manifest_version:3`, `minimum_chrome_version:"120"`, `background:{service_worker:"service-worker.js",type:"module"}`, `incognito:"not_allowed"`; exact permissions/optional host permissions from the spec. Menu ID `rusty-download-link`, title `Download with Rusty`, context `link`.
- Popup has capture toggle (off initially), host/app status and setup-help link. Optional host access is requested only by toggle's user gesture; refusal keeps capture off.
- PowerShell script parameters and owned registry/config locations are exactly those in the spec; both support `SupportsShouldProcess`/`-WhatIf`.

- [ ] **Step 1 — Write static/setup tests:** verifier asserts manifest module/type/floor/permissions and referenced local assets; fail if cookies/debugger/content scripts or remote scripts appear. Add `scripts/tests/chrome-host-registration.Tests.ps1` with isolated provider/path fixtures or a recording registry adapter: invalid ID/missing binary writes nothing; quoted Unicode paths serialize correctly; uninstall refuses a different registration owner. Document that Pester execution is deferred and optional tooling, not an installed runtime dependency.
- [ ] **Step 2 — Deferred RED:** `powershell -NoProfile -Command "Invoke-Pester ./scripts/tests/chrome-host-registration.Tests.ps1"`; expected missing commands/tests before implementation. Do not install Pester or execute fixtures under the current restriction.
- [ ] **Step 3 — Implement wiring and scripts:** register listeners synchronously at service-worker module load. Use non-blocking webRequest observations with `requestHeaders`/`extraHeaders` only under granted host access. Scripts generate JSON with structured serialization, validate absolute files, use `-LiteralPath`, and update only the named HKCU key under ShouldProcess. Do not run the installer as part of implementation. Supply original local 16/32/48/128-pixel icons using existing Rusty artwork; no new image-generation dependency. Extension popup remains compact and accessible, with readable status text.
- [ ] **Step 4 — Verify statically now:** `powershell -NoProfile -File scripts/verify-chrome-source.ps1` parses files only; no registry reads/writes, Chrome startup, native host launch or tests. Later, after explicit setup approval, load unpacked, get its real ID, register the built binaries, verify ping and test uninstall/re-register.
- [ ] **Step 5 — Commit:** `feat(chrome): add capture controls and per-user native host setup` with static checks/docs.

### Task 7: Recover handoffs after disconnects and service-worker restarts

**Files:** Create `extensions/chrome/{recovery.js,tests/recovery.test.js}`. Modify `service-worker.js`, `handoff.js`, `native-client.js`, `popup.js`; extend `crates/engine/tests/submissions.rs` and GUI controller tests where receipt reconciliation needs coverage.

**Interfaces:**
- `recoverPending(browserAdapter, nativeClient, ledger, nowMs) -> Promise<RecoveryReport>`; report has arrays `resolved`, `continued`, `needsAttention` of capture IDs, never URLs.
- `SessionLedger::{put,get,list,remove}` are async functions backed by `chrome.storage.session`; only the capture preference uses `storage.local`.
- Recovery listeners: module startup reconciliation, runtime startup, native disconnect and `alarms.onAlarm`; create one named 30-second recovery alarm while capture/pending records need it, not repeated timers per request.

- [ ] **Step 1 — Write failure-order tests:** `worker_restart_uses_same_id_and_receipt`; `accepted_reply_lost_never_resubmits`; `unknown_unreachable_status_fails_open`; `chrome_cancel_failure_retains_actionable_record`; `user_pause_is_not_resumed`; `expired_prompt_cannot_submit_later`; `two_simultaneous_requests_remain_distinct`. Test expiry/reconnect during app shutdown, accepted reused entries, already completed Chrome items and receipt retention boundaries. Assert browser cancel is never triggered by a transport error or pending response.
- [ ] **Step 2 — Deferred RED:** `node --test extensions/chrome/tests/recovery.test.js`; expect missing reconciliation exports. Record Not run until authorized.
- [ ] **Step 3 — Implement reconciliation:** follow spec status/ownership table and retain unresolved cancel/resume errors visibly. Never assume globals survive worker termination; status/retry use the existing UUID. Revoke a still-pending lease before intentional fallback. If native state is unreachable, prefer Chrome and allow the documented extra-copy race instead of cancelling without evidence. Expire metadata/session URLs; don't put them into local/sync storage or console messages.
- [ ] **Step 4 — Deferred GREEN:** `node --test extensions/chrome/tests/capture-policy.test.js extensions/chrome/tests/handoff.test.js extensions/chrome/tests/recovery.test.js` plus Task 2/4 Rust tests after authorization. All normal/fault-order cases pass; report any platform/manual gaps explicitly.
- [ ] **Step 5 — Commit:** `fix(chrome): recover paused downloads after handoff interruptions`.

### Task 8: Document setup and perform the authorized acceptance pass

**Files:** Create `docs/chrome-integration.md`. Modify `README.md`, `docs/native-gui-validation.md`, `docs/implementation-progress.md` and stale browser-capability copy in `crates/win32/src/app.rs` only after the feature is wired.

**Interfaces:** Document actual build outputs, unpacked extension path, explicit host registration/unregistration, capture toggle, compact File Info/duplicate behavior and fallback status. Keep historical `docs/idm-browser-protocol.md` unchanged and clearly separate.

- [ ] **Step 1 — Write acceptance checklist:** direct public GET; context menu; signed URL query preserved; Cookie/Authorization/POST/blob untouched; duplicate Numbered/Overwrite/Existing/Cancel; File Info cancel/expiry; Download Later; two active plus waiting; app already running/closed/closing; missing host; wrong ID; Chrome completed/pause/cancel/resume race; worker restart; lost ack; Unicode/spaced path; uninstall ownership; no URL/credential log leakage.
- [ ] **Step 2 — Run permitted source checks:** `powershell -NoProfile -File scripts/verify-source.ps1`, `git diff --check`, JSON/PowerShell parsing and `node --check` extension modules. Expected: four exact workspace packages, valid assets/manifests/IDs, clean formatting and syntax. Record what actually ran.
- [ ] **Step 3 — Deferred compiled/test validation:** after the user explicitly requests it, run `cargo test --workspace`, `node --test extensions/chrome/tests/capture-policy.test.js extensions/chrome/tests/handoff.test.js extensions/chrome/tests/recovery.test.js` and setup-script tests once; fix actual failures before broad repeats. A manual pass separately needs built GUI/host binaries and explicit browser loading/registration authorization. Do not count proposed commands as results.
- [ ] **Step 4 — Document limits and evidence:** mark each manual case Pass/Fail/Not run; distinguish source implementation, tests run and real Chrome verification. Describe Chrome-independent request limitations and the absence of crash-persistent history/exactly-once delivery.
- [ ] **Step 5 — Commit:** `docs(browser): document Chrome integration setup and validation`. Check clean working tree, per-feature history and ahead/behind state; no push without a request.

## Dependency order and handoff

Implement 1 → 2 → 3 → 4 → 5 → 6 → 7 → 8. Each task establishes the interfaces
used by the next; do not wire automatic cancellation before receipt tests and GUI
integration exist. In the absence of runtime authorization, finish implementation
with source checks and an honest unexecuted acceptance matrix.

Self-review: every spec section maps to these tasks; the five Review Focus items
have named owning tests; protocol IDs, lease limits and status names match the
design; no browser/registry/build action is authorized by this document. This plan
and its design require user review before product implementation begins.

**Recommended execution:** Native. The eight tasks depend closely on the same
lease, receipt and intent interfaces, so one implementer can keep them aligned,
followed by one independent whole-change source review. Subagent-driven execution
is also available if the user prefers a review gate after each task.
