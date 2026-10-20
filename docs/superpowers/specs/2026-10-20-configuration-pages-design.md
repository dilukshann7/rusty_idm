# Compact configuration pages

Date: 2026-10-20
Status: Approved by the user on 2026-10-20; implemented in source with a final source review and source checks. Compilation and runtime acceptance remain deferred.

## Intent and accepted requirements

Replace Rusty's small Options form with the four configuration pages pictured by the user: General, File types, Save to and Downloads. Match the native Windows layout, grouping, labels, small icons and compact control sizes while retaining Rusty Download Manager branding. The user explicitly selected Chrome, Brave, Edge and Firefox support.

The screenshots are the visual requirements; their displayed paths, obsolete browsers, Internet Explorer user-agent and IDM branding are example content, not Rusty defaults. The additional Connection, Proxy/Socks, Sites Logins, Dial Up/VPN and Sounds pages are outside these four screenshots' implementation scope. Do not add inert tabs or buttons for unimplemented features.

This is a configuration feature project, not just a replacement resource template. Every included control must change actual app/extension behavior, and every Edit button opens a working editor. Keep the existing scheduler, recovery and compact native Windows UI.

## Current code and architecture

- `crates/win32/src/settings.rs`: schema-1 atomic JSON, default/category folders, duplicate choice, completion and double-click preferences. Preserve these settings during migration.
- `crates/win32/src/dialogs.rs` and resource 202: current modal Options entry. Download File Info is also modal; no provisional transfer is currently started while it is open.
- `crates/engine/src/categories.rs`: fixed extension classification. The engine currently saves partials near the destination, owns strict resumability, queue persistence and finalization journals.
- `extensions/chrome/`: Chrome MV3 capture, observed-request safeguards, explicit link context menu, browser-owned handoff and recovery ledger. Desktop capture policy and other browser packages are new work.
- Native-host config/registration currently permits one Chrome extension. Installer startup already uses an owned current-user Run entry.

Keep one engine scheduler and one authoritative saved settings/policy revision. Native callbacks enqueue owned actions. Filesystem/network/host work must not block native painting.

## Compact native presentation

Builder call: approximately 356 by 310 dialog units, Tahoma 8, normal DPI scaling, four native tabs and a shared small icon/section heading. The exact dimensions may be reduced or adjusted to prevent clipping at normal Windows scaling. Preserve the pictured alignment and section order; compress unused whitespace rather than shrinking readable text. Use Rusty's application title and Windows stock section icons.

Use genuine child dialog pages under the tab frame, with DS_CONTROL/WS_EX_CONTROLPARENT, appropriate clipping, a stable callback owner and keyboard navigation. Show only the selected page; never refresh a hidden page with WM_SETREDRAW(TRUE). Update text, icons, selection and enabled states only when values change. Retain draft values across tab changes. Browse/Edit windows inherit the same compact controls.

Footer: OK, Cancel and Help. OK validates and applies all four pages; Cancel discards the draft. Help opens local explanatory text for the current page without starting a browser. Do not open another instance of Options; foreground the existing instance. Invalid fields keep the dialog open on the relevant page.

## General page

Reproduce the browser/system integration heading and the following controls:

- Integration status and Restart: show enabled/needs-setup/unavailable for the registered browser hosts. Restart reconnects integration and refreshes policy; it must not kill browsers, restart Rusty, cancel accepted transfers or abandon pending handoff recovery.
- Launch Rusty Download Manager on startup: reflect the actual owned current-user startup entry. On OK, create/remove only Rusty's own quoted executable plus `--background` entry, using existing installer ownership rules. Preserve a conflicting/unrelated value and report failure.
- Automatically start downloading URLs placed on the clipboard: default off. Subscribe only while enabled. Establish a current sequence baseline on enable/startup; do not process pre-existing clipboard contents. Process one complete HTTP(S) URL from a new text update once, and use the normal download/duplicate/queue flow. Ignore arbitrary text, unsupported schemes and credential-bearing URLs. Do not record clipboard contents or repeatedly reprocess them. Unsubscribe on disable and exit.
- Browser checklist: Chrome, Brave, Edge and Firefox. Show installation/extension connection status. A check permits automatic capture only for that browser; the extension still needs its own one-time installation and host permission. Preserve the user's existing Chrome extension ID and capture preference.
- Add browser: select/recognize an installed supported browser executable or open the setup flow for one of the four supported families. Do not imply that any arbitrary executable can be intercepted. Multiple profiles/extensions may be registered under a browser; allowlisting remains explicit.
- Keys: configure prevent/force modifiers for automatic capture. Default no override. Record short-lived modifier intent in the extension for the originating tab/document; prevent wins when both match. Force bypasses the extension allowlist for a user-triggered download, but not excluded URLs/sites, unsupported schemes, unsafe request/credential checks or incognito restrictions. Missing/expired modifier evidence never forces capture.
- Context-menu Edit: configure which supported explicit actions are shown for links, selected URL text and direct media. Existing Download with Rusty behavior remains available by default. A disabled menu is removed from browser menus.
- Download-panels Edit: opt-in panels for direct HTTP(S) audio/video file sources on permitted pages, with selected media types and display behavior. Panels send an explicit user action through the same guarded handoff path. No stream extraction, DRM bypass, playlist assembly or speculative media downloading. Default off because this is new functionality.

## Browser implementation and handoff

Provide browser-specific Chromium packages/configuration for Chrome, Brave and Edge, and a Firefox package with its required background adapter and stable add-on identity. Reuse shared policy and handoff logic. Preserve Chrome's existing registration files/extension ID through a versioned migration; do not replace them merely to add another browser.

Use per-browser native-host manifests/configuration and explicit allowed caller identities. Firefox uses its own allowed-extension registration and invocation format. A page-provided browser label is not proof of browser identity. The builder chooses the package layout and exact browser-specific host names/config files, provided host attribution and allowlisting are enforced. Register/unregister only owned current-user entries and files; preserve unrelated hosts and startup entries.

The desktop supplies validated, bounded, versioned policy through native messaging. Effective capture requires the desktop browser switch, extension capture switch and granted host access. Extensions apply filters before pausing a browser download. Desktop admission rechecks its current policy so stale cached permissions cannot transfer ownership. Rejected/expired requests leave or restore the browser download through the existing lease/recovery protocol. Accepted downloads, pending receipts and ledger recovery remain idempotent across reconnects. No Cookie/Authorization values enter desktop policy or storage.

## File types page

- Editable automatic-capture extension list with Default. Accept case-insensitive extension tokens, normalize leading dots, remove duplicates and reject malformed/overlong tokens. Include the common types pictured plus existing supported common archive/document/audio/application/video types. Automatic HTML capture is not enabled merely because HTML belongs to Documents.
- Editable excluded-site list with Default. Space/newline-separated host patterns; support exact host and `*.` subdomains using parsed hostname comparisons, not substring matches. Show the matching syntax beside the field. Initial exclusions cover the update sites pictured; literal screenshot example domains are not authoritative beyond those defaults.
- Edit list for excluded download addresses: a compact add/edit/remove list of URL patterns. Builder call: exact normalized HTTP(S) URLs with optional trailing `*` for prefix matching; explain this instead of accepting an ambiguous full regex/glob language. Match full request and final redirected URLs; never strip query values before comparison. Bound pattern sizes and counts. Store private full-address rules in current-user encrypted data; do not log them.
- Exclusions govern automatic capture/clipboard admission. An explicit Add URL or Download with Rusty action may download an excluded address; it remains subject to the existing request validation and duplicate/overwrite approvals.
- Default restores only that section's factory values in the unsaved draft; it does not save other fields or immediately change capture.

## Save to page

- Category dropdown: General fallback, Compressed, Documents, Music, Programs, Video and custom categories.
- New/Edit category editors: name, extension set, default destination and remember-last-folder flag. General is permanent, accepts otherwise unmatched file types and has no editable extension list. Built-in names remain stable for existing category filters; their extension sets and destinations are editable. Custom categories may be renamed while updating owned settings/references without rewriting manually chosen file paths.
- Automatic inference is case-insensitive. Reject an extension assigned to multiple categories unless the user explicitly chooses to move it; show the conflicting category. Unknown types fall back to General. Existing explicit per-download category overrides win.
- Category folder Browse and Change folder on last selected affect future suggestions only. When the checkbox is off, browsing during one download does not overwrite that category's default. A manually edited save path must never be reset by inference, category edits or incoming metadata. Validate saved directories before applying; existing jobs retain their own destinations.
- Set file creation date as provided by the server: default off. When enabled, use a valid HTTP Last-Modified value for the completed file's Windows creation timestamp. Missing/invalid server dates leave the timestamp unchanged; failures are reported without silently deleting a completed download. Do not treat this timestamp as a replacement for resume validators.
- Temporary directory: folder field, Browse and the pictured explanatory text, shortened for the compact layout. Default new partials to Rusty's LocalAppData partial folder. Existing partials/checkpoints stay in their recorded locations; a settings change never moves live recovery data.
- Different-volume temporary storage requires a journaled, cancellable copy into a unique file in the final directory, sync and validation before the existing destination replacement protocol. Never emulate an atomic rename across volumes. Record copy/finalization phases and all owned file identities so restart can recognize completion or preserve ambiguity. Require fresh overwrite approval exactly as today; preserve the original and recovery data on failure. Old finalization journals remain readable.

## Downloads page

Reproduce the controls and section order from the fourth reference:

- Progress dialog Edit: choose opening/foreground behavior, compact details visibility and close-on-completion behavior. Preserve the current compact progress and separate compact completed dialog by default.
- Show start download dialog: default on. When off, use the resolved category/destination and configured duplicate policy; an overwrite that needs fresh approval still prompts. Unresolved credentials, missing folders and unsafe recovery states still require user attention.
- Do not start downloading, only add files to the queue: default off. Available when the start dialog is disabled; captures/admissions become held Main queue entries, or use the configured queue-selection panel when applicable. It must finish durable admission before acknowledging browser handoff.
- Show download complete dialog: reuse and migrate the existing preference. Queue processing is unchanged by these dialog preferences.
- Start downloading immediately while displaying Download File Info: builder call, default off to preserve Rusty's current admission/lease behavior. When enabled, create a cancellable provisional job through the existing engine and limits. Bytes may arrive into its owned partial, but final destination replacement/rename waits for Start confirmation. Persist a confirmation-required provisional record before acknowledging a browser handoff; that acknowledgement transfers ownership, so subsequent Cancel cancels Rusty's owned transfer rather than resuming a browser copy. If the bytes finish first, show a ready-for-confirmation state without committing the file; release the network slot. A restart restores an unfinished provisional record held and still requiring confirmation. Cancel preserves an unrelated/pre-existing destination and removes only verified owned temporary bytes under normal cleanup rules. Source, destination and credentials shown by the open dialog become read-only once transfer starts. Cancel revokes/cancels that owned job and uses normal cleanup; Start confirms that same job, never creates a duplicate. Download Later pauses it, waits for acknowledgement, then durably holds/promotes it to the selected queue; an already confirmed completion wins a pause race, while a ready-for-confirmation partial can still become a held queue entry. Never cancel the browser copy before acknowledged durable ownership. Provisional transfer/session shutdown and modal cancellation use the same engine lifecycle rather than a second executor.
- Show queue selection panel on Download Later: default off, retaining Main queue behavior. The panel lists permanent/custom queues by stable ID and can create a queue through the actor. Cancel returns to the download form without accepting the browser handoff; pending queue creation must be acknowledged before selection.
- Show queue selection panel on closing batch download dialogs: default on for a new minimal batch-URL entry flow. Since Rusty currently lacks batch entry, add a compact multiline Add URLs dialog reachable from Tasks/File. Validate each URL; show per-row validation/duplicate results and submit accepted rows through the same scheduler. On confirming the batch, one queue selector applies to held accepted rows. Closing/canceling with no accepted entries performs no submission. This adds URL batches only, not website grabbing.
- Ignore file modification time changes when resuming: default off. May ignore only a modification-time mismatch after file identity, durable length and full recorded bytes/hash checks pass. Keep strict Range/If-Range, expected response URL and representation validators. It never permits appending to unrelated or changed contents.
- Duplicate-link dropdown: Ask what to do, Add a numbered copy, Replace existing with normal overwrite approval, or Show/resume existing. Map to the current actor/duplicate workflow without automatic retries/redownloads of completed queue members.
- User-Agent for manually added downloads: default RustyDownloadManager/0.1.0. Validate as an HTTP header value, reject line breaks/control characters and bound its length. Apply to Add URL, batch and clipboard requests; do not falsely impersonate the screenshot's Internet Explorer browser. Capture requests keep a separately defined Rusty/browser integration request policy. Store request-level values required for queued recovery, not credentials.
- Virus checking Edit: optional executable plus an argument list containing a standalone file placeholder and enable switch. Default off. Launch only the configured executable directly with separately passed arguments after successful finalization; no shell or implicit PowerShell/cmd execution. Bound concurrency; keep the GUI responsive. Show scanner launch/nonzero-exit errors as scan errors without rewriting a completed transfer as a network failure or deleting its file. If scanning is enabled, defer completion Open actions until scanning finishes; a failed scan prevents automatic opening and leaves explicit user actions/reporting available.

## Settings application, defaults and compatibility

Migrate schema-1 JSON in memory to the new version with existing folders, completion, double-click and duplicate preferences intact. Preserve existing queue and recovery schemas through explicit defaulted/versioned additions. Corrupt/unsupported settings are preserved and reported; do not overwrite them on load.

New automatic behavior is opt-in: clipboard monitoring, media panels, immediate-during-info downloading, queue-only mode, timestamp rewriting, modification-time tolerance and antivirus scanning start off. Keep existing installed startup/capture choices. Start-dialog and completion-dialog defaults remain on. A future download uses the settings revision resolved at its admission; changing Options does not retroactively rewrite a running job's path, headers, partial location or cleanup policy. Global/per-queue concurrency and credentials-required handling remain unchanged.

Saving validates the whole draft first. Use staged atomic files and an ownership-checked startup/host update with rollback/compensation; do not report a successful application until all required mutations succeed. If rollback cannot restore an external setting, keep the last known values, expose a precise partial-apply error and let the user repair/retry; never silently show the requested state as actual. Notify the engine and connected extensions only after an accepted settings revision. Apply failures retain the draft and keep the previous active policy. No registry/setup actions occur merely by opening or editing Options.

## Implementation boundaries and deferred verification

Implement in the current `codex/queues-and-system-icons` checkout. Use existing Rust, Win32/common controls, serde, Tokio and pinned dependencies where practical. No web UI, new service, raw network interception driver, DRM/downloader extraction engine, broad credential forwarding or unrelated nine-tab feature suite. Native integration setup still requires each browser's one-time extension approval; selecting a checkbox is not that approval.

No builds, test additions/execution, fixture servers, application/browser launches, live registry/setup mutations, installs or release publishing without a separate explicit request. Source edits to implement those future runtime actions are in scope after design approval. Formatting, source/resource/manifest/script checks and source review are allowed and must be reported as source-only verification.

Deferred acceptance covers: tab painting/switching/keyboard/DPI without repeated sidebar refresh; settings migration/corruption/atomic-write and startup rollback; installed/missing browsers/profiles, Firefox adapter and native caller checks; extension policy revision/permissions/reconnect/lease races; extension/exclusion matching and modifier expiry; clipboard deduplication; category conflicts and manual paths; same/cross-volume finalization, copy cancellation and recovery; scanner errors; immediate-info confirm/cancel/completion/queue races; batch/queue-selector cancellation; unchanged validators and tampered partial rejection.

## Documentation references used during design

- [Microsoft Edge native messaging](https://learn.microsoft.com/en-us/microsoft-edge/extensions-chromium/developer-guide/native-messaging): browser-specific registration and explicit allowed origins.
- [Mozilla native messaging](https://developer.mozilla.org/en-US/docs/Mozilla/Add-ons/WebExtensions/Native_messaging): Firefox allowed-extension identities and invocation.
- [Mozilla background manifest](https://developer.mozilla.org/en-US/docs/Mozilla/Add-ons/WebExtensions/manifest.json/background): background adapter differences; use a Firefox-compatible package rather than assuming Chrome's service worker is portable.

## Calls to review before implementation

- Only the four supplied page contents are included; the five other page contents were not supplied.
- Keep Rusty branding, compact native pages, existing startup/capture choices and the working Chrome extension identity.
- New automation stays off until enabled. In particular, immediate transfer while File Info is open is implemented but does not become the migration default.
- Add direct URL batch entry to give the pictured batch-queue preference an actual workflow. Media panels support direct media file URLs only.
- Keep byte/identity/validator checks even when modification-time tolerance is enabled. Cross-volume temporary storage gets explicit copy/finalization recovery.
- No builds/tests/app launches, live installation/registration or release publishing are authorized by this design approval.
