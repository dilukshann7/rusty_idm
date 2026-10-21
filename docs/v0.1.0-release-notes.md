# Rusty Download Manager v0.1.0

- Native Windows download manager with original Rusty branding and application icon.
- HTTP/HTTPS downloads, two simultaneous transfers, pause/resume, speed limits,
  guarded replacement, compact progress and completion dialogs.
- Native right-click menu with supported file actions, Redownload, transfer
  commands, Properties and a saved double-click preference.
- Notification-area controls, Close/minimize to tray, hidden sign-in startup,
  single-instance background launches and Explorer-restart icon recovery.
- Per-user Windows installer, portable archive, Chrome extension package,
  integration helper, license inventory and artifact SHA-256 checksums.

## Limits

Windows 10/11 x64 only. The PC must be awake and the user signed in. Chrome needs
one-time extension/website approval. Capture covers supported public GET
downloads; authenticated/private/streaming downloads remain with the browser.
History is session-only and there is no resume after process exit, named queues,
address refresh, Move/Rename, automatic updates or code-signing certificate.

## Release evidence

The release build and artifact verification results are recorded in
`release-manifest.json` beside the installer. No test suite or development server
is run as part of this release request. Installer installation/uninstallation,
Windows sign-in, Explorer restart, tray interactions and live Chrome capture
must be distinguished from source checks and compilation; unexercised scenarios
are not claimed as runtime-verified.
