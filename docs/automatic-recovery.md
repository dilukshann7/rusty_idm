# Automatic download recovery

HTTP(S) downloads can recover automatically from temporary network and server
failures. This applies to manual URLs, browser captures, direct media files, and
queue entries. Media assembly jobs keep their existing recovery behavior.

## Settings

Open **Options → Connection → Automatic recovery**:

| Setting | Default | Allowed values |
| --- | --- | --- |
| Retries | 3 | 0–100; 0 disables automatic recovery |
| Delay between retries | 5 seconds | 1–3600 seconds |

These settings apply to newly added downloads. The initial attempt does not
count as a retry: three retries allow four attempts in total. Every retry uses
the same fixed delay. Existing saved settings and download options receive the
defaults when the new fields are absent.

## Behavior

Recovery covers connection failures, timeouts, interrupted or prematurely ended
bodies, and HTTP 408, 429, 500, 502, 503, and 504. Authentication failures, changed
files, invalid ranges, unsafe continuation, and filesystem/storage errors require
manual attention. An unconfirmed download also requires explicit confirmation.

The failed worker finishes and saves its checkpoint before the actor schedules
another attempt. Recovery reuses the same entry, partial files, range maps,
destination approval, and in-memory request credentials. Existing identity,
saved-byte, response-size, and validator checks still apply. An unsafe resume
preserves the partial and stops recovery; it does not silently restart the file.

The list and progress dialog show the retry number, countdown, and last error.
Pending retries remain waiting downloads. Only exhausted or permanent failures
become failed history entries. A successful recovery becomes a completed entry
with the original creation date and entry identity.

Waiting for a retry releases its transfer slot. Running queues continue other
files. A due retry still uses the global limit of two active files and its
queue's one/two-file limit. Stopping a queue blocks its retries until the queue
is started again.

Pausing a pending retry freezes its remaining delay; Resume preserves that
delay and its retry budget. Explicit Start begins immediately with a fresh
budget. Cancel, removal, and shutdown clear pending timers. Existing recovery
file guards still apply to removal.

Retry policy is saved with download options. Retry counters and deadlines exist
only in the current session. Restarting Rusty retains the existing held-download
startup behavior; it does not restore automatic retry timers or credentials.
Scheduling uses monotonic time, so changing the Windows clock does not shorten
or extend the actual delay.

## Verification boundary

Regression tests cover policy compatibility and bounds, scheduling and limits,
queue continuation and stopping, pause/resume, cancellation, removal, shutdown,
checkpoint reuse, final history, HTTP failures, interrupted streams and ranges,
unsafe resumes, and native status/settings mapping. These tests have been
authored but have not been compiled or executed for this change. Source checks
do not establish runtime correctness or native dialog layout acceptance.
