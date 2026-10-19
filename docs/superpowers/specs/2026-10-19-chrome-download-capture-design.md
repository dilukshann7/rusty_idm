# Chrome download capture design

Status: proposed for user review, 2026-10-19. This records implementation choices
for the user's request to plan a Chrome extension that catches downloads for
Rusty. It is a new Rusty protocol, independent of historical IDM research.

## Intent and scope

Capture normal Chrome downloads into the existing Windows app, offer a
**Download with Rusty** link context menu, and keep Chrome's copy available until
Rusty accepts the handoff. The existing compact File Info, duplicate choices,
transfer status, two-active-download limit and safe output handling remain the
desktop workflow. Capture is an explicit extension preference, off initially.

First-version choices made by the plan author:

- Windows 10/11 and Chrome 120+; Manifest V3, plain JavaScript modules, no bundler.
- Automatic capture covers active, safe HTTP/HTTPS GET downloads with uniquely
  correlated request metadata and no observed Cookie/Authorization headers.
  Ambiguous, POST, incognito, blob/data/file URLs and extension-created downloads
  stay in Chrome. Request-header observation is used only for eligibility; values
  are never retained or forwarded. Cookie/session import, media extraction and
  download response-body interception are separate future work.
- User-agent/referrer-dependent servers may still reject Rusty's independent
  request. Engine acceptance means responsibility for the job, not proof that a
  server will finish sending the file. Rusty reports subsequent failures normally.
- Context-menu capture is an explicit fresh GET; it does not replay POST bodies
  or export browser credentials. Unsupported addresses are rejected visibly.
- Each captured link opens File Info directly, with URL and a sanitized filename
  hint prefilled. It uses the existing duplicate preference/dialog. Cancel or a
  timed-out prompt returns automatic downloads to Chrome. Download Later counts
  as acceptance of a visible held Rusty job.
- Planning creates documentation only. No extension is loaded, native host
  registered, browser setting changed, executable launched or registry modified.

## Component boundaries

1. `extensions/chrome`: service worker, eligibility classifier, handoff ledger,
   compact popup with capture toggle and connection status, context menu.
2. `crates/browser-bridge`: versioned JSON types, framing, user/session identity,
   named-pipe transport and bounded pending-capture controller.
3. `crates/native-host`: console binary `rusty-native-host`, speaking Chrome's
   framed stdin/stdout protocol and relaying validated messages to Rusty's pipe.
4. `crates/win32`: single-instance GUI, capture inbox and existing native dialogs.
5. `crates/engine`: correlated actor acknowledgements and idempotent browser
   actions; it remains the only owner of admission and transfer state.

The native host locates a configured absolute GUI executable and starts it with
the fixed `--browser-integration` switch when capture needs it. URLs never appear
in command-line arguments. Ping checks connection state without starting the GUI.
A second GUI instance forwards activation to the first and exits.

Pipe: `\\.\pipe\RustyIDM.Browser.v1.<user-SID>.<Windows-session-id>`.
Use explicit current-logon-SID access, reject remote clients, verify client/server
token identity, and refuse an unexpected existing endpoint. Use a matching local
session mutex. Worker threads own transport; callbacks only signal the UI.
These checks isolate other users and remote clients; arbitrary code already
running as this Windows user is outside the authentication boundary.

## Protocol v1

Native host name: `dev.rusty_idm.chrome`. The registration script supplies one
exact `chrome-extension://<id>/` allowed origin. The native host validates its
caller argument against its generated host configuration as well.

Envelope: `{v:1,id:<UUID>,op:<operation>,payload:<object>}`. A reply echoes `v/id`,
has `status`, and never echoes the full URL. Operations:

| Operation | Payload | Reply |
| --- | --- | --- |
| `ping` | `{}` | `ok`, with `host_version`, `gui_running` |
| `activate` | `{}` | `ok` after queuing activation of the existing GUI |
| `capture` | `{url,filename_hint?,source,download_id?,expires_at_ms}` | `pending`, then `accepted` or `declined` |
| `status` | `{capture_id}` | `pending`, `accepted`, `declined` or `unknown` |
| `abort` | `{capture_id}` | `aborted`, `already_accepted` or `unknown` |

`source` is `automatic` or `context_menu`. Automatic requires a nonnegative
integer `download_id`. Accepted replies contain `job_id` and `created_new`; the
latter is false for an activated/restarted existing entry. Stable decline codes:
`invalid_request`, `unsupported`, `busy`, `user_cancelled`, `browser_fallback`, `expired`,
`app_closing`, `engine_unavailable`, `recovery_required`.

Native and pipe frames both use a four-byte little-endian byte length followed
by UTF-8 JSON on Windows. Application ceiling: **65,536 payload bytes** in either
direction, checked before allocation. Limits: URL **8,192 UTF-8 bytes**, filename
hint **255 UTF-16 units**, **32** pending captures, **256** completed receipt
records with **10-minute** retention. Reject unknown versions/operations/fields,
invalid UUIDs, NUL/CR/LF and URL userinfo. Preserve URL queries internally; redact
them, userinfo and fragments in UI/logs. Filenames are hints, never save paths.

Repeated IDs with identical validated payloads return their existing result.
Reusing an ID for a different payload is invalid. Status/abort routing and actor
receipt forwarding run off the GUI thread, including while a modal dialog is open.
Abort revokes a pending lease; for an already accepted newly created job it asks
the actor to cancel only that owned job. Reused/restarted existing entries return
`already_accepted` and are never cancelled by fallback. Final aborted status is
`declined` with code `browser_fallback`; an initial capture reply stays immutable
for replay, while status reports its current ownership outcome. Duplicate-link detection is
separate from transport idempotency: two user clicks have different IDs and go
through the existing duplicate workflow.

## Handoff and recovery

1. On `downloads.onCreated`, classify before touching the download. Metadata must
   prove GET, header observation and no credentials; uncertainty stays in Chrome.
   Match exactly one observed request chain by original/final URL and a
   **10-second** window, checking DownloadItem.referrer when available. Chrome
   DownloadItem has no tab ID or webRequest request ID; do not assume either.
   Tab IDs can distinguish observed chains but cannot prove attachment to a
   DownloadItem. Multiple candidates across any tabs stay in Chrome. Any
   non-GET or credential-bearing redirect hop disqualifies the chain.
2. Store an `offered` record in `chrome.storage.session` before requesting pause.
   Mark `paused_by_us` only after `downloads.pause` resolves successfully. If pause
   fails or the download already completed, do not ask Rusty to take it over.
3. Keep a native port for this handoff. Native startup/connect timeout: **10
   seconds**. Entire capture lease: **120 seconds**, including dialogs and queues.
   The desktop clamps the incoming wall-clock deadline and tracks it with a local
   monotonic deadline. Expired dialogs close and stale confirmations cannot submit.
4. The desktop validates the draft and resolves duplicates into an intent, without
   activating/restarting an existing job yet. Its engine command
   carries an ID, shared lease and result sender. The actor validates the lease,
   applies the action once, and replies only after inserting/activating the job.
   A successful `try_send` alone is not acceptance. Rejected/full/closing queues
   produce a decline; they must not create an unreported job.
5. Record `accepted` in session storage before cancelling Chrome. Confirm current
   `status` is still accepted, including after any replay/reconnect. Re-read the
   Chrome item; if it already completed, keep its file and cancel only a newly
   created Rusty job attributable to this handoff. Never cancel an unrelated or
   reused existing job. Do not erase history or call `downloads.removeFile`.
6. Missing host, decline, user cancellation or expiry: abort the pending lease and
   resume only a still-active download paused by this extension. Never resume a
   user's pre-existing pause. If acknowledgement is lost, query the same ID before
   retrying or resuming. Do not mint a second ID for a reconnect.
7. After a worker restart, reconcile session records using Chrome state and
   desktop `status`. A **30-second** alarm provides recovery opportunities; normal
   per-request timers enforce the lease while the worker is alive. Unknown or
   unreachable status falls back to Chrome and revokes the desktop lease when
   possible. An app-closing signal declines pending requests before teardown.

This is a fail-open session handoff, not a distributed exactly-once transaction.
A desktop/connection failure around acceptance can cause an extra fallback copy;
do not trade away Chrome's copy to avoid that. If Chrome cancel/resume fails, keep
the item in the recovery ledger and show actionable status, rather than claiming
success. Process restarts can lose Rusty's current session/history; no URL-bearing
durable journal is introduced. Browser suspension can delay recovery timers.

## Extension permissions and setup

Required: `downloads`, `nativeMessaging`, `contextMenus`, `storage`, `alarms`,
`webRequest`. Optional host permissions: `http://*/*`, `https://*/*`, requested only
by the capture toggle's user gesture. Without them the context menu remains
available and automatic capture remains off. Incognito is `not_allowed`.
No content scripts, `cookies`, `debugger`, `downloads.open`, blocking webRequest,
all-page DOM injection, or download-UI suppression are needed.

Only the capture toggle persists in `storage.local`. In-flight URLs and request
metadata use `storage.session` and expire; never use sync storage for them.
No URL/credential values in console output, Rust Debug implementations or logs.

`register-chrome-host.ps1 -ExtensionId <32 a-p chars> -HostExe <absolute .exe>
-GuiExe <absolute .exe>` generates host manifest/config under
`%LOCALAPPDATA%\RustyIDM\browser` and registers only
`HKCU\Software\Google\Chrome\NativeMessagingHosts\dev.rusty_idm.chrome`.
Require existing binaries and an explicit extension ID; no guessed IDs, admin
rights, auto-loading Chrome, profile edits or network listener. Unregister removes
only the owned matching key/files and leaves unrelated registrations untouched.
Setup and unsetup provide `-WhatIf` using PowerShell ShouldProcess.

## Verification and scope boundaries

Current user instruction: no builds, Rust tests, application, fixture, browser or
dev-server launches unless explicitly requested. Implementation may write tests
and run source-only checks; label Rust RED/GREEN and manual results unexecuted.
Test and setup commands in the implementation plan are deferred instructions,
not permission to run them. Preserve feature-scoped commits when implemented.

Release acceptance after authorized execution: direct GET capture, context menu,
File Info/duplicate cancel, all duplicate choices, held job, missing host/full
queue, stale reply/worker restart, Chrome pause/cancel/resume failure, app exit,
large/malformed frames, wrong origin/user and Unicode/spaced installation paths.
Do not claim support for authenticated/POST/blob downloads, all Chromium browsers,
Chrome Web Store publication, installers or crash-persistent transfers.

## Primary references

- [Chrome Downloads API](https://developer.chrome.com/docs/extensions/reference/api/downloads): detection and pause/cancel/resume.
- [Native messaging](https://developer.chrome.com/docs/extensions/develop/concepts/native-messaging): framing, origin allowlist and Windows host registration.
- [Service-worker lifecycle](https://developer.chrome.com/docs/extensions/develop/concepts/service-workers/lifecycle): ephemeral state and recovery.
- [Storage API](https://developer.chrome.com/docs/extensions/reference/api/storage): local preferences and session state.
- [webRequest](https://developer.chrome.com/docs/extensions/reference/api/webRequest): observational eligibility, not blocking takeover.
- [Permissions](https://developer.chrome.com/docs/extensions/develop/concepts/declare-permissions) and [context menus](https://developer.chrome.com/docs/extensions/reference/api/contextMenus).
