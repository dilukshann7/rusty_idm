# Rusty Download Manager

![Rusty Download Manager icon](assets/branding/rusty-128.png)

**Pre-alpha · Windows 10/11 x64 · Rust + native Win32**

Rusty Download Manager is an early Windows download manager built with Rust and native Windows controls. It can download HTTP/HTTPS files, manage transfers from the notification area, and hand off eligible public downloads from a browser extension.

The current source targets v0.2.1. Check [Releases](https://github.com/dilukshann7/rusty_idm/releases) for packaged builds. The v0.1.0 installer is an older snapshot without the features listed below; installer upgrades and live browser handoff still need acceptance testing.

## What is in the project

| Area | Current source |
| --- | --- |
| Downloads | HTTP/HTTPS transfers, up to two active jobs, 1–16 connections per eligible file, progress, pause/resume, cancellation, per-download speed limits and saved partials for unfinished jobs. |
| Windows app | Native File Info, progress and completion dialogs; download list and categories; notification-area controls and optional sign-in startup. |
| Files | Duplicate-download choices, numbered copies, guarded replacement and partial-file recovery paths. |
| Organization | Manual queues, queue ordering and optional schedules. Unfinished jobs use encrypted current-user storage and are held for review after a restart. |
| Browser capture | A Chrome extension, native-messaging host and local bridge for eligible downloads. The source also includes browser-specific profiles for Brave, Edge and Firefox. |
| Address refresh | A paused, failed or restored download with saved bytes can use a pasted URL or a new browser capture. Rusty checks the local checkpoint, total size and three bounded byte samples before continuing the existing job. |

These are source-level capabilities, not a claim that every path has passed live acceptance testing. Ordinary automatic browser capture remains limited to eligible HTTP/HTTPS GET downloads without observed credentials. When the user explicitly arms address refresh, the extension can pass short-lived headers from the matching browser GET. Private browsing, uploads and streaming media remain outside automatic handoff.

## Refresh an expired link

Pause the download or select a failed or restored held job with saved bytes. Right-click it and choose **Refresh download address**. Paste a fresh HTTP/HTTPS link, or choose **Wait for browser download** and start the same download again in the browser. Rusty asks which browser capture to use before it takes over. If the original browser capture included a source page, the dialog can open it.

Rusty checks the owned partial and requests exact byte ranges from the replacement URL. It compares the response's total size and up to 64 KiB from the beginning, middle and end of the saved bytes. A failed check leaves the saved partial in place; a declined or failed browser handoff stays in the browser. This sampling is a practical identity check, not a full comparison of unseen bytes. Protected browser headers stay in memory and must be captured again after an app restart. See [address refresh](docs/refresh-download-address.md) for behavior and limits.

## Current limits

- Unfinished jobs are held after restart, but this is recovery state rather than a permanent download history. Completed and explicitly cancelled ordinary jobs are cleaned from that state.
- Continuing saved bytes requires exact HTTP ranges and a known total. A changed file, missing range support or expired authorization can stop the download while retaining its partial.
- Address refresh is limited to HTTP/HTTPS GET. Rust and JavaScript tests cover its key checks; a live Windows/browser handoff still needs acceptance testing.
- The browser profiles beyond Chrome and the newer queue/configuration UI still need live desktop/browser acceptance checks. Firefox distribution also needs its separate signing step.
- There is no automatic updater or code-signing certificate. Rusty needs an awake Windows session to run.

## Try the published build

The [v0.1.0 Windows installer](https://github.com/dilukshann7/rusty_idm/releases/download/v0.1.0/RustyDownloadManager-0.1.0-windows-x64-setup.exe) is an **older experimental snapshot**, not a build of all the features listed above. The release also provides a portable archive and checksums. It is unsigned.

The v0.1.0 build supports basic transfers, tray operation and optional Chrome capture. It does **not** include named queues, restart recovery, the newer Options pages or the additional browser profiles. Read the historical [v0.1.0 user guide](docs/v0.1.0-user-guide.md) and [release notes](docs/v0.1.0-release-notes.md) for that build. The current [v0.2.1 guide](docs/USER_GUIDE.md) and [release notes](docs/RELEASE_NOTES.md) describe the new source.

## Work with the source

The workspace has four Rust crates:

| Path | Purpose |
| --- | --- |
| `crates/engine` | Transfers, queue state, scheduling and file safety. |
| `crates/win32` | Native Windows interface and app lifecycle. |
| `crates/browser-bridge` | Browser handoff protocol and local transport. |
| `crates/native-host` | Native-messaging host used by browser extensions. |

Windows 10/11 x64, the Rust MSVC toolchain and the Windows SDK are needed for the desktop app. Python 3, Node.js and Inno Setup are used by the source checks or release packaging. Dependencies are pinned in `Cargo.lock`.

See [development](docs/DEVELOPMENT.md) for the repository layout and build instructions, [Chrome integration](docs/chrome-integration.md) for extension setup, and [validation](docs/VALIDATION.md) for what has and has not been exercised. The current [configuration](docs/configuration.md) and [queue](docs/queues-and-system-icons.md) guides describe newer source work.

## License

MIT. See [third-party notices](THIRD_PARTY.md).
