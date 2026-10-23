# Rusty Download Manager 0.3.0-beta.1

## Install

Run `RustyDownloadManager-0.3.0-beta.1-windows-x64-setup.exe`. Setup installs for the
current Windows user and offers **Start Rusty in the background when I sign in**
(selected by default), an optional desktop shortcut, and Chrome setup.
This release is unsigned; Windows may show an unknown publisher prompt.

## Connect a browser

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

Chrome, Brave, Edge and Firefox each need their own extension package and native
host registration. The installer offers a Chrome helper. For another browser,
open **Options > General > Add browser** and follow its extension setup steps.
Firefox distribution requires signing before normal installation.

After setup, supported HTTP/HTTPS GET downloads open Rusty's File Info prompt,
even while the main manager is hidden. A uniquely observed download with Cookie
or Authorization headers requires a separate, unchecked approval in File Info.
Declining it, missing headers or an expired observation leaves the download in
the browser. Headers stay in memory and require a new capture after restart.
Unsupported downloads and failed handoffs stay with the browser. Upload requests
and private downloads are outside automatic capture. See [address refresh](refresh-download-address.md).

If media panels are enabled in Options and website access is granted, a video
can show **Choose video format in Rusty**. Click it to inspect an observed public
HLS/DASH source or an individual public YouTube page, choose a quality/container
and submit it to Rusty's destination dialog. Direct file buttons still work.
The preview expires; inspect again if it does. Rusty does not support playlists,
live streams, DRM or account-required video. See [browser video and protected
capture](browser-media-support.md).

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

Unfinished jobs are held for review after restart. Select a held job to resume
it if its saved bytes and server response can be verified. A changed response,
missing range support or altered partial may require a fresh download address
or an explicit restart from zero. Completed and failed downloads now remain in
the list across restarts until explicitly deleted. The Last Try column is
preserved; Transfer Details shows Added, Last Try and Finished dates in local
time. Older unknown dates remain Unknown. Delete and Delete Completed remove
history records without deleting completed destination files. History holds up
to 10,000 entries within the shared 16 MiB encrypted storage limit; at capacity,
delete records before adding more. See [permanent history](permanent-history.md)
for migration, recovery and storage-failure behavior.

## Connections per file

Open **Options > Connection** to choose 1–16 connections per file (default 8).
The choice is copied into each newly added download; existing queued downloads
keep their earlier choice. A setting of 1 uses the regular single stream. Rusty
uses multiple connections only for a known-size HTTP(S) response that accepts
exact byte ranges and supplies a consistent strong ETag or Last-Modified value.
Other responses continue through one stream. The limit of two active files is
unchanged.

Open **Show details** in a transfer window to see each segment's byte range,
progress bar, connection status and speed. **Connections: N active (limit M)**
counts logical data-transfer requests, not physical TCP sockets or metadata
probes. Completed segments remain listed while other segments finish. Speeds
exclude saved bytes and local file assembly; paused or finished transfers show
zero active connections and zero speed. Unknown-length responses use an
indeterminate progress bar. Connection details are session-only: restored history
and media backends without worker telemetry show **Connections: Unavailable**.

Segmented downloads save later byte ranges in separate temporary files beside
the main partial. Keep all recovery files together if you move them for manual
inspection. Rusty verifies each saved range before resuming and checks adjacent
range overlaps when relying on Last-Modified. A changed response or damaged
range holds the download with its saved bytes intact until you refresh the
address or explicitly restart it.

Use the sidebar and Scheduler to create named queues, change their order and
set optional start/stop times. Rusty must be running and Windows awake for a
schedule to fire. A missed start is skipped.

For a paused, failed or restored job with saved bytes, right-click and choose
**Refresh download address**. Paste a fresh HTTP/HTTPS URL or wait for a new
browser download. Rusty checks representative saved bytes against exact server
ranges before continuing. Protected browser headers are kept only in memory
and must be captured again after Rusty restarts. See [address refresh](refresh-download-address.md).

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
See [release notes](RELEASE_NOTES.md) and [validation](VALIDATION.md) for the
current checks and remaining live acceptance cases.

## Toolbar

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
Tasks → Queues.
