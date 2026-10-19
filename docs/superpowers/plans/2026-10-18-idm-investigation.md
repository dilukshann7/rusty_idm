# IDM Engine and Browser Integration Investigation Plan

Approved 2026-10-18. Goal: produce an evidence-backed specification for an independent Rust download manager using installed IDM artifacts and controlled runtime tests.

## 1. Establish reproducible evidence

- Record paths, versions, sizes, and SHA-256 hashes for IDMan.exe, IDMMsgHost.exe, and the selected Chromium package.
- Preserve preliminary findings separately from verified behavior.
- Maintain a ledger with question, artifact hash, REA evidence ID, address/source location, conclusion, limits, and unknowns.
- Export evidence before closing native sessions; reuse retained analysis.

## 2. Trace the download engine

- Use REA/Ghidra; follow strings, imports, callers, callees, and focused decompilation.
- Investigate connection/range creation, limits and reassignment; pause/resume/restart state; validators, unsupported ranges, redirects and failures; queues, retries and throttling.
- Follow supporting libraries only from relevant traced dependencies.
- Record pseudocode as recovered analysis, including uncertain types and indirect calls.
- Inspect persistence only through code references and records created by test downloads.

## 3. Trace browser integration

- Inventory packages, safely extract a selected Chromium package, and inspect manifest/JavaScript.
- Trace browser action -> native messaging -> IDMMsgHost.exe -> IDM forwarding.
- Establish framing, types, fields, responses and errors from code; distinguish downstream IPC.
- Identify transfer of URLs, filenames, referrers, cookies and headers using synthetic values.
- Compare Firefox host declaration; defer full Firefox investigation.

## 4. Controlled experiments

- Python loopback fixture with deterministic files and timestamped logs.
- Baseline download/hash; valid/ignored ranges; unknown length; redirects; interruptions; temporary failures; delays.
- User-assisted pause/resume and restart; changed content/validators; two queued downloads; speed limit.
- Disposable Chromium profile with IDM extension; retain static findings if runtime integration is unavailable.
- Repeat only ambiguous/contradictory tests, retain settings/logs/hashes/outcomes, preserve existing downloads/settings and restore test changes.

## 5. Rust specification and reports

- Update IDM_REVERSE_ENGINEERING.md and write docs/idm-investigation.md; evidence under analysis/idm/.
- Define transfer, segment scheduling, state, persistence, throttling and browser boundaries.
- Specify minimum download request, state, progress, checkpoint and browser interfaces.
- Separate IDM observations from Rust choices; no private-format compatibility promise.
- Milestones: single stream, verified resume, segmentation, queues/limits, browser integration.

## Acceptance and boundaries

Every question is answered with evidence or explicitly unresolved. Completed IDM downloads match fixture hashes. Protocol claims cite code or captured messages. Windows and Chromium first. No GUI reconstruction, full scheduler parity, media extraction, licensing investigation, or Rust product implementation. No application dev server or builds. Brief manual runtime actions are authorized where Windows UI automation is unavailable.

## Execution decisions

- IDM was already running at execution start: do not terminate it for restart tests. Ask the user to perform restart only when their existing work permits it.
- Investigation tooling uses Python standard library. The runtime fixture binds IPv4 loopback only and does not edit IDM global settings.
- Reports must distinguish fixture self-checks from actual IDM requests and completed IDM files.
