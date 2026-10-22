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

## Floating media bar

Enable download panels and Video files in Rusty's browser panel options, then
reload this extension and refresh the video page. The compact **Download this
video** bar floats above the player. Drag its main button to move it; its offset
follows that player while scrolling and resets when the page reloads. **?**
explains supported videos and the **close button** dismisses the player's bar until reload.

With download panels enabled, visible videos prepare public format information
in the background as they load or play. Hovering or focusing a bar also starts
preparation. Cached formats remain in memory, are scoped to the tab/document and
source, and refresh near expiry. At most two native inspections run per browser.
No preview cache is saved to disk.

Click the bar to send a direct file to Rusty or open a numbered quality menu.
Each row shows title, container, quality, audio, and available size; click a row
to choose Rusty's destination. Escape or an outside click closes the menu.
Already prepared menus open without restarting yt-dlp. A cold lookup still shows
preparation status; play the video first to expose streaming sources. Rusty
continues to recheck the selected media/format before the destination dialog.

After a definite cancellation, choose the quality again to retry. Connection
failures retain the original submission receipt; retry that same quality. If
confirmation remains unknown after expiry, check Rusty's main list before
trying again. **Refresh formats** after a definite failure bypasses the preview
cache. The hover-only setting and draggable positioning continue to apply.
