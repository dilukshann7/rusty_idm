# Development

## Source layout

| Path | Responsibility |
| --- | --- |
| crates/engine | Download actor, HTTP transfer, pause/limits and guarded output |
| crates/win32 | Native controls/dialogs, tray, presenter and engine/capture bridge |
| crates/browser-bridge | Framing, leases, protocol and secured Windows transport |
| crates/native-host | Chrome native-messaging executable |
| extensions/chrome | Original Manifest V3 extension and its source tests |
| assets/branding | Original SVG, PNG and ICO assets |
| scripts | Branding, source verification, setup integration and release build |
| installer/windows.iss | Current-user setup, startup, shortcuts and removal |
| tools/package_release.py | Runtime payload, license inventory, ZIPs and checksums |

The active tree contains production source, original assets, current behavior
documentation and tests. Old investigation snapshots, extracted reference files,
experimental concepts and implementation plans were archived before removal;
their original commits remain available in Git history.

## Prerequisites and source checks

Windows 10/11 x64, Rust MSVC, Windows SDK resource compiler, Python 3, Node.js for
extension syntax checks, and Inno Setup for installer compilation.

`scripts/verify-source.ps1` inspects locked/offline Cargo metadata, Rust formatting,
native assets and IDs, all current documentation links, and extension/PowerShell
syntax. It does not build, run tests, launch applications or start a server.

## Build a release

```powershell
./scripts/build-release.ps1 -Compiler 'C:\Path\To\ISCC.exe'
```

The script prepares the pinned media tools, produces optimized static-CRT GUI/host executables, validates their
version resources, stages required runtime files and dependency licenses, and
compiles setup plus portable/extension ZIPs under ignored `dist/`. Preserve an
existing `dist/` before rebuilding. No app or installer is run automatically.

`python tools/fetch_media_tools.py` can prepare the tools independently without
building Rusty. It verifies downloaded archives and extracted executables against
`tools/media_tools_lock.json`; existing executable copies must match their pinned
hashes. Tools and download cache live under ignored `assets/media-tools*` paths.
`python tools/test_media_package.py` checks executable hashes, PE architecture,
runtime hash consistency and notices. The package generator refuses missing or
changed tools and includes them in both installer and portable payloads.

Toolbar and app artwork can be regenerated independently with
`scripts/generate-toolbar-icons.ps1` and `scripts/generate-branding.ps1`.

## Checks and history

Existing Rust, extension and PowerShell tests are retained. Run them only when
explicitly authorized in this workspace. See [validation](VALIDATION.md) for
the difference between source/build/package checks and pending live acceptance.
Never expose native-host stdout to logs: it is reserved for framed messages.
