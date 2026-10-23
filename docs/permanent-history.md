# Permanent download history (source)

Completed and failed downloads remain in the main list across restarts until
you delete their entries. This includes ordinary downloads outside a queue.
All Downloads, Finished, Unfinished and category filters use the same records.
Explicitly cancelled ordinary downloads retain their existing cleanup behavior.

History preserves the destination, source address/page, category, description,
queue association, media selection, last result, byte counts and dates. The
Last Try column survives restart. Transfer Details shows Added, Last Try and
Finished in Windows local time. Older records display Unknown where dates were
never recorded. A missing downloaded file does not remove its history entry;
Open remains unavailable until that file exists.

Retry/redownload reuses the same entry and keeps its Added date, while recording
the latest attempt and result. Restored completed or failed records do not start
automatically or replay completion dialogs, scanner jobs or progress windows.
Interrupted downloads remain held for review. Existing partial-file checks,
credential prompts, fresh browser capture and replacement approvals still apply.

Delete removes the selected entry after its storage update succeeds. Delete
Completed removes eligible completed entries. Neither deletes completed files.
Recovery files still require the existing cleanup/recovery decisions. A failed
save leaves the entry visible, with the previous storage file preserved.

History remains until manually removed; there is no age-based or automatic
pruning. The shared store supports 10,000 entries and at most 16 MiB. At capacity,
new additions are rejected with a message to delete history entries or use
Delete Completed. Existing records remain available for cleanup.

The store remains `%LOCALAPPDATA%\RustyIDM\queues.v1.dpapi`, encrypted for the
current Windows user and replaced atomically. Schema 8 reads schemas 1–7 and
migrates on the next successful save; it adds optional [Site Grabber](site-grabber.md)
membership. Older binaries reject schema 8. Existing
IDs, queues, checkpoints and journals are retained. Previously discarded records
cannot be reconstructed. Passwords, cookies and authorization headers are not
persisted. Stored failure explanations use fixed, sanitized wording rather than
arbitrary server or media-tool messages.

If a final history write fails after a file has been published, the download
remains Completed. Unsaved metadata stays in memory for retry, and storage
errors are reported. New admissions are rejected and queued starts wait while
pending history cannot be saved; retry occurs every five seconds and once
during shutdown. Closing the app while storage remains unavailable can lose
unsaved metadata; the previous durable state and any recovery journal remain
available for the next startup.

This implementation has not been built or runtime-tested. See
[validation](VALIDATION.md) for the source checks and deferred acceptance cases.
