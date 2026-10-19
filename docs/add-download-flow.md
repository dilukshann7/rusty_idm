# Add Download flow

Implemented from the two user-supplied dialog references, using actual native
Windows controls. No build, test, app or fixture was launched for this change.

1. Enter New Address to Download: address, optional HTTP Basic authorization,
   login/password, OK/Cancel.
2. Download File Info: URL, category, Save As/Browse, remember folder for category,
   description, Download Later/Start Download/Cancel.

URL/filename checks occur before moving to File Info. Filesystem checks occur
after the native dialog has returned, outside the window procedure. Final output
still uses the existing handle-based no-replace behavior. No network metadata
probe blocks either dialog.

Download Later creates a deferred Waiting entry that is not admitted until the
user selects Downloads → Start Download. No new scheduler, pause/resume or durable
job history is implied. History and deferred downloads last for this session.

Only folder preferences persist, including the category-to-folder map. Existing
version-1 settings remain compatible. Credentials remain in engine-private memory
for the request/retry lifetime and are omitted from request Debug and snapshots.

Tests were written for filename decoding/safety, authorization privacy and
transmission, settings compatibility/folder mapping, and explicit deferred start.
Execution remains deferred under the user's no-build instruction. Independent
source review found no blocking issue and the Options wording was corrected.
