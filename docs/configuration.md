# Compact configuration (unreleased source)

Open **Options** for four compact native pages: **General**, **File types**,
**Save to** and **Downloads**. Changes are staged until **OK**. A failed save
keeps the draft open and the previous active capture policy. The worker validates
the draft, applies owned integration/startup changes and saves settings atomically;
failed changes are compensated. A repair message identifies any external value
that could not safely be restored.

## General and browsers

- Startup shows the actual current-user Windows startup entry. Closing Rusty
  keeps it in the tray; **Exit** stops it.
- Clipboard monitoring ignores existing contents when enabled. A newly copied,
  complete HTTP(S) URL without credentials passes the extension/site/address
  filters and ordinary duplicate/overwrite decisions. Each clipboard sequence is
  consumed once. Turning this option off removes the listener.
- Chrome, Brave, Edge and Firefox each require their own Rusty extension package,
  native host registration and one-time website-access approval. Browser
  checkboxes control automatic capture permission, not extension installation.
- **Add browser** stages an exact extension ID and optional installed executable.
  Copy the ID from that browser's extension details. Chromium IDs are 32 letters
  from `a` through `p`; Firefox uses its configured add-on ID. Existing profiles
  remain registered. **OK** invokes the packaged owned registration helper.
- **Restart** requests an extension/host reconnect on the next accepted save.
- **Keys** configures Ctrl/Alt/Shift combinations. Prevent wins. Force overrides
  the file-type list only; exclusions and unsafe/authenticated requests still
  remain in the browser. Modifier observations expire and are tied to navigation.
- The menu and panel editors control explicit link/selection/direct-media actions.
  Panels support direct audio/video HTTP(S) files, not streaming/DRM extraction.

The release packager stages the shared extension with browser-specific manifests
and native caller configurations. Firefox distribution/signing and live browser
installation require their usual separate release/setup steps. The existing
published v0.1.0 Chrome extension does not acquire these changes automatically.

## File types and folders

Extension names are space-separated and case-insensitive. Excluded sites accept
exact hosts or `*.example.com`; excluded addresses accept exact HTTP(S) URLs or
a trailing `*` prefix. Address exclusions are encrypted for the current Windows
user on disk. Section **Default** buttons only update the unsaved draft.

**General** is the permanent fallback category. The five built-in categories and
custom categories support extension rules, default folders and a remember-folder
switch. Moving conflicting extensions asks for confirmation. A custom category
rename also updates manually categorized entries through the engine. Automatic
classification never rewrites a manually selected destination.

New transfers normally use `%LOCALAPPDATA%\RustyIDM\partials`. A configured
temporary folder affects future admissions; existing partial locations remain
recorded. Publishing to another directory uses a journaled, verified staging
copy before guarded destination replacement. Server Last-Modified can optionally
set Windows creation time; it never replaces the normal resume validators.

## Downloads

- Start/completion dialogs remain enabled by default. Queue-only admission,
  clipboard monitoring, immediate File Info transfers, media panels, timestamp
  rewriting, modification-time tolerance and scanning default off.
- With the start dialog disabled, category folders and duplicate preferences
  apply. Required overwrite, credentials and recovery decisions still prompt.
- Queue-only takes precedence over immediate File Info downloading.
- Immediate File Info receives into one durable provisional transfer. Its
  destination and credentials are immutable. **Start Download** confirms that
  same transfer; final publication waits for confirmation. **Cancel** cancels
  Rusty's owned transfer. **Download Later** waits for pause acknowledgement and
  moves that entry into the selected queue. Unconfirmed received bytes release
  the connection slot; a restart restores the entry held for confirmation.
- Queue selection can be enabled for **Download Later** and batch URL admission.
  The picker uses acknowledged engine queue IDs and can create a queue. Cancelling
  a normal Later picker returns to File Info. Batch URL input is available under
  **Tasks → Add batch URLs**; up to 128 rows are admitted held, with per-row results.
  Existing duplicate entries are retained without starting them as a batch side
  effect.
- Progress display, foreground activation, initial details and close-on-finish
  are independently configurable. Explicit **Transfer Details** remains available.
- The custom User-Agent applies to manually added, batch and clipboard transfers.
  Browser captures keep Rusty's fixed User-Agent.
- Modification-time tolerance still requires file identity, recorded byte hashes
  and strict server Range/If-Range validation.
- Virus checking invokes the chosen `.exe` directly, with one argument per line
  and a standalone `{file}` argument. There is no shell command string. At most
  two checks run concurrently; each is bounded to ten minutes. Open/Open with
  waits while a check is pending. A launch, timeout or nonzero-exit failure keeps
  the downloaded file and requires explicit confirmation before opening it.

Queued entries and retained recovery records use versioned current-user-encrypted
storage. No passwords are persisted. Ordinary unqueued downloads retain their
existing session/exit lifecycle; this does not add full permanent history.

## Verification boundary

This feature has source formatting/syntax, resource/control-ID, manifest, asset,
JavaScript and PowerShell parser checks. These checks do not compile Rust or prove
desktop/browser behavior. No build, test suite, fixture server, application,
browser, setup, live registration or release publishing was run for this change.

Runtime acceptance remains deferred: compact layouts/painting/keyboard/DPI;
browser caller/permissions/reconnect/lease races; settings migration/rollback;
clipboard deduplication; custom category changes; scanner launch/failure/exit;
provisional confirm/cancel/Later races; batch cancellation; same/cross-volume copy,
interrupted finalization and identity/validator recovery.
