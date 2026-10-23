# Rusty Download Manager v0.3.0-beta.1

Windows x64 beta. Installer, portable and browser-extension archives are provided
with SHA-256 checksums. Executables are unsigned.

## Changes since v0.2.1

- Site Grabber: saved public-site projects, bounded HTML/CSS discovery, filters,
  selectable results, shared downloads and offline snapshots. Saved state uses
  schema 8; older supported state migrates when saved.
- Permanent completed/failed history, with dates, metadata and categories retained
  until deletion. Capacity is 10,000 records within the shared 16 MiB encrypted
  state limit. Previously discarded records cannot be reconstructed.
- Automatic recovery for eligible temporary HTTP/network failures, including
  bounded retry settings, queue behavior and preserved checkpoints.
- Live transfer details: active connection counts, per-segment status/progress
  and improved telemetry.
- Segmented checkpoint verification no longer changes sidecar modification times.
  Uncheckpointed tails are trimmed only at the verified resume write boundary.
- Browser media controls, direct-file handling and compatible MP4 format choices,
  with pinned bundled yt-dlp, Deno and FFmpeg tools.
- Release cleanup: package versions derive from Cargo; Windows displays the beta
  version; Chromium extensions use numeric versions plus a beta label; unused
  code is removed and the README describes current installation and features.
- Updated recovery/concurrency regression tests and 10,000-entry admission
  coverage.
- Corrected a garbled Site Grabber resource label found during beta smoke checks.

## Verification and remaining acceptance

The pre-version Windows run passed 215 Rust tests, 82 extension tests and four
package checks. It produced static-CRT binaries and verified fresh installation,
upgrade from published v0.2.1, running-process protection, hidden startup command,
uninstall/reinstall data retention, and changed/matching URL refresh with
segmented partials. The final file matched the fixture SHA-256 and owned sidecars
were removed. These observations apply to the recorded pre-beta binaries;
the beta build's checks are recorded separately in [validation](VALIDATION.md).

Full Windows acceptance remains incomplete: attended sign-in/shutdown, Explorer
restart, forced process termination during active transfer/assembly, live browser
capture, screen-reader announcements, DPI scaling and monitor transitions need
verification. Very large histories may restore slowly. See
[Windows acceptance](windows-acceptance.md) for per-case results and limitations.

## Limits

Rusty needs an awake, signed-in Windows session. Capture handles eligible
HTTP/HTTPS GETs; protected requests require separate approval and fresh browser
credentials after restart. Public on-demand HLS/DASH and individual YouTube
videos are supported through the opt-in media panel. Private browsing, uploads,
playlists, live streams, DRM and account-required video are unsupported. Firefox
distribution requires signing. Saved-byte sampling cannot prove equality of
unseen bytes. No automatic updater is included.
