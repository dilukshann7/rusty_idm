# Download complete dialog

A successful download opens a compact native completion window matching the
supplied reference's arrangement: folder icon, completion/size summary, Address,
saved-file path, Open, Open with..., Open folder, Close, and Don't show this dialog
again. Size uses binary megabytes plus the exact completed byte count. The
address uses the existing query/credential/fragment-redacted display URL.

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

Windows debug compilation passed. The source verifier passed for 59 Rust files,
including resource control IDs, syntax/formatting and links. The installed GUI
copy was updated and its hash matched the build output. No test suite, GUI
automation, reference pixel comparison or actual completion/action/persistence
acceptance was run. The existing unused `show_duplicate` warning remains.

Manual acceptance: successful completion and retry, cancel/failure exclusion,
multiple simultaneous completion windows, all three Open actions, missing files,
keyboard Close/Escape, saved opt-out and re-enable through Options.
