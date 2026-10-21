# Protected browser capture and public video

The Chrome, Edge, Brave and Firefox extension profiles can offer a uniquely
observed HTTP(S) GET with Cookie or Authorization headers. Rusty opens File Info
with a separate unchecked approval for that download. If the request is
ambiguous, any redirect hop or header observation is missing, the approval is
declined, or the capture expires, the browser keeps the download. Rusty cancels
the browser copy only after its normal accepted receipt and fallback checks.
Protected captures used for address refresh also require the unchecked File Info
approval, with the existing destination shown for that job.

Request header values stay in process memory. Rusty uses each set only for its
exact observed redirect URL, and does not write values to browser storage, the
queue or logs. After restart, a held protected job needs a fresh browser capture
before it can use those headers again.
Confirming a restart from zero cannot bypass this capture requirement.

Media panels remain optional and require website permission. A permitted page
can show observed public HLS/DASH manifest requests or an individual public
YouTube watch/shorts page. Click the panel button to ask Rusty for a bounded,
two-minute format preview. The picker shows quality, container, audio and size
when available. Submitting a variant makes Rusty recheck its media identity and
format before opening the native destination dialog. Accepted jobs use the main
list and support progress, pause, resume and cancel. Incomplete media staging
paths are retained for recovery when a job fails or is cancelled.
Use **Retry** to continue the retained media stage after a failure or cancellation.
To remove an entry while preserving its recovery data elsewhere, move the listed
recovery files/directories first and then remove the entry again. Rusty clears
only paths that are confirmed absent; it does not delete those recovery files.

This release supports only single public on-demand videos. It does not support
playlists, live streams, DRM or videos requiring an account. Media extraction
does not import browser cookies. The installer and portable archive include
checksum-verified yt-dlp, FFmpeg/ffprobe and Deno with license notices. The
bundled tools are launched by absolute path, without a shell or user yt-dlp
configuration, plugins or remote components.

The extension and package checks exercise source behavior. A Rust build, live
handoff in all four browsers, public HLS/DASH downloads and a YouTube merge
still require separate acceptance runs.
