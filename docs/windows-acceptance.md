# Windows acceptance record — 2026-10-23

Acceptance is **partial**. PASS below means the named case was observed on the
recorded binaries; it does not validate all related Windows behavior.

## Tested snapshot and environment

- Source baseline: `81f77a8738d2cb0ee6b38a8903a0336633ffa86d` on `main`, plus
  regression-test corrections described below. Production binaries were built
  before the v0.3.0-beta.1 metadata/dead-code cleanup.
- Windows 11 Home Single Language, x64, build 26300.
- Published v0.2.1 installer SHA-256:
  `931cca2c034c19e9d11f0c9207141320ca6f24673050f426757e80515cad5e96`.
  Verified against its published checksum before installation.
- Acceptance v0.2.1 installer SHA-256:
  `b4b1a7f7747c6cd4b0639642891d01083ecbd0b483ab9c92c98fa8850c12e4de`.
- Acceptance portable ZIP SHA-256:
  `7f916fc22acc79f19f4dfd3ae9fd4cb0622c7ab2d01a37f63736633ae39fc963`.
- The later beta packages are separately identified by the release's
  `release-manifest.json` and `SHA256SUMS.txt`; the live observations below
  must not be relabeled as beta-binary acceptance.

Local evidence is retained under ignored
`analysis/windows-acceptance/2026-10-23/`: command logs, result JSON, UI trees,
screenshots, fixture request records, original-state backups and preserved
pre-beta packages. These local files are not distributed with the release.

## Automated results

| Check | Result | Evidence |
| --- | --- | --- |
| Locked/offline Rust workspace | PASS — 215 passed; two ignored subprocess helper entrypoints, invoked by their parent containment test | `cargo-workspace-final.log`, `suite-summary.json` |
| Segmented integration | PASS — all 12 cases, no exclusions | Workspace log; includes the previously failing interrupted-checkpoint case |
| Extension suite | PASS — 82 tests | `extension-tests.log` |
| Browser-package checks | PASS — one test stages all four profiles | `browser-package-tests.log` |
| Media-package checks | PASS — three tests | `media-package-tests.log` |
| Static-CRT build, installer and archives | PASS for the pre-beta acceptance snapshot | `build-release.log`, `v0.2.1-packages/release-manifest.json` |

Test repairs corrected a stale shutdown API call, counted distinct running jobs
instead of repeated telemetry events, and replaced the old 1,000-entry rejection
expectation with retained-history/shutdown coverage. A unit case verifies actor
admission rejects the 10,001st record without pruning. No sidecar safety checks
were disabled. Two refresh fixtures failed in an early run, then passed on the
focused rerun and final full suite; both outputs were retained.

## Observed Windows cases

| Case | Status | Observation/evidence |
| --- | --- | --- |
| Fresh install | PASS | Published v0.2.1 installed with exit code 0 into the temporary acceptance environment; `older-install-result.json` |
| Running-app upgrade guard | PASS | Setup returned 1, app remained running and installed executable hash stayed unchanged; `running-process-guard.json` |
| Upgrade with segmented partial | PASS | Eight sidecars at 60.93% survived upgrade unchanged; settings hash unchanged and installed binary matched the new payload; `upgrade-result.json` |
| Restored job held after exit/upgrade | PASS | One held `upgrade.bin` row appeared without starting a transfer; `upgraded-restored-job.txt` |
| Changed-content address refresh | PASS | UI rejected mismatched bytes; every partial hash stayed unchanged and destination remained absent; `refresh-rejection-result.json` |
| Matching replacement URL | PASS | Same job resumed its segmented saved bytes against `/new.bin`; 64 MiB result matched SHA-256 `0a0f07dff884a453c6d2a69ea623144d3e86d7e849fc02282a6afd0dd67d796d`; no owned sidecars remained; `upgrade-refresh-success.json` |
| Uninstall | PASS | Exit 0; binary/startup entry removed; encrypted state, settings, synthetic partial and completed download preserved; `uninstall-result.json` |
| Reinstall | PASS | Exit 0; encrypted state unchanged and installed executable matched payload; `reinstall-result.json`. Post-reinstall UI reopening was not repeated. |
| Owned startup command | PASS | Exact quoted executable plus `--background` registered; executing it created a process without a visible manager; uninstall removed the matching entry; `startup-command-result.json` |
| Keyboard/UI accessibility spot checks | PASS for inspected controls only | Ctrl+N, File Info input, pause, keyboard context menu and refresh flow exercised; UI trees exposed labels/statuses. No full screen-reader claim. |
| Original-environment restoration | PASS | Original files matched backup, original browser registry values stayed unchanged, startup returned to its original absent state; `restoration-result.json` |

The original Rusty folder was preserved by same-volume rename before testing and
restored afterward. Synthetic residual data remains in the separate
`RustyIDM.acceptance-test-20261023` LocalAppData folder. The pre-existing debug
native host was stopped only after explicit user authorization. It was not
manually relaunched: the browser owns that host's lifecycle. No Windows display
settings were changed; no sign-out, reboot or Explorer restart was performed.

## Beta executable smoke checks

The v0.3.0-beta.1 release executable was launched with a separate acceptance-only
LocalAppData directory. Its title and product version matched the beta label;
a second launch activated the same single process. The native Site Grabber
dialog opened and exposed its controls. A garbled resource label reproduced in
the first beta build; replacing its non-ASCII dash fixed the displayed text in
the rebuilt executable. The pre-fix smoke process also completed explicit Exit.
The resource correction passed the 40 native UI unit tests. Evidence:
`beta-manager-smoke.txt`, `beta-single-instance.json`,
`beta-site-grabber-fixed.txt`, `beta-final-win32-tests.log`.

These are launch/dialog spot checks, not crawler, accessibility or DPI
acceptance. Original user download state was not loaded by the smoke process.

## Unfinished acceptance

| Case | Status | Reason |
| --- | --- | --- |
| Forced active-transfer restart, retry wait and final assembly | BLOCKED | No successful active-process kill was completed; the attempted check found the transfer already complete and refused to label it a crash test. |
| Two simultaneous large transfers; live sidecar corruption/replacement | BLOCKED | Automated range/output coverage passed; equivalent desktop fault injection was not completed. |
| Native-host guard and foreign integration ownership lifecycle | BLOCKED | App-running guard passed; separate live host/ownership cases were not completed. |
| Tray mouse/keyboard, second instance and Explorer restart | BLOCKED | Full interaction matrix was not completed. |
| Real sign-in, sign-out and Windows shutdown during prompts | BLOCKED | Requires an attended session; startup-command execution is not sign-in proof. |
| Browser capture and authenticated segmented refresh | BLOCKED | Pasted public URL path passed; live extension handoff was not exercised. |
| Screen-reader announcements and every dialog's keyboard order | BLOCKED | Accessible trees and selected controls were inspected; Narrator and the full dialog matrix were not tested. |
| 100/125/150/200% DPI and monitor transitions | BLOCKED | Scale matrix and physical monitor transitions were not exercised. |
| Very large history startup | FAIL — performance finding | A synthetic 10,000-entry restore consumed over 60 seconds before the acceptance-only fixture was stopped. Per-entry full-store snapshots make this path expensive; functional capacity coverage passes, but startup performance remains open. |

The user subsequently requested cleanup and a beta release. These unfinished
cases remain explicit beta limitations; publishing a prerelease does not close
the Windows acceptance gate.
