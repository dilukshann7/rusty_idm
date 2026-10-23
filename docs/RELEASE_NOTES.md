# Rusty Download Manager v0.2.1

## Changes since v0.1.0

- Site Grabber in current source: native saved projects, bounded public HTML/CSS
  discovery, selectable results, shared transfers and offline snapshots. Queue
  storage migrates to schema 8. The engine compiled during focused transfer
  tests; native UI compilation and live feature acceptance remain deferred.
  See [Site Grabber](site-grabber.md).

- Permanent completed/failed history in current source, preserving dates, metadata and categories until deletion; schema-7 migration and a 10,000-entry/16 MiB capacity limit. Builds and runtime acceptance for this change are deferred.

- Named download queues, categories, Windows file icons, queue ordering, and optional startup, one-time or weekly schedules.
- Encrypted checkpoints for unfinished downloads. On restart, Rusty holds those jobs for review before continuing.
- Pause and resume with stricter HTTP range checks, per-download speed limits, and a smoother speed display.
- Up to 16 connections for one eligible HTTP(S) file, with dynamic range splitting, aggregate speed limiting, verified sidecar recovery and single-stream fallback for unsupported servers. The native **Options > Connection** page defaults to 8.
- **Refresh download address** for a paused, failed or restored job with saved bytes. A pasted link or an explicitly selected browser capture is checked against the saved partial before continuation.
- Compact native Options pages for browser capture, file types, folders and download behavior.
- Browser extension profiles for Chrome, Brave, Edge and Firefox. Each browser requires its own extension and native-host setup; Firefox distribution requires signing.
- Protected GET capture with a separate unchecked File Info approval and exact observed redirect headers.
- Opt-in public HLS, DASH and single YouTube video format inspection and media jobs, using pinned bundled tools in installer and portable distributions.
- Updated native toolbar, scheduler, File Info and transfer dialogs.

## Scope and limits

Windows 10/11 x64. Rusty must run in a signed-in, awake Windows session. Automatic browser capture covers eligible HTTP/HTTPS GET downloads; protected ones require a separate approval. Public on-demand HLS, DASH and individual YouTube videos are selected from the opt-in media panel. Playlists, live streams, DRM, account-required video, private browsing and uploads are outside this release. Saved-byte verification samples up to three ranges, so it does not prove equality of unseen bytes. Permanent history is a newer source change; previously discarded records cannot be recovered. This release is unsigned and has no automatic updater.

## Verification status

See [validation](VALIDATION.md) for the checks completed on this version and the remaining acceptance cases. Installer upgrade/uninstall, live browser handoff and authenticated address refresh still need hands-on verification.
