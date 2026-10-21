export const operation = (op, captureId) => ({
  op,
  payload: { capture_id: captureId },
});
export async function nativeStatus(native, id) {
  try {
    return await native.send(operation("status", id));
  } catch {
    return { status: "unknown" };
  }
}
export async function abortCapture(native, id) {
  try {
    return await native.send(operation("abort", id));
  } catch {
    return { status: "unknown" };
  }
}
export async function fallback(
  record,
  browser,
  native,
  ledger,
  { keepAttention = false } = {},
) {
  await ledger.put({ ...record, phase: "fallback_pending" });
  // Revoke before resuming. If unreachable, preserve Chrome and accept the documented extra-copy race.
  await abortCapture(native, record.id);
  const latest = await ledger.get(record.id);
  record = { ...record, ...latest };
  try {
    const item =
      record.downloadId == null ? null : await browser.get(record.downloadId);
    if (
      item?.state === "in_progress" &&
      item.paused &&
      record.pausedByUs &&
      !record.userPaused
    )
      await browser.resume(record.downloadId);
    if (
      item?.state === "in_progress" &&
      item.paused &&
      !record.pausedByUs &&
      !record.userPaused
    ) {
      await ledger.put({
        ...record,
        phase: "fallback_pending",
        attention: "pause_ownership_unknown",
      });
      return "needs_attention";
    }
    if (keepAttention) {
      await ledger.put({
        ...record,
        phase: "done",
        attention: "browser_cancel_failed",
      });
      return "needs_attention";
    }
    await ledger.remove(record.id);
    return "continued_in_chrome";
  } catch {
    await ledger.put({
      ...record,
      phase: "fallback_pending",
      attention: "browser_resume_failed",
    });
    return "needs_attention";
  }
}
export async function finishAccepted(record, reply, browser, native, ledger) {
  record = {
    ...record,
    ...(await ledger.get(record.id)),
    phase: "accepted",
    receipt: { job_id: reply.job_id, created_new: reply.created_new },
  };
  await ledger.put(record);
  // An immutable replay receipt may outlive an owned-job abort; confirm current status.
  const status = await nativeStatus(native, record.id);
  if (
    status.status !== "accepted" ||
    status.job_id !== reply.job_id ||
    status.created_new !== reply.created_new
  )
    return fallback(record, browser, native, ledger);
  if (record.downloadId == null) {
    await ledger.remove(record.id);
    return "accepted";
  }
  let browserCancelled = false;
  try {
    const item = await browser.get(record.downloadId);
    const latest = await ledger.get(record.id);
    record = { ...record, ...latest };
    if (
      record.cancelRequested &&
      item?.state === "interrupted" &&
      item.error === "USER_CANCELED"
    ) {
      // A worker may have died after our successful cancel but before the next storage write.
      await ledger.remove(record.id);
      return "accepted";
    }
    if (
      !item ||
      item.state !== "in_progress" ||
      !item.paused ||
      !record.pausedByUs ||
      record.userPaused
    ) {
      if (reply.created_new) await abortCapture(native, record.id);
      await ledger.remove(record.id);
      return "continued_in_chrome";
    }
    record = { ...record, cancelRequested: true };
    await ledger.put(record);
    await browser.cancel(record.downloadId);
    browserCancelled = true;
    await ledger.put({ ...record, phase: "browser_cancelled" });
    await ledger.remove(record.id);
    return "accepted";
  } catch {
    // Once Chrome cancellation succeeded, responsibility must stay with Rusty
    // even if a later storage operation fails. The pre-cancel record survives.
    if (browserCancelled) return "needs_attention";
    if (record.cancelRequested) {
      try {
        const current = await browser.get(record.downloadId);
        if (
          current?.state === "interrupted" &&
          current.error === "USER_CANCELED"
        )
          return "needs_attention";
      } catch {
        return "needs_attention";
      }
    }
    if (reply.created_new) await abortCapture(native, record.id);
    return fallback(record, browser, native, ledger, { keepAttention: true });
  }
}
export async function handoff(
  downloadId,
  request,
  browser,
  native,
  ledger,
  now = Date.now,
) {
  const id = request.id || crypto.randomUUID();
  const expiresAtMs = Math.min(
    request.expires_at_ms ?? now() + 120000,
    now() + 120000,
  );
  const existing = await ledger.list();
  if (
    existing.length >= 32 ||
    existing.some(
      (r) => r.id === id || (downloadId != null && r.downloadId === downloadId),
    )
  )
    return "continued_in_chrome";
  let record = {
    id,
    downloadId,
    phase: "offered",
    pausedByUs: false,
    expiresAtMs,
  };
  await ledger.put(record);
  if (downloadId != null) {
    let pauseConfirmed = false;
    try {
      const item = await browser.get(downloadId);
      if (!item || item.state !== "in_progress" || item.paused) {
        await ledger.remove(id);
        return "continued_in_chrome";
      }
      await browser.pause(downloadId);
      pauseConfirmed = true;
      const latest = await ledger.get(id);
      record = {
        ...record,
        ...latest,
        phase: "paused",
        pausedByUs: !latest?.userPaused,
      };
      await ledger.put(record);
      const paused = await browser.get(downloadId);
      if (
        !paused ||
        paused.state !== "in_progress" ||
        !paused.paused ||
        !record.pausedByUs
      )
        return fallback(record, browser, native, ledger);
    } catch {
      if (pauseConfirmed)
        return fallback(
          { ...record, pausedByUs: !record.ownershipLost },
          browser,
          native,
          ledger,
        );
      await ledger.remove(id);
      return "continued_in_chrome";
    }
  }
  if (now() >= expiresAtMs) return fallback(record, browser, native, ledger);
  record = { ...record, phase: "pending" };
  await ledger.put(record);
  let reply;
  const {
    id: envelopeId,
    expires_at_ms: originalDeadline,
    ...payload
  } = request;
  try {
    reply = await native.send({
      id,
      op: "capture",
      payload: { ...payload, expires_at_ms: expiresAtMs },
    });
  } catch {
    reply = await nativeStatus(native, id);
  }
  if (reply.status === "accepted")
    return finishAccepted(record, reply, browser, native, ledger);
  if (reply.status === "pending" && now() < expiresAtMs)
    return "needs_attention";
  const outcome = await fallback(record, browser, native, ledger);
  return downloadId == null ? "needs_attention" : outcome;
}
