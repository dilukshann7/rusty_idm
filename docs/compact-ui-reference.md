# Compact native UI reference pass

Source implementation: 2026-10-19. The user asked for the supplied IDM download
status layout and compact sizing across Rusty's UI. This supersedes the earlier
large Segoe UI dialogs and 1250×650 default window metrics.

## Analysis

The prior dimensions were dialog units, not physical pixels. Combining a
420-DLU-wide resource with Segoe UI 9 made the dialogs much wider and taller than
the screenshots. The toolbar also reserved 96×68 logical-pixel buttons with
32-pixel icons and a 76-pixel band, adding unnecessary space.

Local font measurements at 96 DPI using System.Drawing estimated Segoe UI 9's
alphabet average at 6.63 px and line height at 15.96 px, versus Tahoma 8's 5.85 px
and 12.87 px. These are font measurements, not a rendered native-dialog test.
All dialog templates now use Tahoma 8; body controls use the same face/point size.
Windows continues to own caption/menu font sizing and monitor scaling.

| Area | Prior source dimensions | Revised source dimensions |
| --- | --- | --- |
| Address dialog | 420×92 DLU, Segoe UI 9 | 340×58 DLU, Tahoma 8 |
| Download File Info | 420×195 DLU, Segoe UI 9 | 348×146 DLU, Tahoma 8 |
| Transfer dialog | 380×172 DLU, large multiline label | 380×145 DLU collapsed, tabbed rows |
| Options | 380×100 DLU | 330×74 DLU |
| Recovery notice | 420×242 DLU | 380×185 DLU, scrolling text |
| Main window | 1250×650 logical px | 1000×540 logical px |
| Navigation / footer | 180 / 24 logical px | 140 / 20 logical px |
| Toolbar | 96×68 buttons, 32 px glyphs, 76 px band | 68×48 buttons, 24 px glyphs, 54 px band |

## Download status

The collapsed reference layout has three native tabs, a selectable source address,
Status/File size/Downloaded/Transfer rate/Time left/Resume capability rows, a
native progress bar, and Show details/Pause/Cancel footer buttons. Receiving-data
text and numeric values use the reference blue. The title includes the whole
percentage and filename; downloaded text includes a two-decimal percentage.
Size/rate use binary units labelled KB/MB/GB to follow the reference convention.

The dialog opens once per attempt, including very fast terminal outcomes that
coalesce away the Running event. Dismissing it leaves the transfer active and
does not reopen it on progress. Explicit retries open it again. Shutdown suppresses
new automatic windows. The main list's existing Details action still opens it.

Show details expands to 228 DLU in height using MapDialogRect at the window's
current DPI, revealing a scrolling, selectable text area for destination,
description, connection count and error/recovery paths. Unchanged text is not
replaced on every progress tick, preserving selection and reducing flicker.
Unknown size uses marquee without invented percentages or ETA. Empty completed
files show 0 Bytes and 100 percent. Marquee starts/stops only on mode transitions.

The later [transfer-controls implementation](transfer-controls.md) enables Pause,
Resume and the per-download Speed Limiter tab. Resume capability reflects whether
safe byte continuation can be attempted; otherwise resume restarts from zero.
Options on completion has a working,
default-off close-on-success checkbox; it is disabled after terminal results.

Engine snapshots expose a deliberately sanitized display URL: username,
password, query and fragment are removed. Original request URLs remain private.

## Verification

No compiler, Rust test, app, server or fixture was launched. Rustfmt and the
workspace's non-compiling source checks passed. Icon resources were regenerated
with 24/30/32/36/40/48/64/96-pixel frames and both enabled/disabled variants.
Written tests cover percentage precision and u64 bounds, unknown/zero length,
once-per-attempt opening/retry/dismissal, deferred jobs and retained paths, and
sanitized source addresses. They have not been executed.

One focused independent source review found no blocking issue. Its minor finding
about toggling close-on-completion after completion was addressed by disabling
that checkbox for terminal states.

Pixel-for-pixel rendering remains unverified: native metrics, theme and effective
display DPI must be compared against the running app when launch is authorized.
Pending manual checks: dialog at 100/125/150/200 percent DPI; tab focus/keyboard;
expansion and monitor changes; two concurrent automatic dialogs; window close
during transfer; retry; checkbox behavior; cancel versus completion; known,
unknown and zero-length bodies; long Unicode URL/path copying.
