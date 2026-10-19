# Download complete dialog

A successful download opens a compact native completion window matching the
supplied reference's arrangement: folder icon, completion/size summary, Address,
saved-file path, Open, Open with..., Open folder, Close, and Don't show this dialog
again. Size uses binary megabytes plus the exact completed byte count. The
address uses the existing query/credential/fragment-redacted display URL.

The completion resource is now 310 by 132 dialog units (previously 348 by 170),
with 14-unit-high buttons, 13-unit-high path/address fields and tighter margins.
The progress window closes on a transition to Completed, leaving this completion
notification as the success window. Very fast downloads that first appear as
Completed do not briefly open a progress window. Properties can still be opened
explicitly from the list after completion and stays open for inspection. Failed,
cancelled and paused transfers keep their status windows.

The window appears once per observed successful completion, including a completed
retry. Failed, cancelled, merely paused and previously completed entries do not
create completion notifications. It is modeless, so other transfers can continue.
Closing it does not remove the download or its file. File/folder opening requires
an explicit button click and checks that the file/folder still exists. Open with
uses the native Windows single-file app chooser. Its cancellation is allowed.

Don't show this dialog again saves `show_completion_dialog: false` in the existing
local settings JSON. Older settings files default to showing completion dialogs.
**Options > Downloads > Show the download complete dialog** re-enables them. No
URLs, file histories or credentials are added to the saved preferences.

The original dialog's Windows debug compilation passed. Its source verifier passed for 59 Rust files,
including resource control IDs, syntax/formatting and links. The installed GUI
copy was updated and its hash matched the build output. No test suite, GUI
automation, reference pixel comparison or actual completion/action/persistence
acceptance was run. The existing unused `show_duplicate` warning remains.

The later automatic progress-close and compact-size revision has only received
source verification; it has not been compiled, installed, launched or visually
checked. No test suite was run for this revision.

Manual acceptance: successful completion and retry, cancel/failure exclusion,
multiple simultaneous completion windows, all three Open actions, missing files,
keyboard Close/Escape, saved opt-out and re-enable through Options.
Also check progress-window closure on success, a fast completion before the first
GUI update, reopening completed Properties, and the compact layout at each DPI.
