# Queue editor and timed scheduling

Date: 2026-10-20
Status: Approved by the user on 2026-10-20; implemented as unreleased source.

## Intent

Add the queue workflow shown in the supplied IDM screenshots using Rusty's existing compact native Windows controls. The user explicitly selected **queue editor plus timed scheduling**. Creating a named queue should open its editor automatically, and the sidebar should offer the matching context menu.

## User flow and native layout

- Right-click the Queues heading: Create new queue is enabled; entry-specific commands are disabled.
- Right-click a queue: Start now, Stop, Edit queue, Schedule, Delete, and Create new queue. Resolve the target under the pointer and pin its QueueId while the menu is open. Keyboard invocation uses the selected node.
- Create new queue opens the existing name prompt. On successful engine creation/persistence acknowledgement, open the Scheduler with the new queue selected and the Schedule tab active. Cancel, duplicate names, or storage errors must not open an editor for another queue.
- Edit queue opens Files in the queue; Schedule opens Schedule. The Queues main menu also exposes the editor so keyboard users can reach it.
- One reusable, modeless native Scheduler window: queue tree on the left, selected queue name and two tabs on the right, and New queue/Delete plus Start now/Stop/Apply/Close along the bottom. Preserve the compact Tahoma/native-control appearance. Use native stock queue icons and Windows-associated file icons through the existing asynchronous icon infrastructure.
- Files in the queue shows ordered File Name, Size, Status and Time Left columns; one/two-download parallelism; Move Up, Move Down and Remove from queue controls. Retain the existing global maximum of two active downloads.
- Display acknowledged queue status and counts. Start/Stop/reorder/unassign use engine commands. Pending Apply edits must survive incoming progress events. Selection changes and closing with unsaved settings offer Apply, Discard or Cancel. File operations remain immediate and are not reverted by discarding schedule edits.
- New queue works from both the sidebar menu and Scheduler. Main download queue remains permanent; delete is enabled only for empty custom queues and is checked again by the engine.

## Schedule controls

- Start queue when Rusty starts (unchecked by default).
- Enable scheduled start (unchecked by default).
- Start time, with minute precision and local Windows date/time display.
- Once on a chosen date, or Weekly on selected days (seven weekday checkboxes; selecting all means daily).
- Optional Stop at time, also unchecked by default.
- A short explanation that timed actions work only while Rusty is running and Windows is awake, plus next-start/status feedback.
- Apply saves schedule and parallelism together through one acknowledged engine transaction. Start now does not implicitly save dirty settings; the user must resolve Apply/Discard/Cancel first. Stop acts immediately, even with unsaved edits, using the last applied configuration.

## Scheduling rules

These defaults are proposed implementation calls, not behavior the user previously specified:

- Existing queues and new queues have every automatic-start option disabled. Their current restart-held behavior remains the default. A saved start-on-startup or timed-start option is the explicit opt-in to automatic queue Start.
- The engine evaluates wall-clock time; the GUI never runs a second scheduler. Times follow the current Windows time zone. Reject invalid dates/times, an empty weekly day selection, a past one-time start, and equal configured timed start/stop times.
- Fire at most once during a matching local minute. Persist an occurrence high-water mark before admitting transfers so clock rollback, repeated polling, fall-back daylight-saving time and app restart cannot replay the same scheduled occurrence. Applying a changed schedule starts a new schedule revision.
- A scheduled minute missed while Rusty was closed, the computer was asleep, or the clock jumped forward is skipped; do not unexpectedly catch up later. Opening/waking within the matching minute may run it. Nonexistent spring-forward times are skipped. A one-time occurrence is consumed after its acknowledged firing.
- Start-on-startup runs once per engine session for queues with that saved option. Credential-required, restart-required and recovery-required entries remain blocked; eligible members can proceed. Start-on-startup and a timed start in the same minute must coalesce into one start, without extending an existing stop deadline.
- Scheduled Start uses the same scheduler/order/limits as Start now. Completed and failed entries stay skipped until explicitly redownloaded or retried. It does not synchronize or periodically redownload completed files.
- A configured stop time arms for the next local occurrence strictly after the queue starts, including overnight windows. It applies to Start now, startup starts and timed starts. Starting an already-running queue does not postpone that stop. A stop that becomes due during sleep is applied as soon as the engine resumes, rather than waiting for the next day.
- Stop holds pending entries and pauses active transfers, waiting for the existing pause acknowledgement. Completion can still win that race. Stop now does not erase future start settings; disable scheduled start to prevent future timed runs.
- Deleting a queue also removes its schedule and occurrence records. Editing or disabling a stop setting while running recomputes/removes its stop deadline from the applied configuration; it must not create an immediate start.
- Persistence failure must hold the affected queue and report an error; never announce a successful save or perform an automatic start whose occurrence could not be recorded.

## Engine, persistence and ownership

- Add serializable schedule configuration and last-fired/revision information to persisted queue definitions, and schedule/status fields to queue snapshots.
- Read existing schema-1 queue storage with scheduling disabled, migrate in memory to schema 2, and atomically write schema 2 on the next accepted mutation. Retain the existing encrypted container path for compatibility. Unsupported/corrupt storage stays preserved and blocked as before. No credentials enter schedule storage.
- Stable queue/entry IDs, recovery checkpoints, journal handling, strict resume validation and manual membership rules remain authoritative. Scheduling operates only on saved queued entries.
- Native callbacks enqueue owned actions only. The outer message loop handles editor state, file dialogs/name prompts and commands. Keep HWND access on the GUI thread; never join workers or borrow AppState in a native callback.
- Reuse current queue commands where possible; provide a correlated acknowledgement for queue creation and atomic editor Apply. The builder can choose the acknowledgement representation and internal calendar helper structure.
- A calendar/clock boundary must make occurrence comparison, overnight stops and skipped-time decisions inspectable without a GUI. Use existing Windows time APIs and pinned dependencies where practical; avoid a separate service or OS scheduled task.

## Boundaries proposed for this change

The screenshots are visual references. This change includes the queue editor, its context menu, and the start/startup/weekday/stop scheduling described above. It does not add periodic content synchronization, automatic failure retries, modem hang-up, completion program launching, application exit, computer shutdown, download-limit profiles or a wake-from-sleep service. Do not show inert copies of those controls.

## Validation boundary

No builds, test execution, fixture servers, app launches or publishing without a separate explicit request. Do not add tests without authorization. Perform source/resource/manifest checks and source review. Document deferred runtime cases: creation acknowledgement and validation errors; context-menu targeting; dirty edits; queue switching/deletion; ordering and concurrency; once/weekly/day selection; startup coalescing; overnight and overdue stop; missed/offline starts; DST and clock changes; schema-1 migration; storage failures; credentials/recovery blocks; pause/completion races; DPI and modeless-window teardown.
