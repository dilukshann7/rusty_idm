# Rusty Download Manager

**v0.1.0 · Windows 10/11 x64 · Native Win32 · Rust**

![Rusty icon](assets/branding/rusty-128.png)

A compact Windows download manager that stays in the notification area and can
catch supported public downloads from Chrome.

## Install

Use `RustyDownloadManager-0.1.0-windows-x64-setup.exe` from the local `dist` folder.
Setup installs for your Windows user, offers background startup at sign-in, and
provides a **Connect Chrome** helper. Chrome requires one-time extension and
website-access approval. Existing integration preferences are preserved on upgrade.

Close or minimize the manager to keep it running in the background. Use its
tray icon to reopen it, add a URL, change options or Exit. Rusty operates while
Windows is awake and you are signed in. The portable ZIP also runs without an
installer; it does not configure startup or Chrome automatically.

See the [user guide](docs/USER_GUIDE.md), [release notes](docs/RELEASE_NOTES.md)
and `dist/SHA256SUMS.txt`. This is an unsigned local release.

## Features

- HTTP/HTTPS downloads with two simultaneous transfers.
- Pause/resume, per-download speed limits, progress/speed/ETA and cancellation.
- Compact native dialogs, a [download context menu](docs/download-context-menu.md),
  and a [completion dialog](docs/completion-dialog.md) with file-opening actions.
- Duplicate-link choices, numbered copies and guarded file replacement.
- Chrome capture through an extension, native host and secured local bridge.
- Tray/background operation, hidden login startup and single-instance launches.

History is session-only. Restart-resume, segmentation, named queues, Move/Rename,
address refresh and automatic updates are not included. Chrome capture covers
supported public GET downloads; private/authenticated/streaming downloads remain
with the browser. Preferences use LocalAppData, including portable execution.

## Source and release build

The workspace contains `engine`, `win32`, `browser-bridge` and `native-host` crates.
Dependencies are pinned in Cargo.lock. Original branding is reproducible with
`scripts/generate-branding.ps1`; toolbar artwork has a separate generator.

With Rust MSVC, the Windows SDK resource compiler, Python and Inno Setup available,
`scripts/build-release.ps1 -Compiler <path-to-ISCC.exe>` builds the static-CRT
optimized binaries, stages required files/licenses and produces setup/portable/
extension artifacts with SHA-256 checksums. It does not launch the app, run the
installer or run test suites. Generated release files live under ignored `dist/`.

See [implementation history](docs/implementation-progress.md),
[Chrome integration details](docs/chrome-integration.md) and
[native validation](docs/native-gui-validation.md) for evidence and pending live
acceptance scenarios. MIT license; see [third-party notices](THIRD_PARTY.md).
