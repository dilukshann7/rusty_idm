# Duplicate download dialog and behavior

Implemented in source on 2026-10-19 from the user's IDM screenshot. The native
dialog uses Tahoma 8 and a 270×165 DLU template: selectable URL, explanatory copy,
three radio choices, OK/Cancel and a remember checkbox. The third choice is the
default. Exact pixel matching and interaction are not runtime-verified.

After File Info is accepted, a matching link, existing destination or reserved
save path opens the duplicate dialog. Cancel submits nothing. Choices are:

- **Numbered filename:** add a separate request as `name (1).ext`, incrementing
  past files/directories on disk and every known destination. The engine still
  refuses an unexpected collision that occurs after the suggestion.
- **Overwrite:** a matching entry restarts under its existing ID and destination.
  Running or paused workers are cancelled and cleaned up before restarting;
  Stop/Stop All/Remove/shutdown cancel pending restart intent. Disk-only collisions
  use an explicit identity approval. A different active link using the same path
  must be stopped first or given a numbered filename.
- **Show/resume existing:** a completed entry shows its status; paused entries
  resume through normal admission, held entries start, and failed/cancelled entries
  retry from zero when no recovery artifacts need attention. A missing completed
  file is downloaded again. Already running entries keep running. Disk-only files
  show a completion notice. No file is launched automatically.

Matching compares normalized full URLs without fragments and compares explicit
authorization separately, keeping queries and credentials distinct. Private
requests remain in the engine; lookup returns only a sanitized JobSnapshot.
The query index stays current after event drains and is cleared on entry removal.
The engine rejects unapproved duplicate submissions and conflicting active paths,
including races with a just-submitted request.

Remember stores only the selected enum in settings; legacy settings default to
asking. Options has a Downloads group with an Ask checkbox and action list.
URLs, authorization and job history are never written to settings. Saving a
preference failure is reported without duplicating a submitted download.

## Guarded Windows replacement

Approval identifies the existing handle's volume/index, size and modified time.
Before streaming, the engine opens and verifies that exact file with write/delete
sharing denied; reparse points and directories are rejected. The original remains
at its name during download, cancellation, failure and sync. Once the new partial
is ready, the cancellation/commit lock encloses moving the original handle to a
unique backup, publishing the new handle without replacement, and rollback if
publication fails. An unexpected winner is never overwritten during rollback.
Successful publication deletes the owned backup through its handle. Recovery
errors expose both original-backup and partial paths when needed.

The two renames are not a crash-recovery journal. A process crash between them can
leave `.original` and `.part` files; these bytes remain on disk but automatic
recovery after restart is not implemented. Approved replacement is Windows-only.

## Verification boundary

The user's no-build/no-launch restriction remains in force. Rustfmt parsing and
source checks cover manifests/lock, XML, resource IDs/assets and documentation
links. An independent read-only review found no blocking duplicate-flow issue.
Tests are written for private matching, index/removal, numbered names, persistence,
cleanup-before-restart, cancellation of pending restart, approved HTTP replacement,
guarded identity, original preservation, rollback and raced destinations. They
have not been executed. Compilation, GUI/DPI comparison, HTTP fixtures and actual
native replacement behavior remain unverified.
