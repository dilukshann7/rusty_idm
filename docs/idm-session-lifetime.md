# IDM HTTP-session lifetime and mismatch counter

Date: 2026-10-18. Static analysis of IDMan.exe SHA-256 `c7c80b420579d774f445b6b3f9282c0850760ef790373c0ece0f856a0a05d1d5` using REA/Ghidra 12.1.4. No product code, builds, runtime probes or IDM settings changes.

## Result

The traced HTTP transfer wrapper creates a new session for each invocation and destroys it on its normal cleanup path. The mismatch counter therefore starts at zero for a newly constructed session, including the explicitly traced outer retry that recursively re-enters the wrapper. A retry inside a connection reader can instead continue within the existing session after incrementing its shared counter. **A mismatch count is not a count of user resume clicks or a permanent download-history total.**

The exact UI pause/resume dispatch remains unresolved: this pass establishes the wrapper's lifetime, not that every possible resume command enters it. No claim is made that successful comparisons clear the count, or that every internal retry preserves it without another intervening write.

## Construction, execution and destruction

`0x5ca290` has two branches that construct HTTP sessions. Assembly confirms an allocation request of `0x890` bytes at `0x5ca3eb` / `0x5ca677`. The returned pointer is passed in ECX to constructor `0x594ec0` at `0x5ca418` / `0x5ca6a4`. This is fresh allocation, not merely assigning a retained session pointer: allocator `0x5f93a8` calls malloc, with an allocation-failure callback fallback.

The owner stores the session at owner +0x960. Setup `0x5b9340` initializes request/persistence context, then the wrapper invokes the session's virtual function at vtable +8. Constructor assembly installs vtable `0x6b7490`; its third little-endian pointer at `0x6b7498` is `0x599b90`, the previously inspected transfer routine. This resolves that indirect call without assuming every similarly shaped virtual call has the same target.

On normal wrapper cleanup, the owner pointer is cleared, `0x593a60` runs with the session pointer in ECX, and `0x5f93e2` frees the allocation. For example: clear at `0x5ca62b`, cleanup call at `0x5ca637`, free call at `0x5ca63d`. `0x593a60` installs the HTTP-session vtable and closes its socket when valid before continuing cleanup. The corresponding second branch has the same clear/cleanup/free sequence at `0x5ca912` onward.

Status `0x7583` explicitly takes the same cleanup/free path and then recursively calls `0x5ca290` (`0x5ca527` or `0x5ca7b3`). That next invocation constructs a new session, whose base constructor zeroes session +0x184. Other shown statuses route toward alternate wrappers or the outer dispatch after cleanup; their complete meanings are unknown.

The outer routine `0x5caaa0` dispatches descriptor mode 1 or 2 to `0x5ca290`. Its broader UI entry mapping and all exceptional exits have not been reconstructed. Provider terminal-flow limitations and missing original signatures remain applicable; the assembly confirms pointer identity and allocation/cleanup on the cited paths.

## Counter lifetime

Earlier evidence established `0x5b8ea0` initializes session +0x184 to zero, and `0x594ec0` calls it. Reader `0x58db10` increments the field through its session pointer after status -12; counts below 3 take local rollback/reopen/reset/retry logic without constructing a new HTTP session in that immediate branch. The alternate reader `0x5785f0` also increments shared session state. These are per-session accumulated mismatch events, potentially originating from different connections.

Combining that evidence with the new wrapper trace establishes a concrete reset boundary: **destroying the old session and constructing a new session resets the count through initialization**. It does not establish an explicit in-place reset at every stop, success, pause, connection replacement or retry.

The descriptor +0x184 write in `0x599b90` is still a different object. Additional +0x184 occurrences in TLS fallback helpers `0x57e280` / `0x57e610` and an owner-related field in `0x597b60` must not be treated as writes to the session mismatch counter merely because their displacement matches.

## Evidence ledger

| Question / observation | Location | REA evidence ID | Status / limitation |
| --- | --- | --- | --- |
| HTTP constructor references | 0x594ec0 | `ev_c22fa93bdcb7420aaa141ab4efd6a1ceaab3dacf484fec0ebb303b1921883ced` | Four analyzed references; indirect references may be incomplete. |
| Wrapper construction and cleanup | 0x5ca290, queried via 0x5ca418 | `ev_8d757f713f6b7240cdf4f27073fd1740072939ca5b3577b80721a7c88106211d` | Decompiled control flow; original signatures incomplete. |
| Allocation and pointer identity | 0x5ca290 assembly | `ev_bd9d5dfd78b03bab8e1b20d6d3ee29108082c91cc8c59f6ba689abe226dc7c96` | Confirms size, ECX, owner pointer and cleanup calls. |
| Allocation helper | 0x5f93a8 | `ev_e8fdec68b3a4f04722734651ee4b96df7cf2cceac32dbe11ae6d22ae21d35165` | malloc plus failure callback. |
| Free helper | 0x5f93e2 | `ev_fe4ba4ea3bac3a55865a1363d102219433983a5fb4ad068d1f957d2a2b25921c` | Calls free. |
| HTTP cleanup | 0x593a60 | `ev_538cb16e7df38d4369c392d5a4ba183d3c56f2e9ed55436a388dfd69747b8e33` | Socket close and cleanup continuation; not every exceptional path recovered. |
| Constructor vtable | 0x594ec0 assembly | `ev_b8ac118f0286c5e878f5faf4c05fdb5c21d6f310f989a1074f0b462c41e19892` | Installs 0x6b7490, calls base constructor. |
| Virtual transfer target | 0x6b7490, 32-byte read | `ev_a38ce405667aa90bf4126f54c5a5c1dab59796bf482704ac2675d163ac0893d6` | Third pointer is 0x599b90; read extends beyond four table entries. |
| Outer dispatch | 0x5caaa0, queried via 0x5cb1c4 | `ev_1517456cf64dc74163d8266c0a702d7bb1116e2e882a88561b0173507197f953` | Modes 1/2 dispatch to HTTP wrapper; UI semantics unresolved. |
| Bounded displacement search | 0x550000 through 0x5d0000 | `ev_5f0c04e4014b2e0cc8ddcae3ce1da09d29a855a5bd52b4da7b8d5c2769041c68` | 33 byte-pattern matches; not exhaustive field-write/alias analysis. |

Earlier constructor and reader evidence is retained in the exported bundle and linked from [sample state](idm-sample-state.md) and [overlap validation](idm-overlap-validation.md).

## Reconstruction implications and remaining unknowns

For an independent Rust manager, keep transient recovery counters in the active transfer session and define their reset policy explicitly. Persist verified byte ranges and validators separately. This is a proposed design informed by the observed object lifetime; it does not promise compatibility with IDM storage or identical retry behavior.

Answered: new-session initialization; fresh ownership in the selected HTTP wrapper; normal cleanup; reconstruction on the traced outer retry. Partially answered: local mismatch retries retain access to the existing shared session in their immediate branch. Unresolved: exact user pause/resume dispatch, all exceptional lifetimes, every possible alias/bulk write to the counter, success-based resets, and any persistence outside the traced flow. No further manual tests were requested for this pass.

Evidence was exported before target closure to `analysis/idm/native/session-lifetime-evidence.json`; the corresponding snapshot retains 136 records. Extracts are `analysis/idm/native/reuse-*.txt`. [The index](../analysis/idm/native/session-lifetime-index.json) records hashes and verifies this report's evidence IDs against that bundle. The scheduler/retry-policy pass remains separate.
