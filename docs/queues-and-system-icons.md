# Download queues, categories and Windows file icons

Approved implementation scope (2026-10-19).

- All Downloads, Unfinished and Finished contain Compressed, Documents, Music, Programs and Video. Custom overrides appear under All Downloads. Automatic inference includes HTML documents and never rewrites a chosen save path.
- Queues contain permanent Main download queue and named manual queues. Queue start follows saved order; stop pauses immediately and waits for acknowledgement. Parallelism is one or two per queue, globally two. Direct start runs one entry. Failures do not retry or block following entries.
- Engine owns queue membership, ordering, acknowledgement, persistence and scheduling. Download Later admits durably to Main. Membership changes require paused transfers. Removing membership holds the entry and retains recovery data.
- Stable queue/entry IDs differ from runtime job IDs. Versioned encrypted state lives under LocalAppData and uses atomic replacement. Passwords/login are never persisted; restored authenticated entries require new credentials.
- Sync partials before five-second/pause/exit checkpoints. Validate file identity, durable length and hashes before resuming; use strict Range/If-Range and require confirmation to restart unsafe continuations. Journal replacement before file moves and preserve ambiguous recovery files.
- Windows Shell icons use association lookup outside the UI thread, stock sidebar icons, caching, association invalidation and DPI image-list reconstruction. Completed executable icons come from the actual file.

## Deferred validation

Builds, tests, fixture servers, application launches and publishing require a separate explicit request. Source review covers category overrides and filters, Shell icon ownership, queue ordering and concurrency, stop/completion races, browser acknowledgement, restart-held recovery, credentials, invalid ranges, partial tampering, interrupted finalization and storage failure. Existing settings remain compatible; ordinary unqueued transfers do not gain complete persisted history.

## Using the controls

- Choose a category beneath All Downloads, Unfinished or Finished. File Info starts at Automatic; a chosen category overrides extension inference. The list context menu also offers Set category.
- Download Later saves a held entry in Main download queue. Start Download remains immediate.
- Select a queue in the sidebar, then use Queues > Start or Stop. The status bar reports held/running/stopping, active/completed/total counts and parallelism. Completed members remain listed; order and membership appear in Description.
- Queues > Create/Rename/Delete manages names; Main is permanent and only empty custom queues can be deleted. Select a row to Move Up/Down. Right-click Add to queue or Delete from queue changes membership after the entry is paused.
- Direct Start/Resume starts the selected entry. Failed entries require an explicit action; running queues continue past them.
- On restored entries, right-click Supply credentials when required. Confirm required restart explicitly discards verified saved bytes. A modified partial is preserved separately and its path remains in the recovery record/Properties.

## Storage and ownership

`%LOCALAPPDATA%\RustyIDM\queues.v1.dpapi` holds versioned definitions, ordered stable entry IDs, private full URLs, completion markers, checkpoints and finalization journals. The GUI receives redacted URLs. Credentials are kept only in memory. Ordinary downloads are not persisted unless assigned to a queue; removing membership retains a held recovery entry.

A checkpoint syncs bytes before storing file identity, durable length, prefix and full-byte SHA-256 hashes, expected size, final response URL and a strong ETag. Startup holds unfinished entries; continuation validates the original file handle and all recorded bytes before discarding any trailing uncheckpointed bytes. Existing destination approvals are not restored. Finalization records intended original-backup and partial moves before those operations. Corrupt or unreadable storage is preserved and blocks mutations rather than being overwritten.

## Deferred acceptance matrix

| Area | Cases to exercise after explicit authorization |
| --- | --- |
| Categories | Case-insensitive extensions, HTML, unknown files, status subfilters, custom overrides, preserve edited path |
| Shell icons | Change HTML default app, embedded EXE icons, missing file fallback, association invalidation, DPI rebuild, row replacement |
| Scheduling | Saved order, one/two per queue and two globally, direct Resume, immediate Stop, pause acknowledgement and completion race |
| Membership | One queue per entry, paused requirement, reordering, unassign held with checkpoint, permanent Main, delete-empty guard |
| Failures | Continue to next entry, no automatic retry, storage failure holds queue, recovered failed entries held |
| Browser | Persist before acknowledgement, storage rejection, expired lease rollback, duplicate actions use same scheduler |
| Recovery | Restart-held, credentials absent, valid range, changed validators, ignored ranges, truncated/tampered/missing partial |
| Finalization | Before/after original move, before/after partial commit, original restoration, destination race and ambiguous files |
| Storage | DPAPI user boundary, corrupt/unreadable state, atomic replacement, write/sync failures, pause and exit checkpoints |

Source formatting/resource/manifest/script checks do not replace compilation or this runtime matrix.

## Queue editor and timed scheduling (unreleased source)

Right-click the Queues heading or a queue and choose **Create new queue**. After the name is saved by the engine, Rusty opens its Scheduler with the new queue selected. **Edit queue** opens Files in the queue; **Schedule** opens the scheduling tab. The Queues main menu provides the same editor access.

The Scheduler has a queue tree, an ordered file list, one/two simultaneous downloads, Up/Down and Remove from queue, and Start now/Stop/Apply/Close. Main is permanent and custom queues can be deleted only when empty. File reordering and membership actions apply immediately. Apply/Discard/Cancel protects unsaved schedule/parallelism edits when changing queues, creating another queue, starting, closing the editor or exiting Rusty. Progress updates preserve your draft; failed saves leave it editable.

Schedules can start a queue when Rusty starts, once on a date, or weekly on selected weekdays (all seven means daily). An optional stop time pauses the queue. These options default off. Times are Windows local time, shown with 24-hour minute precision. Rusty must be running and Windows awake: a missed start is skipped, while an already-armed stop overdue after sleep is applied on wake. Overnight stop windows are supported. Stop deadlines use absolute instants: a repeated daylight-saving hour selects the next actual occurrence, and a nonexistent stop minute is skipped to its next valid day. Starting an already-running queue does not postpone its stop.

Timed starts process eligible entries through the existing queue order and global limits. Completed/failed entries are not automatically redownloaded/retried. Missing credentials and recovery/restart requirements still block their entries. Occurrences are recorded before admission; clock rollback and repeated daylight-saving minutes cannot replay the same occurrence. Stop now keeps future start settings; disable the start setting to prevent future timed runs. A due stop takes precedence over an automatic start in the same local minute.

Schema-1 queue definitions migrate in memory to schema 2 with scheduling disabled. The next accepted mutation saves schema 2 to the same encrypted `queues.v1.dpapi` container; IDs, checkpoint and journal records are preserved. Unsupported/corrupt storage remains preserved and blocked.

The approved [design](superpowers/specs/2026-10-20-queue-editor-and-scheduling-design.md) and [implementation plan](superpowers/plans/2026-10-20-queue-editor-and-scheduling.md) record scope and deferred acceptance. Runtime checks still needed include creation/save failures and acknowledgements, dirty edits, sidebar keyboard targeting, once/weekly/startup coalescing, overnight/overdue stops, DST/time-zone/clock changes, schema migration, pause/completion races and DPI/window teardown. No builds, tests or application launches were performed for this source change.
