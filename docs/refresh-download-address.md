# Refresh download address (pre-alpha)

An HTTP(S) download can receive a new address without discarding saved bytes. Select a paused, failed or restored held job with a checkpoint, then use **Refresh download address** from its context menu. Paste a fresh URL or wait for a new browser download. If the earlier capture supplied a valid source page, **Open source page** is available.

Only one job can wait for a browser refresh at a time. Rusty asks before associating a new browser capture with that job. Declining leaves the download in the browser. The browser copy is cancelled only after Rusty verifies the saved partial and accepts the handoff lease. A failed or expired lease also falls back to the browser.

## Verification and recovery

The partial's owned file identity, saved length and hashes are checked first. Rusty then requests up to three exact ranges, each at most 64 KiB, from the beginning, middle and end of the saved bytes. Every probe must have an identity-encoded `206 Content-Range` with the recorded total, and the returned bytes must match. The replacement URL, final response URL, total and strong ETag (when present) are saved with the checkpoint before the job continues. The first continuation request and reconnects must also satisfy the engine's range checks. This is a sampled identity check; it does not prove equality of unseen bytes.

An ignored range, changed total, different sample, altered local partial, expired authorization or storage error stops refresh and preserves the partial. The engine reports the specific failure in the app. Explicit cancellation still cleans a verified owned partial. Unfinished jobs are restored held after restart, including ordinary downloads. A protected browser refresh needs a new capture after restart because its request headers are not stored.

## Browser context

Ordinary automatic capture keeps its credential restrictions. Only while a user has armed refresh may the extension attach `Cookie`, `Authorization`, `Referer` and `User-Agent` headers observed on the matching GET. Values are kept briefly in extension memory, scoped to each observed redirect URL and never written to browser storage or the durable queue. Rusty rejects malformed headers and will not replay those headers on an unobserved redirect. The capture wire format remains version 1; older captures can omit the new fields.

## Verification status

The JavaScript extension suite and source-only checks can run without building the app. Rust compilation, Rust tests and a live Windows/browser handoff remain unverified until a build is requested. See [validation](VALIDATION.md) for the wider project's validation history.
