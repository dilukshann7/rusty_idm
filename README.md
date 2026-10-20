# Rusty Download Manager

**v0.1.0 · Windows 10/11 x64 · Native Win32 · Rust**

![Rusty icon](assets/branding/rusty-128.png)

A compact Windows download manager that stays in the notification area and can
catch supported public downloads from Chrome.

## Install

Download the [v0.1.0 Windows installer](https://github.com/dilukshann7/rusty_idm/releases/download/v0.1.0/RustyDownloadManager-0.1.0-windows-x64-setup.exe).
Setup installs for your Windows user, offers background startup at sign-in, and
provides a **Connect Chrome** helper. Chrome requires one-time extension and
website-access approval. Existing integration preferences are preserved on upgrade.

Close or minimize the manager to keep it running in the background. Use its
tray icon to reopen it, add a URL, change options or Exit. Rusty operates while
Windows is awake and you are signed in. The portable ZIP also runs without an
installer; it does not configure startup or Chrome automatically.

See the [user guide](docs/USER_GUIDE.md), [release notes](docs/RELEASE_NOTES.md)
and the release's `SHA256SUMS.txt`. This release is unsigned.

## Features

- HTTP/HTTPS downloads with two simultaneous transfers.
- Pause/resume, per-download speed limits, progress/speed/ETA and cancellation.
- Compact native dialogs, a [download context menu](docs/download-context-menu.md),
  and a [completion dialog](docs/completion-dialog.md) with file-opening actions.
- Duplicate-link choices, numbered copies and guarded file replacement.
- Chrome capture through an extension, native host and secured local bridge.
- Tray/background operation, hidden login startup and single-instance launches.

The published v0.1.0 release has session-only history and does not include named
queues or restart recovery. Move/Rename, segmentation, address refresh and automatic
updates are not included. Chrome capture covers
supported public GET downloads; private/authenticated/streaming downloads remain
with the browser. Preferences use LocalAppData, including portable execution.

## Unreleased source: queues and Windows icons

The `codex/queues-and-system-icons` source adds named manual queues, category
hierarchies and Windows-associated file icons. **Download Later** saves to Main
download queue; the **Queues** menu manages selected queues and their order.
Creating a queue opens a compact Scheduler with Schedule and Files in the queue
tabs. Its sidebar menu provides Start/Stop/Edit/Schedule/Delete/Create controls.
Optional startup, one-time or weekday starts and stop times run through the engine.
Automatic options default off; missed starts are skipped.
Queue state and checkpoint metadata use Windows current-user encryption under
LocalAppData. After restart, unfinished entries remain held. Credentials must be
entered again; unsafe continuation requires an explicit restart confirmation.
Ordinary unqueued downloads keep their session lifecycle.

See the [queue and recovery guide](docs/queues-and-system-icons.md). These source
changes have not been compiled or exercised in the desktop app. They are not part
of the published v0.1.0 installer.

## Source and release build

The workspace contains `engine`, `win32`, `browser-bridge` and `native-host` crates.
Dependencies are pinned in Cargo.lock. Original branding is reproducible with
`scripts/generate-branding.ps1`; toolbar artwork has a separate generator.

With Rust MSVC, the Windows SDK resource compiler, Python and Inno Setup available,
`scripts/build-release.ps1 -Compiler <path-to-ISCC.exe>` builds the static-CRT
optimized binaries, stages required files/licenses and produces setup/portable/
extension artifacts with SHA-256 checksums. It does not launch the app, run the
installer or run test suites. Generated release files live under ignored `dist/`.

See [development](docs/DEVELOPMENT.md), [Chrome integration](docs/chrome-integration.md)
and [validation](docs/VALIDATION.md) for source structure and verification boundaries.
MIT license; see [third-party notices](THIRD_PARTY.md).
