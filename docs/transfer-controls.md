# Pause, resume and per-download speed limits

Implemented in source on 2026-10-19 and extended with durable address refresh on
2026-10-21. Rust/Win32 and existing Tokio/reqwest dependencies are retained.
The refresh extension has not been compiled or exercised in the app.

## Behavior

- Pause transitions Running → Pausing → Paused after the worker drops the HTTP
  response and acknowledges. The unique partial stays open; no new bytes are
  written after acknowledgement. Paused jobs release an active connection slot.
- Resume transitions Paused → Waiting → Running through the same FIFO and maximum
  two active transfers. It wakes the existing worker, preserving file ownership,
  byte count and attempt identity. Closing/reopening the status dialog preserves
  the limit and paused state. The main Downloads menu can also resume a job.
- Byte continuation requires a known total and either a strong ETag or fresh
  exact-range sample verification. A resume sends `Range: bytes=<written>-`
  and `If-Range: <etag>` when a strong ETag exists, with identity encoding. Before
  appending, a 206 response must match the original final URL, validator, exact
  starting offset, ending byte, total length and response length when provided.
  These conditional requests follow [HTTP semantics](https://www.rfc-editor.org/rfc/rfc9110.html#name-if-range).
- Without a known total or a valid checkpoint, continuation needs a confirmed
  restart. A full response to a saved range request fails safely. Truncation and
  rewind operate on the owned partial handle before fresh bytes are written.
  Invalid ranges, changed partial-response validators and encoded bodies fail
  safely rather than splice incompatible bytes. Range support can only be
  confirmed when the server actually responds to the request.
- Cancel, Stop All and Remove wake paused or throttled workers and clean verified
  owned partials. Exit checkpoints unfinished jobs and restores them held later.
  The existing no-replace finalization remains in effect.
  A pause arriving during finalization may lose to completion; the completed
  file remains valid. Closing just the status window leaves the job active.
- The Speed Limiter tab has enable, KB/sec entry and Apply controls. Values are
  positive whole numbers, with 1 KB = 1024 bytes; unchecked + Apply is unlimited.
  Changes apply to the existing worker without restarting the connection.
  Limits are per download, retained on retry, and session-only.
- Small body slices are paced before writing, with no accumulated idle burst
  credit. Pause, cancel and live configuration changes interrupt pacing waits.
  HTTP/socket buffers can already contain in-flight bytes; this is application
  body backpressure rather than an OS network-interface quota. Only actual
  network waits count toward timeouts, not intentional throttle delays.
- A failed saved partial can use [Refresh download address](refresh-download-address.md).
  Completed and explicitly cancelled ordinary jobs do not become permanent history.

## Verification boundary

The user disallows build/app/server launches unless explicitly requested. Tests
were written before corresponding behavior where practical, but no RED/GREEN or
runtime result is claimed: Rust tests compile and fixtures launch servers.
The permitted checks are rustfmt parsing/formatting and `scripts/verify-source.ps1`
(offline no-dependency Cargo metadata, resource IDs, XML, assets and handoff links).
Rustfmt and the source-check script passed for all 34 Rust files, workspace
manifests/lock, XML/resource IDs, icon assets and handoff links. There are 66
written tests across the workspace; none were executed. The independent source
review found no concrete blocking issue in the state/HTTP/filesystem/Win32 paths
and checked native API declarations against the cached Windows 0.62.2 source.

Pending runtime cases: repeated pause/resume, paused cancellation/removal/shutdown,
three jobs with paused slot release and queued resume, safe Range/If-Range bytes,
ignored ranges/changed representations/unknown lengths, throttled byte budget,
live limit increases/decreases/removal and paused limit changes, completion races,
closing/reopening the status window, keyboard/focus and multiple-DPI rendering.
