# Validation

The v0.1.0 release uses Windows x64 optimized binaries and an unsigned current-user
Inno Setup installer. Source verification, native resource/version inspection,
static-CRT dependency inspection, archive integrity and SHA-256 checks establish
build/package results. They do not establish live desktop/browser behavior.

## Manual acceptance still required

| Area | Cases |
| --- | --- |
| Setup | Fresh install, existing registration, running-process guard, uninstall preserves user data |
| Background | Hidden sign-in startup, second instance, Close/minimize to tray, explicit Exit |
| Windows lifecycle | Explorer restart, sign-out during modal prompts, active-transfer shutdown |
| Capture | Supported public GET, unsupported/authenticated downloads remain in Chrome, lost acknowledgements |
| File Info | Add/held/cancel, duplicate choices, missing folders, Unicode/long paths |
| Transfers | Two simultaneous jobs, pause/resume, cancellation, limit changes, retry and guarded replacement |
| Completion | Progress closes on success, compact dialog actions, saved opt-out and explicit Properties |
| Context menu | Right-click row targeting, keyboard menu, state-dependent actions and double-click preference |
| Layout | 100/125/150/200 percent DPI, monitor changes, compact controls and keyboard access |

No test suite or live acceptance is claimed by packaging the release. The exact
artifact hashes and sizes are in the published SHA256SUMS.txt and release manifest.
The v0.1.0 build record is available in
[the v0.1.0 validation record](v0.1.0-release-validation.md); freshly published
artifacts use the hashes attached to their GitHub release.
