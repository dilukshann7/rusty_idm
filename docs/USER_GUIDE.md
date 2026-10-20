# Rusty Download Manager 0.1.0

## Install

Run `RustyDownloadManager-0.1.0-windows-x64-setup.exe`. Setup installs for the
current Windows user and offers **Start Rusty in the background when I sign in**
(selected by default), an optional desktop shortcut, and Chrome setup.
This release is unsigned; Windows may show an unknown publisher prompt.

## Connect Chrome once

Open **Connect Chrome** from the Start menu or the end of setup.

1. Open `chrome://extensions` in Chrome and turn on Developer mode.
2. Click Load unpacked and select the `chrome-extension` folder shown by the helper.
3. Copy the Rusty extension's 32-letter ID into the helper and click Connect.
4. Open Rusty's Chrome popup. Enable **Capture public downloads** and approve
   Chrome's requested website access.

An existing Rusty-owned registration is retained during upgrades. Your extension
capture setting is preserved. Reload the existing extension in Chrome after an
upgrade if Chrome has not yet picked up the changed files. Setup does not edit
Chrome profiles or install extensions without approval.

After that, supported public HTTP/HTTPS GET downloads open Rusty's File Info
prompt, even while the main manager is hidden. Unsupported downloads and failed
handoffs stay with Chrome. Signed-in/cookie-protected pages, streaming/blob URLs,
DRM, upload requests and private downloads are outside this release's capture scope.

## Background use

Rusty's icon lives in the Windows notification area (possibly under the hidden
icons arrow). Click/double-click it to open the manager; right-click for Open,
Add URL, Options or Exit. Closing or minimizing the manager hides it to the tray
while transfers and capture keep running. **Tasks > Exit** or tray **Exit** stops
the app and asks before cancelling active downloads.

Rusty starts hidden at the next Windows sign-in when startup is enabled. Manage
that entry in Windows Settings > Apps > Startup, or rerun setup to change its
startup task. Background mode does not keep the PC awake or operate while it is
off, sleeping or logged out. Chrome's native host can start Rusty for a new
capture when needed. Explicit Exit closes Rusty until you launch it again or a
later browser handoff starts it.

Successful transfers close their progress window and show the compact Download
complete dialog. Use Open, Open with, Open folder or Close. The opt-out checkbox
is saved; re-enable notifications in Options > Downloads. Failure/cancellation
status stays available. Right-click a list row for file/transfer actions and
Properties; Properties can be opened again after completion.

The download list and matching history last for the current process session.
Pause/resume is available within that session; there is no restart-resume or
persistent job history. Some servers require resuming from zero. Named queues,
Move/Rename and address refresh are not implemented; their menu entries are disabled.

## Upgrade or uninstall

Let active downloads finish, then use tray Exit. Close Chrome as well if setup
reports that the browser host is still in use. Setup will not stop downloads or
force-close the app. Uninstall from Windows Settings > Apps. It removes installed
files/shortcuts and matching startup/Chrome registration, preserving downloads
and saved preferences. Chrome's extension can be removed separately in Chrome.

## Portable archive

Extract the full portable ZIP and run `bin/rusty-win32.exe`. To launch hidden,
pass `--background`. Portable mode does not register startup or Chrome by itself.
The helper can explicitly connect Chrome to the extracted copy; keep that folder
in place afterward. Preferences use the same LocalAppData store as installed
Rusty, so this is portable execution, not isolated portable data storage.

## Verification

Use SHA256SUMS.txt to check downloaded artifacts with PowerShell Get-FileHash.
See RELEASE_NOTES.md for the actual release checks and live behavior limitations.

## Unreleased toolbar source

The native command row follows the reference order: Add URL, Resume, Stop,
Stop All, Delete, Delete Completed, Options, Scheduler, Start Queue, Stop Queue,
and Tell a Friend. Outline icons are 32 pixels at 100% scale, with matching
Windows-disabled artwork and labels underneath. Queue buttons have separate
arrow menus; their main click acts on the selected sidebar queue, or Main.

Delete Completed removes completed list entries through the engine without
removing downloaded files; active transfers and entries with recovery files
are excluded. Scheduler opens the existing queue editor. Tell a Friend opens
a compact link dialog with an explicit Copy link action; it sends no message.
The menu row is Tasks, File, Downloads, View, Help. Queue management is under
Tasks → Queues. This is source-only work, not a rebuilt published installer.
