# Rusty Download Manager

![Rusty Download Manager](assets/branding/rusty-128.png)

**Native Windows download manager, written in Rust.**

Rusty downloads files over HTTP/HTTPS, preserves unfinished transfers and permanent
history, and runs from the Windows notification area. Browser extensions send
eligible downloads to the app. Site Grabber discovers public website assets and
builds offline snapshots.

**v0.3.0-beta.1 · Windows 10/11 x64 · Unsigned beta**

## Download

- [Windows installer](https://github.com/dilukshann7/rusty_idm/releases/download/v0.3.0-beta.1/RustyDownloadManager-0.3.0-beta.1-windows-x64-setup.exe)
- [Portable ZIP](https://github.com/dilukshann7/rusty_idm/releases/download/v0.3.0-beta.1/RustyDownloadManager-0.3.0-beta.1-windows-x64-portable.zip)
- [Release notes, browser extensions and SHA-256 checksums](https://github.com/dilukshann7/rusty_idm/releases/tag/v0.3.0-beta.1)

Run the installer for a per-user installation, or extract the entire portable ZIP
and open `bin/rusty-win32.exe`. Both distributions include the pinned media tools
and their licenses. The default installation needs no administrator access.

Close or minimize the manager to keep it in the notification area. Use
**Tasks > Exit Rusty** or tray **Exit** to stop it. Optional sign-in startup
launches it in the background. Windows must remain awake and signed in.

## Features

| Area | Included |
| --- | --- |
| Transfers | Up to two active downloads, 1–16 connections per eligible file, dynamic segment splitting, pause/resume and per-download speed limits. |
| Recovery | Encrypted current-user checkpoints, saved partials and automatic retries for temporary HTTP/network failures. Restored jobs stay held for review. |
| Address refresh | Continue an unfinished job with a fresh URL after checking saved bytes, total size and bounded byte samples. Failed verification preserves the partial. |
| History and queues | Completed/failed history until deletion, categories, named queues, ordering and optional schedules. |
| Browser integration | Packages for Chrome, Brave, Edge and Firefox; eligible capture, explicit link handoff and opt-in media panels. |
| Public video | Format selection for supported public HLS/DASH and individual YouTube videos, with bundled yt-dlp, Deno and FFmpeg. |
| Site Grabber | Bounded public HTML/CSS discovery, filters, saved projects, selection before downloading and offline snapshots. |
| Windows UI | Native Win32 controls, transfer/segment details, File Info and completion dialogs, tray controls and optional startup. |

## Connect a browser

Install the matching extension package from the release, then connect its native
host through **Options > General > Add browser**. Setup also offers a Chrome
connection helper. Each browser needs separate setup; portable copies do not
register a host or enable startup automatically.

Protected HTTP/HTTPS GETs require separate approval in File Info. Browser
credentials stay in memory for that download and need fresh capture after
restart. Private browsing, uploads, DRM, live streams, playlists and
account-required video are unsupported. Firefox extension distribution requires
signing; the release ZIP is intended for development testing.

See [browser setup](docs/chrome-integration.md) and
[supported media and protected capture](docs/browser-media-support.md).

## Beta status

This is an experimental prerelease, without code signing or an automatic updater.
Keep backups of files you cannot replace.

Windows acceptance exercised installation, upgrade, uninstall, preserved data,
hidden startup and segmented address refresh. Full sign-in/shutdown, forced
active-transfer restart, browser handoff, screen-reader and DPI acceptance are
incomplete. Very large histories may restore slowly. The
[acceptance record](docs/windows-acceptance.md) and [validation log](docs/VALIDATION.md)
identify exact versions, results and remaining checks.

Resuming requires a known total size and verified HTTP byte ranges. A changed
response, damaged partial or expired authorization can stop continuation.
Address refresh samples saved bytes; it cannot prove equality of unseen bytes.
History supports 10,000 records within a shared 16 MiB encrypted-state limit.
Existing records are preserved at capacity.

## Build and contribute

Four Rust crates form the workspace: `engine` for transfers and persisted state,
`win32` for the desktop UI, `browser-bridge` for local handoff, and `native-host`
for browser native messaging.

Use Windows x64, the Rust MSVC toolchain and Windows SDK. Release packaging also
uses Python 3.11+, Node.js and Inno Setup 6.

```powershell
cargo test --workspace --locked --offline
node --test extensions/chrome/tests/*.test.js
python -m unittest discover -s tools/tests
python tools/test_media_package.py
./scripts/verify-source.ps1
./scripts/build-release.ps1 -Compiler 'C:\Path\To\ISCC.exe'
```

Prepare pinned tools with `python tools/fetch_media_tools.py` before media package
checks. The build script also prepares them, then creates the installer, portable
ZIP, extension packages and checksums under `dist/`. Preserve an existing
`dist/` before rebuilding. The script does not launch the app or installer.

Follow local workspace instructions before running builds or live checks.
Report bugs with the release version, Windows version and reproducible steps;
remove download tokens, cookies and authorization headers from reports.

## Documentation

[User guide](docs/USER_GUIDE.md) · [Configuration](docs/configuration.md) ·
[Queues and scheduling](docs/queues-and-system-icons.md) ·
[Address refresh](docs/refresh-download-address.md) ·
[Permanent history](docs/permanent-history.md) · [Site Grabber](docs/site-grabber.md) ·
[Development](docs/DEVELOPMENT.md)

## License

MIT for Rusty. Bundled dependencies and media tools have their own licenses;
see [third-party notices](THIRD_PARTY.md).
