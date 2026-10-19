# Rusty Download Manager

A Rust download engine and native Win32 desktop application. The first milestone
uses actual Windows menu, Toolbar, TreeView, ListView, status bar, dialogs and
Progress Bar controls, with system fonts and per-monitor DPI declarations.
The compact toolbar includes original colored outline icons above the action
labels, grey disabled variants, and embedded artwork for multiple display scales.
Body controls/dialogs use compact Tahoma 8 metrics to follow the supplied reference.

Implemented source includes HTTP/HTTPS streaming, two simultaneous transfers,
waiting jobs, progress/speed/ETA, pause/resume, live per-download speed limits,
cancellation, removal of list entries, retry
from the beginning, native destination pickers, transfer details, explicit file
and folder opening, and a saved default destination. History is session-only.

Each download reserves a unique partial beside the destination. Ordinary downloads
refuse to replace a target that appears meanwhile. Explicit duplicate overwrite
guards the approved original until the new partial is complete and synced, then
uses handle-based backup/commit/rollback. Failed finalization exposes recovery
paths for retained partials and original backups.

Add URL uses two native dialogs: enter the address and optional HTTP authorization,
then review Download File Info (category, suggested filename/Save As, Browse,
remembered category folder and description). Start Download submits immediately.
Download Later adds a held entry for this session; select it and use
**Downloads → Start / Resume Download** when ready. Cancelling either dialog submits nothing.
Only folder preferences are saved; URLs, authorization and job history are not.

A compact download-status dialog opens automatically for each transfer attempt,
with percentage/filename title, aligned status fields, native progress, tabs and
expandable details. Closing it leaves the download running; a retry opens it again.
Pause keeps the partial file and frees a connection slot; Resume rejoins the
two-transfer queue. Safe continuation uses a known length and strong ETag with
HTTP Range/If-Range. If safe continuation is unavailable or the server sends a
fresh complete response, resuming restarts from zero. Resume capability indicates
whether byte continuation can be attempted. Speed Limiter accepts a positive
whole-number KB/sec limit (1 KB = 1024 bytes); Apply updates the running transfer,
and unchecking then applying removes the limit. The completion tab can close the
dialog automatically after a successful transfer. See the
[reference sizing analysis](docs/compact-ui-reference.md) for the layout changes
and [transfer controls](docs/transfer-controls.md) for behavior and the source-only
verification boundary.

## Verification status

Source implementation only. Rust formatting/syntax and source/resource checks
are recorded in [the validation record](docs/native-gui-validation.md).
No build, Rust test, app or server command was launched by this implementation
session. VS Code's existing rust-analyzer independently ran a background check;
its saved diagnostics exposed missing imports/native flag operators, which were
corrected in source. Automatic editor checks/build-script loading are now disabled
in this workspace. **A clean compilation of the final source, test execution and
GUI validation remain pending.** No working executable is claimed.
Direct dependency versions are pinned and Cargo.lock is included. The source
milestone was completed before this local Git repository was initialized.

When explicitly authorized, Windows with a stable Rust MSVC toolchain and Windows
SDK resource compiler can use these commands from this directory:

```powershell
cargo check --workspace
cargo test --workspace
cargo run -p rusty-win32
```

These are deferred instructions, not commands run during implementation. No dev
server is needed. Test HTTP fixtures bind loopback and run only within tests.

## Source map

- `crates/engine`: validation, states, safe output, HTTP streaming, admission,
  cancellation, bounded event delivery, and tests.
- `crates/win32`: native controls/resources, presenter, dialogs, settings,
  details, event bridge, lifecycle, and tests.
- [Native design](docs/superpowers/specs/2026-10-18-rusty-win32-design.md)
  and [implementation plan](docs/superpowers/plans/2026-10-18-rusty-win32-implementation.md).
- [Execution record](docs/implementation-progress.md) and
  [manual acceptance matrix](docs/native-gui-validation.md).
- [IDM investigation](docs/idm-investigation.md): historical reference evidence,
  preserved separately from Rusty's implementation.

Durable history, resume after application exit, segmentation, named queues, browser capture, tray
operation, installers and automatic updates remain future milestones.
