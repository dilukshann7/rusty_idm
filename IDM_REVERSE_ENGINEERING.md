# Rusty IDM — Initial Reverse-Engineering Brief

Date: 2026-10-18

Implementation update (2026-10-19): the native Win32 milestone now has source in
`crates/engine` and `crates/win32`. See [README](README.md) and the
[validation record](docs/native-gui-validation.md). Formatting and manifest/source
checks are separate from final compilation/runtime validation, which remain pending.
The investigation notes below describe the earlier pre-implementation state.

Follow-up completed: see [the investigation and Rust specification](docs/idm-investigation.md), [browser protocol](docs/idm-browser-protocol.md), and [runtime results](docs/idm-runtime-results.md). The initial-pass notes below are retained as a historical baseline; the follow-up includes native decompilation and controlled runtime tests. The independent Rust implementation is documented in [README](README.md).

## Goal

The bounded follow-up investigation is now closed. See [session lifetime](docs/idm-session-lifetime.md) and [scheduler/retry findings](docs/idm-scheduler-retry.md) for the final passes and remaining unknowns. The independent Rust specification informed the implementation under `crates/engine` and `crates/win32`.

Build an independent download manager in Rust, using the installed Internet Download Manager (IDM) as a reference for features and behavior. This document records the initial static inspection and proposed implementation direction, before Rust implementation began.

## Inspected target

| Item | Observed value |
| --- | --- |
| Executable | `C:\Program Files (x86)\Internet Download Manager\IDMan.exe` |
| Product metadata | Internet Download Manager (IDM), Tonec Inc. |
| File/product version metadata | `6, 42, 61, 3` |
| Architecture | x86, PE32 |
| File size | 6,043,648 bytes |
| SHA-256 | `c7c80b420579d774f445b6b3f9282c0850760ef790373c0ece0f856a0a05d1d5` |

## Initial findings

| Finding | Evidence | What it establishes |
| --- | --- | --- |
| Native Windows application with embedded UI resources | REA PE resource inventory: 436 resources, including 105 dialog resources and 37 string-table resources | Static resource structure; actual UI behavior remains untested. |
| Download controls and queues | Executable strings mention starting/resuming downloads, stopping downloads, and download queues | Feature labels exist; their underlying logic has not been traced. |
| Connection and progress controls | Executable strings mention maximum connections, per-server limits, and progress by connection | Suggests a connection-based download design; scheduling and segmentation algorithms remain unknown. |
| Scheduling and throttling controls | Executable strings include scheduler and speed-limiter settings | These controls are represented in the binary; runtime effects remain unverified. |
| Browser integration interface | Installed `IDMMsgHost.json` declares host `com.tonec.idm`, type `stdio`, and executable `IDMMsgHost.exe` | A native messaging interface is declared. Its message format and runtime behavior have not been inspected. |
| Browser capture and retry behavior are described locally | Installed `tips.txt` describes browser integration, scheduler retries, and command-line downloads | Shipped documentation describes these features; this pass did not test them. |

REA evidence ID: `ev_36ca9a1242174b18659b1df9cdf2f1b7ab5b17af04117ac7e524365445e02e20`.

REA examined 887 resource-directory entries. Its resource inventory is complete within that operation's scope. Except for icon-group relationships, resource payloads remain opaque in that result. Feature-label findings above came from a separate local UTF-16 string scan, rather than decompilation or runtime observation. Version information came from Windows file metadata; the browser-interface declaration and tips came from the installed files.

## Proposed Rust MVP

The following is a new design proposal, not a recovered description of IDM internals.

1. Accept an HTTP/HTTPS URL and destination path.
2. Download a file with progress, speed, and estimated remaining time.
3. Support cancellation, pause, and safe resume where the server permits it.
4. Persist queued downloads and partial-file metadata across restarts.
5. Add bounded parallel range downloads after single-stream transfers work reliably.
6. Add configurable concurrent downloads, retries, and bandwidth limits.

Start with a command-line interface. A desktop interface, browser extension, and scheduler can follow once the download engine is reliable.

### Suggested module boundaries

```text
src/
  main.rs         # CLI entry point
  download.rs     # HTTP requests, redirects, streaming, progress
  segments.rs     # Byte-range planning and segment completion
  queue.rs        # Job states, concurrency limits, cancellation
  storage.rs      # Job metadata, partial files, recovery
  settings.rs     # Paths, limits, retry settings
```

### Download flow

```text
URL -> inspect response -> choose single stream or byte ranges
    -> write partial data -> persist progress
    -> check completion -> rename completed file
```

For range transfers, require a valid `206 Partial Content` response and matching `Content-Range`. If a server responds with `200 OK`, use a single stream without appending a full response to partial data. Resume should verify resource identity using server validators such as ETag or Last-Modified; changed or unverifiable content should trigger a safe restart. Unknown-length responses should remain supported without assuming a fixed segment layout. Record completed bytes only after the corresponding file writes are durable.

## Next investigation steps

- Trace download-related strings to native functions and inspect relevant calls.
- Inspect the shipped browser extension and native host to establish message framing and fields.
- Observe controlled downloads against a test server: range support, ignored ranges, interruption, redirects, and changed content.
- Compare observed queue, resume, retry, and speed-limit behavior with the Rust implementation as it develops.

## Initial-pass limits (historical)

This was a basic static reconnaissance pass. IDM was not launched, attached to, or modified. No functions were decompiled, network traffic captured, or download behavior tested. IDM's exact segmentation strategy, persistence format, retry algorithm, and browser message protocol remain unknown. No build or development server was run.

## Follow-up status

The follow-up traced range construction, response headers, selected persistence writes and the native messaging bridge. Controlled downloads exercised browser handoff, ranges, unknown size, redirects, transfer failures, changed-content recovery and resume after an IDM restart. Retained completed files match their fixture hashes. The detailed report identifies remaining gaps, including exact segment allocation, checkpoint encoding, autonomous retry policy, measured throttling and clean two-ready-job scheduler behavior. The evidence index is [analysis/idm/evidence-index.json](analysis/idm/evidence-index.json).
