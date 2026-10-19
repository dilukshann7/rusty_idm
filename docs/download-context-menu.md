# Download list context menu

Right-click a download row to open a native Windows menu in the reference's
order, with separators, disabled entries and nested submenus. Shift+F10 or the
keyboard Menu key opens it beneath the selected row. Right-clicking empty space
does not act on a previously selected download.

Available actions:

- Open and Open with... require a completed download and an existing file.
- Open folder checks the containing directory and works during a transfer too.
- Redownload is available for terminal entries without retained recovery files.
  It asks for confirmation, then uses the engine's guarded restart/replacement
  flow. The selected JobId stays attached to the action while the menu is open.
- Resume Download starts deferred entries or resumes paused transfers.
- Stop Download cancels an active transfer using the existing Stop command.
- Remove removes the entry using the engine's existing cancellation/cleanup flow;
  it does not delete a completed file.
- Properties opens the existing transfer details window.
- On Double click saves Open, Open folder or Properties in local settings.
  Properties is the default for existing settings files. Choosing Open still
  shows Properties for unfinished downloads. Enter continues to open Properties.

Move/Rename, Refresh download address, Add to queue and Delete from queue are
shown disabled. Rusty does not yet provide these operations or named queues.
The displayed Ctrl-M hint belongs to the disabled Move/Rename item; it does not
register an active shortcut.

This change has not been compiled, installed, launched or exercised through the
GUI. No test suite was run. Existing settings test fixtures were adjusted for the
new preference field. Manual acceptance remains: mouse targeting including empty
space/subitems, keyboard invocation, state-specific enabled items, each supported
action, cancelling Redownload, missing files, double-click preference persistence,
and selection/state changes while the menu is open.
