# Rusty Download Manager v0.2.0 (release candidate)

## Changes since v0.1.0

- Named download queues, categories, Windows file icons, queue ordering, and optional startup, one-time or weekly schedules.
- Encrypted checkpoints for unfinished downloads. On restart, Rusty holds those jobs for review before continuing.
- Pause and resume with stricter HTTP range checks, per-download speed limits, and a smoother speed display.
- **Refresh download address** for a paused, failed or restored job with saved bytes. A pasted link or an explicitly selected browser capture is checked against the saved partial before continuation.
- Compact native Options pages for browser capture, file types, folders and download behavior.
- Browser extension profiles for Chrome, Brave, Edge and Firefox. Each browser requires its own extension and native-host setup; Firefox distribution requires signing.
- Updated native toolbar, scheduler, File Info and transfer dialogs.

## Scope and limits

Windows 10/11 x64. Rusty must run in a signed-in, awake Windows session. Ordinary automatic browser capture covers eligible public HTTP/HTTPS GET downloads. Protected captures are used only when the user explicitly arms address refresh. Streaming, private browsing and uploads are outside automatic handoff. Saved-byte verification samples up to three ranges, so it does not prove equality of unseen bytes. Completed downloads are not permanent history. This release is unsigned and has no automatic updater.

## Verification status

The v0.2.0 source, extension tests, Rust tests and a debug desktop smoke check have been exercised; see [validation](VALIDATION.md) for exact results and remaining acceptance cases. Optimized release binaries, installer upgrade/uninstall and live browser handoff have not yet been verified. This is preparation for v0.2.0, not a published release.
