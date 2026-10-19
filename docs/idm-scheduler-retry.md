# IDM connection scheduling, throttling and retry boundaries

Date: 2026-10-18. Final bounded static pass using REA/Ghidra 12.1.4. Artifact: IDMan.exe SHA-256 `c7c80b420579d774f445b6b3f9282c0850760ef790373c0ece0f856a0a05d1d5`. No implementation code, builds, new downloads or settings changes.

## Findings

IDM separates a configured connection limit, a computed slot target, an initial launch count and a later growth boundary. Limiter settings can reduce startup parallelism and suppress further growth; rate enforcement also waits in the session receive loop. Recovery depends on the connection state and error path. The observed 30-second timeouts and conditional 2.5-second restart delay are not a universal retry schedule.

This closes the two requested follow-up passes. It provides an implementation-ready behavioral reference for an independent Rust engine, with the unresolved items below retained explicitly. It does not establish complete IDM equivalence.

## Connection target and configuration

Analyst field labels below describe the selected paths, not recovered source names. Session +0x1ac is a configured limit; +0x1b0 is the computed slot target; +0x1b8 is a growth boundary; +0x1b8 must not be interpreted as a continuously accurate count of active sockets. Slot records have a stride of 0x3488 and an active marker at slot +0x9c.

Constructor helper `0x5b87b0` calls rule lookup `0x4fed80`, then setter `0x5b8090` stores the result at session +0x1ac. Lookup compares descriptor type and a request string against configured entries, including wildcard matching, and otherwise returns the configuration object's default at +0x14. This supports request-specific limit selection; exact UI labels, matching grammar, rule ordering guarantees and cross-job/per-origin arbitration are not recovered. A per-request connection rule is not proof of a global per-origin connection budget.

Target helper `0x5b8120`, in its ordinary branch, computes remaining bytes from the total at +0x1c0 and transferred bytes at +0x1c8 (or an alternate count at +0xc8 when transferred is zero). For remaining bytes greater than 0x2800 (10,240):

1. Start from the smaller of the configured limit and floor(remaining / 10,240).
2. Apply the +0xf0 field as a lower-bound preference, constrained by the configured limit in this branch.
3. Clamp to 32.
4. When +0x188 is nonzero, apply a further cap of 4 for descriptor type 2, otherwise 8.

For remaining bytes at or below 10,240, the helper returns +0xf0 when nonzero, capped at 32; otherwise it returns 1. This short branch does not visibly reapply the configured limit. Negative/invalid configurations were not tested. Descriptor modes 4/5 have a separate floating-point calculation whose meaning is unresolved. The hard ceiling of 32 is a property of this helper, not a claim that all IDM traffic or every special mode is limited identically.

In `0x599b90`, a target of 1 can be promoted to 2 when the configured limit exceeds 1 and another remaining-size condition permits it. Descriptor flags can force the target back to 1. The initial launch count at +0x1b8 is derived from min(target, 2) and the preference at +0xf0, capped by the target. Other media/descriptor paths use different calculations and are outside the general rule above.

## Limiter interaction and staged growth

`0x5b7eb0` selects limiter enablement: mode +0x1a8 of zero inherits a global flag; nonzero mode returns enabled for mode 1. `0x5b7e80` chooses a session-specific rate at +0x170 when +0x178 is nonzero, otherwise a global rate. Rate units are inferred from byte/time arithmetic; exact UI conversion and global sharing across jobs remain unresolved.

The startup path in `0x599b90` reduces an initial count greater than 2 to 2 when limiting is enabled; inside that reduction branch, a rate at or below 0xc00 (3,072 internal units) reduces it to 1. **The latter is nested inside the greater-than-two branch**, so it is not a universal rule forcing every low-rate startup to one connection.

Growth routine `0x594570`, called from the receive loop `0x597b60`, gates expansion on the growth boundary being below the target, internal mode +0x13c not being 2, and additional descriptor/session flags. When limiting is enabled, it returns without expansion if the stored peak-rate metric at +0x1d4 is at least floor(rate × 85 / 100).

After more than one clock second since the growth timestamp, with inactive slots available and its active/progress readiness conditions satisfied, the routine normally doubles the growth boundary up to the target. A flagged path uses 2→3, 4→6 and 6→8 instead. It initializes eligible inactive slots and calls their virtual start method; selected special modes also call the previously recovered chunk-insertion routine `0x55efc0`. Early break conditions exist, and the final boundary assignment can differ from actual successful launches. This is staged growth, not proof that every download doubles connections on a fixed timer.

`0x597b60` uses select/read dispatch and limiter accounting within the same session loop. Its limiter branch compares bytes transferred over elapsed milliseconds with the selected rate, computes an excess-rate wait, adjusts it with other speed metrics and caps the wait at 12,000 ms. It uses Sleep when an auxiliary socket is absent, otherwise select with a timeout. Therefore a Sleep found here is rate pacing, not necessarily a retry backoff. `0x5b8a20` updates rate/progress statistics; it also contains separate 100-ms message-acknowledgment waits that must not be labeled network retry timing.

## Timeout and retry paths

Clock helper `0x628c41` calls __ftime32_s. `0x5658f0` computes the difference in clock seconds from the saved connection timestamp and leaves that difference in EAX; assembly confirms the return even though Ghidra labels the routine void.

| Path | Statically observed behavior | Boundary |
| --- | --- | --- |
| Reader `0x58db10`, states 1 / 0x1c | Selected no-readiness branches reconnect after elapsed time reaches 30 seconds, moving received progress into the next request offset. | Other events may restart sooner; timestamp initialization and all state names remain incomplete. |
| Reader timeout (3) | Threshold expression is 300 seconds in session mode 2, otherwise 30 seconds, subject to TLS/read-state guards. | Mode 2 is not a numeric connection cap. |
| Inactive/readiness helper `0x584060` | Several state groups also use 30-second thresholds; state 0 uses a per-connection field. | Different states have distinct cleanup and restart paths. |
| Session restart s011 in `0x599b90` | A guarded early-restart path sets a one-shot flag; descriptor flag 0x10 adds Sleep(2500), then reconnects. | Does not establish a default delay or exponential backoff. |
| URL fallback `0x566650` | Selected googlevideo URL handling rewrites a candidate and fallback_count before a retry. | Service-specific fallback, not a generic retry-count policy; media extraction remains deferred. |
| Context recovery `0x5972b0` | May refresh cookie/request context, reconnect and retry header handling. | No cookie values were inspected or retained by this static analysis. |
| Resume-byte mismatch | Shared session counter controls local recovery versus the restart path. | See [overlap recovery](idm-overlap-validation.md) and [session lifetime](idm-session-lifetime.md); not three user retries. |

No general retry maximum, exponential backoff, HTTP Retry-After policy or guaranteed automatic 503 behavior was established. Earlier interrupted/503 fixtures completed correctly, but browser/manual actions prevent assigning their timing to a particular native branch.

## Evidence ledger

| Finding | Procedure | REA evidence ID |
| --- | --- | --- |
| Configured limit setup | 0x5b87b0 | `ev_4766880da2fcda52bc4e60075d60d8ad9249daf147dc954d2a6fe443922e5d80` |
| Request rule lookup | 0x4fed80 | `ev_245057899011869d033310771cc7e2b5f97d2eff56f045c01e07c061cb505a14` |
| Target calculation | 0x5b8120 | `ev_37405982f6bd07370d3d29ccb2d5a1a7884f45460c806dfe22370dbe3463df1f` |
| Startup counts / s011 | 0x599b90 | `ev_078ef60602434b531e6544db06d40bef570025c48e638e9e9fa50b1b9a038b88` |
| Staged expansion | 0x594570 | `ev_169ca90cf8d77f193692579851053f8d5e46680434240f9a4779a3999f5f6c2e` |
| Limiter enablement | 0x5b7eb0 | `ev_c73a397b1bc96fdf4dee000106df41bbe1089f3b8ed2f91fb284d6e5a10cbe68` |
| Rate selection | 0x5b7e80 | `ev_43d5b09f44f7a0713ede27de6daf4f7a32aca937a59299e5f85efcd66f4346fa` |
| Receive loop / pacing | 0x597b60 | `ev_cb0d052c14949ad6e7f9400b95eb71241824c2819126f0a7b72b7c071732bd07` |
| Statistics / acknowledgment waits | 0x5b8a20 | `ev_952569828687d856a9c8cc2ad0ab32a886a185c353e2de9ae3b19370ad6311ec` |
| Elapsed-time helper assembly | 0x5658f0 | `ev_97371dd4c934691dac7da804a06ed1d45fb7b7cb334192e2bdc05038c1b5192a` |
| Clock source | 0x628c41 | `ev_51736810450dfeb3a6cf8d85514f72408c9a093702ed6b0f4b397feac6454a44` |
| Readiness timeout handling | 0x584060 | `ev_2dfffb4ea874294eea8eaa8e32d7950fddd8816c68de638708ab2f543bad3e30` |
| Service-specific URL fallback | 0x566650 | `ev_bbdd14f78a495c7b3dca0773b96cee4cbba0e6206be6e17000092bd47df7f308` |
| Context-based recovery | 0x5972b0 | `ev_c0fa92b63b137bda04d4a52b33cc13ab2d70835fcf5fec70c1a12912b8c22466` |

The earlier reader extracts remain under `overlap-*.txt`; the complete exported bundle retains prior evidence. New extracts are `scheduler-*.txt`. Evidence and snapshot were exported before closure: `scheduler-evidence.json` and `scheduler-snapshot.json` (154 records). [The index](../analysis/idm/native/scheduler-index.json) records identities and verifies cited evidence IDs. Provider unknown counts do not represent the number of unanswered investigation questions.

## Completion boundary and Rust specification

Answered for traced paths: request-specific limit lookup, size-based target, startup reductions, staged expansion, limiter-rate selection, pacing, selected timeout thresholds, and distinct recovery routes. Partially answered: exact configured-setting provenance, all flag meanings, cap precedence across special modes, and alternative/split policy. Unresolved: queue-wide arbitration, cross-job bandwidth sharing, general retry/backoff policy, exact UI pause/resume routing and private checkpoint layout. Browser runtime frame capture and malformed/cross-origin cases remain outside established evidence.

For Rust, retain the [independent interfaces and milestone order](idm-investigation.md). Implement explicit global/per-origin limits, cancellable timeouts and waits, a shared rate budget, and a documented bounded retry policy. Keep slot target separate from active transfers. These are deliberate design choices; copying IDM's numeric thresholds is unnecessary. The investigation is sufficient to begin that implementation when requested. No additional investigation pass is scheduled automatically.
