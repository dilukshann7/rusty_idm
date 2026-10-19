# Chrome integration

Source implements an opt-in Manifest V3 extension, Windows native messaging
host, user/logon/session secured pipe, File Info integration, actor-confirmed
receipts and session recovery. Windows debug compilation and per-user host
registration succeeded. A correlated framed native-host ping passed. Runtime
test suites and end-to-end download acceptance have not run.

Setup preparation update, 2026-10-19: a Windows debug build of `rusty-win32`
and `rusty-native-host` succeeded using the locked offline dependencies. Copies
were staged under `%LOCALAPPDATA%\RustyIDM\bin`, and the unpacked extension was
staged under `%LOCALAPPDATA%\RustyIDM\chrome-extension`. Binary copies matched
their build hashes. Chrome was opened to its extensions page. The user loaded the unpacked extension
and provided its ID, `ipolekmihgpdbmelheajogmgnjgmenol`. The native host was then
registered for that exact origin. A direct framed ping to the installed binary
passed, reporting version 0.1.0 and GUI not running, with empty stderr. The
automatic capture preference was left off.
The build emitted one unused-function warning for `show_duplicate`. Runtime
tests and live handoff acceptance remain unrun.

## Setup when ready

These are deferred instructions for an explicitly authorized build/setup pass.
Requirements: Windows 10/11, Chrome 120+, Rust MSVC and Windows SDK resource compiler.

```powershell
cargo build --release -p rusty-win32 -p rusty-native-host
```

Outputs: `target/release/rusty-win32.exe` and
`target/release/rusty-native-host.exe`. Keep both at stable absolute paths.
In Chrome's extensions page enable developer mode, choose **Load unpacked**,
select `extensions/chrome` and copy its actual 32-character ID.

```powershell
# Replace placeholders. -WhatIf previews setup without changing registration.
./scripts/register-chrome-host.ps1 -ExtensionId '<32 a-p characters>' `
  -HostExe 'C:\Your Rusty Folder\rusty-native-host.exe' `
  -GuiExe 'C:\Your Rusty Folder\rusty-win32.exe' -WhatIf
# Repeat without -WhatIf only when you intend to register this installation.
./scripts/unregister-chrome-host.ps1 -WhatIf
```

Registration writes three owned JSON files under
`%LOCALAPPDATA%\RustyIDM\browser` and the default value of
`HKCU\Software\Google\Chrome\NativeMessagingHosts\dev.rusty_idm.chrome`.
The host validates the exact extension origin, installed host path and absolute
GUI path. Launch arguments contain only `--browser-integration`, never URLs.
The popup ping does not start Rusty. A capture can start the GUI; an additional
GUI process forwards activation to the first instance.

Unregister removes only matching owned files/key and refuses unrelated or
modified registrations. Both scripts support `-WhatIf` and `-Confirm`.
Pester is optional tooling for authored setup tests, not a product dependency.
The registration script ran during installation. Unregister and Pester tests
were not executed; Pester was not installed.

## Use and handoff

Capture defaults to off. The popup toggle asks for optional HTTP/HTTPS site
access; denial keeps automatic capture off. **Download with Rusty** in a link's
context menu makes a fresh public GET and opens File Info directly. A sanitized
filename hint cannot supply a save path. Existing duplicate numbered-copy,
overwrite, existing-entry and cancel choices remain available, including a
remembered preference. Download Later accepts a visible held Rusty job.
The engine still owns two-active-download admission, pause/resume and guarded
replacement.

Automatic capture requires one uniquely observed safe GET chain within ten
seconds, header observation for every redirect hop, no observed credentials,
and an active, unpaused, non-incognito download. POST, Cookie/Authorization,
ambiguous same-URL requests, extension-created downloads and unsupported schemes
stay in Chrome. Referrer correlation stores only a session digest. Missing
observations after a worker wakeup fail open.

An offer is stored before Chrome pause. One UUID follows a 120-second capture
lease through File Info, duplicate handling and actor admission. Dialog timers
and actor checks reject stale confirmation. Only an actor-confirmed receipt,
followed by current accepted status, permits Chrome cancellation. Completed
Chrome files and history are never removed. Fallback can cancel only a newly
created Rusty job owned by this capture; reused jobs are never cancelled.
A user's pause is not resumed.

Disconnects and worker restarts query the same UUID. One 30-second alarm handles
pending recovery, with request timers enforcing deadlines while the worker lives.
Popup status and a badge report unresolved problems. Use **Check pending
downloads** and inspect Chrome Downloads when pause/cancel/resume could not be
confirmed. An interrupted offer whose pause ownership was never recorded may
need manual resumption in Chrome.

Browser capture now restores Rusty when minimized and activates File Info and
duplicate dialogs on the GUI thread. Before delivery, the native host requests
foreground permission for the verified pipe server's process only, using
[AllowSetForegroundWindow](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-allowsetforegroundwindow).
System foreground-lock settings and permanent topmost flags are not changed.
The updated GUI/host Windows debug build passed and both installed binaries were
refreshed with matching build hashes. Actual Chrome focus behavior needs a user
retry; no GUI automation or runtime test suite was run for this change.

## Limits and privacy

This is a session handoff. Rusty's receipts/history do not survive a process
crash. Failure around acceptance can leave an extra copy; Chrome is preferred
when ownership is uncertain. Acceptance means the engine owns a job, not that a
server finished sending it. Servers requiring browser cookies, referrer or
user-agent can reject Rusty's independent request.

Frames: 65,536 payload bytes, checked before allocation. URLs: 8,192 UTF-8 bytes.
Filename hints: 255 UTF-16 units. Pending captures: 32. Completed receipts: 256
for ten minutes; new work is rejected while that capacity is occupied.
Only the capture preference uses permanent extension storage. Observations use
session storage with a 30-second TTL. Recovery records use IDs/download metadata
without retaining capture URL payloads. URL queries reach the engine but are
redacted from UI/debug/log output. Credential header values are never stored.

No cookie import, POST replay, DOM injection, profile edits, network listener,
media extraction, store publication or other Chromium-browser support is supplied.
The v0.1.0 installer bundles the extension and a visible Chrome setup helper;
Chrome approval is still required. Historical investigation material is archived
separately from the production source.

## Verification

Permitted source checks: `scripts/verify-source.ps1` (including the Chrome static
verifier), locked offline Cargo metadata, rustfmt parsing/formatting, JSON and
PowerShell parsing, `node --check`, and `git diff --check`. These do not prove
Rust compilation or runtime behavior.

The final source verifier passed for 58 Rust files and all four exact workspace
packages. The Chrome verifier passed manifest/permission/import/PNG checks,
JavaScript syntax and PowerShell parsing. A partial independent GPT-6.1 Sol
review identified type errors and handoff failure cases; source corrections and
regression tests were authored. That reviewer hit a usage limit before finishing
the whole change. The author completed the remaining source review. These are
source inspection results; none of the regression tests were executed.

Deferred commands, requiring explicit execution authorization:

```powershell
cargo test --workspace
node --test extensions/chrome/tests/capture-policy.test.js extensions/chrome/tests/handoff.test.js extensions/chrome/tests/recovery.test.js
Invoke-Pester ./scripts/tests/chrome-host-registration.Tests.ps1
```

| Acceptance case | Runtime result |
| --- | --- |
| Installed host configuration, registry origin and correlated stdio ping | Pass |
| Direct public GET and link context menu | Not run |
| Signed query preserved and UI/log redaction | Not run |
| Cookie/Authorization/POST/blob/incognito untouched | Not run |
| Redirects, absent metadata, concurrent identical URLs | Not run |
| Duplicate numbered/overwrite/existing/cancel/remember | Not run |
| File Info cancel/expiry and Download Later | Not run |
| Two active downloads plus waiting/held jobs | Not run |
| GUI running/absent/starting/closing and second instance | Not run |
| Missing host, wrong origin, invalid/oversize frames | Not run |
| Wrong user/logon/remote pipe and occupied endpoint | Not run |
| Chrome complete/user pause/pause/cancel/resume races | Not run |
| Worker restart, lost acknowledgement, receipt retention | Not run |
| Unicode/spaced executable paths and shell-free launch | Not run |
| Setup/uninstall/re-register and unrelated-owner refusal | Not run |

Primary references: [Chrome Downloads](https://developer.chrome.com/docs/extensions/reference/api/downloads),
[native messaging](https://developer.chrome.com/docs/extensions/develop/concepts/native-messaging),
[webRequest](https://developer.chrome.com/docs/extensions/reference/api/webRequest),
[storage](https://developer.chrome.com/docs/extensions/reference/api/storage),
[service-worker lifecycle](https://developer.chrome.com/docs/extensions/develop/concepts/service-workers/lifecycle).
