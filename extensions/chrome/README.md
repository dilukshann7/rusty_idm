# Rusty for Chrome

Windows 10/11, Chrome 120 or later. Load this folder as an unpacked extension
only when you are ready to set it up. Build `rusty-win32` and `rusty-native-host`
separately, then register the exact extension ID with
`scripts/register-chrome-host.ps1` in the repository. Detailed commands and the
acceptance checklist are in `docs/chrome-integration.md` in the repository.

Capture is off initially. Enabling it asks for optional HTTP/HTTPS access.
The link context menu, **Download with Rusty**, works without automatic capture.
It makes a fresh public GET request and opens Rusty's File Info dialog.

Automatic capture requires one uniquely observed safe GET without browser
credentials. Authenticated, POST, ambiguous, incognito and blob/data/file
downloads stay in Chrome. Referrer or user-agent requirements may prevent
Rusty's separate request from succeeding.

Chrome's copy is paused during File Info. An actor-confirmed Rusty receipt and
current acceptance status are required before Chrome cancellation. Cancelling
File Info or losing the host returns eligible paused downloads to Chrome.
Interrupted handoffs are reconciled with session metadata; if a cancellation
or resume fails, check Chrome Downloads and the Rusty list before retrying.
Rusty's transfer history is not crash persistent; a connection failure around
acceptance can result in an extra copy.

Only the capture preference is stored permanently. In-flight metadata is held
in extension session storage. Use the popup to check connection and recovery.
The extension does not import cookies or passwords, erase Chrome history, or
remove downloaded files. Use the repository's unregister script to remove
only the owned per-user native-host registration. Both setup scripts support
`-WhatIf`. No automatic setup or browser changes are performed by source checks.
