# IDM feature parity gap analysis

Checked 2026-10-21 against the local `IDMan.exe` (6.42.61.3, SHA-256
`c7c80b420579d774f445b6b3f9282c0850760ef790373c0ece0f856a0a05d1d5`),
IDM's published help, and the current Rusty source at `695d216`. This is a
functional parity map, not a claim that IDM's internal algorithms have been
recovered. Rusty is an independent product with its own branding.

## Status key

- **Present in source:** an implementation path exists; live behavior may still
  need acceptance testing.
- **Partial:** Rusty covers only part of the IDM workflow.
- **Missing:** no corresponding implementation was found in the inspected source.

## Feature gaps

| Priority | IDM capability | Rusty status | Work needed for comparable behavior |
| --- | --- | --- | --- |
| P0 | Dynamic multi-connection segmentation of one file, including segment reassignment and resume | **Missing.** `crates/engine/src/transfer.rs` streams one request per file; the app describes itself as single-stream. Two active *files* are supported, which is different. | Add bounded parallel HTTP ranges per file, segment allocation/rebalancing, durable segment checkpoints, validator-aware resume, safe assembly and fallback to one stream when ranges are unsupported. Per-site connection limits and failure recovery belong in this design. |
| P0 | Automatic handoff of downloads from authenticated browser sessions | **Partial.** Ordinary capture deliberately excludes observed Cookie/Authorization requests; explicitly armed address refresh can use short-lived matching headers. | Design an opt-in, scoped credential handoff with origin and redirect checks, in-memory lifetime, clear browser fallback and end-to-end tests. Do not turn the current public-download path into silent credential replay. |
| P0 | Video download panel for web players, with available formats/qualities | **Partial.** Rusty's panel handles direct HTTP(S) media files; it does not discover or assemble streaming media. | Add browser-side media discovery and a format chooser, then support the corresponding unprotected stream manifests/segments and final media assembly. Establish exact supported formats with observed cases. |
| P0 | Durable download list/history, including completed unqueued downloads | **Partial.** Queued and unfinished recovery records survive restart; completed ordinary downloads are not permanent history. | Persist completed and failed records, timestamps, metadata and category/queue associations; add migration, pruning/import policy and startup loading. |
| P1 | Per-segment progress chart and connection detail in the download dialog | **Missing.** Rusty's current detail tabs show transfer status, speed limiter and completion options; there are no file segments to plot yet. | After segmented transfer exists, expose each segment's range, state and rate in the progress dialog with an aggregate estimate. |
| P1 | Site Grabber: crawl pages, filter links/files, save projects and mirror pages for offline browsing | **Source implemented; runtime unverified.** Native saved projects, bounded HTML/CSS discovery, filters, selected transfers and offline snapshots. See [Site Grabber](site-grabber.md). | Complete deferred build/live acceptance. Scheduled rescans and browser-rendered/authenticated sites remain outside this version. |
| P1 | FTP transfers | **Missing.** `validate_request` accepts HTTP/HTTPS only. | Add FTP transport, authentication, resume/size handling and integration with the existing job/file lifecycle. |
| P1 | Advanced connection options: per-server connection limits, download quotas, proxy/SOCKS/PAC controls and reusable site logins | **Partial.** Rusty has per-download speed limits, basic credentials for a request, and system-proxy discovery; it has no comparable settings pages or durable site-login rules. | Extend the engine policy and Options UI; secure credential storage and proxy-auth handling need dedicated design. |
| P1 | Scheduler synchronization queues, periodic rechecks, bounded automatic retries and actions after a queue finishes | **Partial.** Named queues, order, once/weekly/startup schedules and optional stops exist; failures move on without automatic retry. | Add sync queue semantics and change detection, repeat intervals, configurable retry/backoff, and chosen completion actions. |
| P2 | Numbered/wildcard batch generation, link drag-and-drop and the floating drop target | **Partial.** Rusty accepts a list of up to 128 batch URLs and has browser context actions; these IDM entry methods are not present in source. | Add batch pattern expansion with preview/limits, OLE drag-and-drop, and an optional native drop target. |
| P2 | Command-line download and queue controls | **Missing.** The desktop currently recognizes `--background`; no URL/queue CLI is implemented. | Add a documented CLI and forward commands to the existing instance with validation and acknowledgement. |
| P2 | Customizable main list/toolbar, including header sorting and configurable columns | **Partial.** Rusty uses native TreeView/ListView/toolbar and has IDM-like default controls, but columns and toolbar buttons are fixed in source; jobs are ordered by ID except within a queue. | Add header sort, persisted column visibility/order/width and toolbar layout preferences. |
| P2 | Import/export of download lists and settings | **Missing.** Current persistence is an internal recovery/queue store, without a portable list format or migration UI. | Define a versioned export format with private URL/credential handling, import preview and duplicate resolution. |
| P2 | ZIP preview, per-event sounds, and automatic update workflow | **Missing or unverified.** No corresponding UI or service path was found; Rusty explicitly has no updater. | Implement these only after core transfer/browser parity; ZIP preview must be bounded and read-only. |

## Already covered in current source

The current source has native Windows controls and IDM-like toolbar/category
layout, manual HTTP(S) URL admission, File Info/progress/completion dialogs,
pause/resume, strict saved-byte recovery, duplicate choices, categories and
folders, speed limits, named queues and scheduling, clipboard monitoring,
browser file-type/site/address rules, context actions, guarded destination
replacement, and address refresh. See [README](../README.md),
[configuration](configuration.md), [queues](queues-and-system-icons.md), and
[validation](VALIDATION.md). These are source-level findings; the published
v0.1.0 installer predates much of the current v0.2.0 source.

## Recommended implementation order

1. Finish live acceptance of the existing v0.2.0 transfer, recovery, settings,
   browser and installer paths. A present source feature is not yet parity.
2. Add parallel segments per file and durable complete history. These affect
   everyday download behavior most directly.
3. Expand browser handoff for user-authorized protected downloads and add
   unprotected streaming-video support.
4. Add advanced connection/proxy/login options and scheduler retries/sync.
5. Add FTP, Site Grabber and the smaller input/UI conveniences.

## Evidence and limits

- REA opened the installed PE as x86 and recorded its identity. PE resource
  inspection returned Evidence ID
  `ev_36ca9a1242174b18659b1df9cdf2f1b7ab5b17af04117ac7e524365445e02e20`.
  The resource tree is static evidence of packaging, not feature execution.
- REA's Ghidra-backed string query timed out during import/analysis. No
  decompilation or runtime behavior is claimed from it.
- The IDM feature descriptions are from its published help:
  [dynamic segmentation](https://support.internetdownloadmanager.com/support/segmentation.html),
  [main dialog/categories](https://www.internetdownloadmanager.com/support/using_idm/using_idm.html),
  [starting downloads](https://www.internetdownloadmanager.com/support/using_idm/starting.html),
  [options](https://www.internetdownloadmanager.com/support/using_idm/options.html),
  [scheduler](https://help.internetdownloadmanager.com/support/idm-scheduler/idm_scheduler.html),
  [Site Grabber](https://www.internetdownloadmanager.com/support/idm-grabber/grabber_wizard.html),
  [video panel](https://www.internetdownloadmanager.com/register/new_faq/video5.html),
  and [IDM release history](https://www.internetdownloadmanager.com/news.html).
  Help pages may describe versions other than this exact installed build.
- This pass read source and documentation. It did not build or launch Rusty or
  IDM, use a browser, transfer a file, or test installer behavior. Exact UI
  geometry, hidden edge cases, and IDM's internal segmentation algorithm remain
  unknown pending controlled runtime and deeper binary investigation.
