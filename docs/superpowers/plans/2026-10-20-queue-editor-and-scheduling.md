# Queue Editor and Timed Scheduling Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** Open a compact native queue editor after acknowledged creation and add opt-in engine-owned startup/once/weekly/stop scheduling.

**Architecture:** Extend encrypted queue definitions and snapshots with schedule/configuration revisions. One engine scheduler drives existing queue commands; a modeless editor submits correlated commands and renders acknowledgements. Native callbacks enqueue actions only.

**Tech Stack:** Existing Rust workspace, Tokio actor, serde/DPAPI atomic store, windows 0.62.2 common controls and time APIs.

**Spec:** ../specs/2026-10-20-queue-editor-and-scheduling-design.md (approved by user)

## Global Constraints

- Implement on the existing codex/queues-and-system-icons feature branch in D:/Programming/Rust/rusty_idm; keep the normal project folder usable.
- Automatic starts default off; global concurrency remains two and per queue remains one or two.
- No builds, tests/test additions, fixture servers, application launches or publishing. Update existing fixture literals only for interface compatibility. Use source/resource checks and independent source review.
- Preserve stable IDs, current-user encryption, credential exclusion, recovery data and actor ownership.
- One fresh GPT-6.1 Sol review of the complete change after inline implementation; no implementation subagents.

## Review Focus

- Persist a scheduled occurrence before starting, including startup/timed coalescing, DST repeats and clock rollback.
- Overdue overnight stops must pause on wake without postponement or a new automatic start.
- Creation/configuration failures must not open/select an unrelated queue or silently discard dirty drafts.
- Modeless callbacks and teardown must preserve stable owners and avoid borrowing AppState inside callbacks.
- Schema-1 migration must preserve queue membership, credentials flags, checkpoints and journals.

## Task 1: Schedule types, clock and storage migration

**Files:** Create crates/engine/src/schedule.rs; modify engine lib.rs, queues.rs and storage.rs.

**Interfaces:** Public Date, LocalStamp, StartRule, QueueSchedule, QueueSettings and ScheduleStatus; QueueSchedule::validate()/validate_for_apply(now), due(now,last), next_start(now,last); LocalStamp::now()/next_day(); persisted ScheduleRecord with generation/high-water state. QueueSnapshot gains configuration revision, applied schedule and computed schedule status.

- [x] Define minute-precision civil date/time values with Gregorian validation, ordering, weekday and next-day operations. Get local time through existing Win32 time APIs; keep the civil calculation separable from clock access.
- [x] Implement disabled-default schedule configuration, exact-minute once/weekly due decisions, future occurrence lookup and next strictly-future stop calculation.
- [x] Add defaulted schedule/configuration revision fields to StoredQueue; accept schema 1 and migrate in memory to schema 2, saving only on a later accepted mutation.
- [x] Source-inspect once/weekly/all-days/empty-days/past-date rules, leap dates, skipped starts, overnight/overdue stops and stable occurrence comparison. Runtime verification remains deferred.
- [x] Commit coherent schedule/storage model.

## Task 2: Engine scheduling and acknowledged editor commands

**Files:** Modify engine service.rs, service_queues.rs and events.rs.

**Interfaces:** Command::CreateQueueForEditor { request, name }; Command::ConfigureQueue { request, id, settings }; correlated Event queue creation/configuration results. QueueRuntime::clock_tick(jobs,waiting,bus) and shared start/hold paths.

- [x] Persist queue creation before emitting the correlated QueueId result; bound pending acknowledgement storage and preserve existing CreateQueue command behavior.
- [x] Apply schedule and parallelism atomically, advance configuration revision, and advance schedule generation/reset occurrence only when schedule itself changes.
- [x] Route manual/startup/timed starts through one start path. Arm strictly-future stop deadlines only for a newly running queue; process overdue stops before automatic starts.
- [x] Persist consumed timed occurrence before start. Coalesce startup/timed starts, consume once schedules, skip failed/completed/blocked entries through existing scheduler rules, and hold/report on persistence failures.
- [x] Source-inspect acknowledgement failures, clock rollback/DST, restart defaults, shared concurrency, stop races and corrupt-store behavior. Commit engine integration.

## Task 3: Native modeless queue editor

**Files:** Create win32 scheduler.rs and scheduler_form.rs; modify queue_ui.rs, commands.rs, lib.rs and resources/app.rc.

**Interfaces:** SchedulerWindow::open(owner,view,queue,tab), poll(view)->Vec<EditorAction>, result(request,result), is_dialog_message(msg), close_all(); EditorAction carries engine commands, NewQueue and user-facing errors. A stable signal owner collects callbacks without AppState access.

- [x] Build compact native queue tree, tabs, file report, schedule date/time/day controls, one/two parallelism and footer/file-order actions. Add resource/control IDs and common-control classes.
- [x] Reuse ShellImages for asynchronous queue/file icons and DPI invalidation; refresh by stable QueueId/JobId and show engine status/counts.
- [x] Maintain an applied baseline plus editable draft and correlated pending Apply. Progress updates never overwrite dirty values. Apply/Discard/Cancel resolves queue switches, close, New queue and Start now; Stop is immediate.
- [x] Match successful Apply to both its acknowledgement and a newer queue revision before running its deferred intent. Preserve edits on failure. Marshal keyboard navigation and teardown outside callbacks.
- [x] Source-inspect empty/deleted queues, failed Apply, immediate file actions, keyboard focus, DPI changes, control enablement and lifecycle cleanup. Commit native editor.

## Task 4: Sidebar workflow, docs and final review

**Files:** Modify win32 app.rs, controls.rs, context menus/resources, README.md and docs/queues-and-system-icons.md.

- [x] Add pointer/keyboard queue-tree context targeting with pinned QueueId, root Create action, and state-aware Start/Stop/Edit/Schedule/Delete/Create commands.
- [x] Add main-menu editor entry. On correlated create success, open/select the acknowledged new queue on Schedule after its snapshot exists; errors/cancel must not open it.
- [x] Poll the modeless editor in the existing message loop; route its commands/actions, acknowledgements, dialog-key processing and close/session teardown.
- [x] Document schedule defaults, missed/startup/stop behavior and the deferred acceptance matrix.
- [x] Run cargo fmt, scripts/verify-source.ps1 and git diff --check only. Request one independent GPT-6.1 Sol source review, fix material findings and repeat source checks. Commit remaining integration and report precise verification boundaries.
