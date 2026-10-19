# IDM comparison samples and mismatch-counter initialization

Date: 2026-10-18. Static follow-up on x86 IDMan.exe, SHA-256 `c7c80b420579d774f445b6b3f9282c0850760ef790373c0ece0f856a0a05d1d5`. No implementation code, builds, new runtime tests or live registry changes.

## Findings

- The expected resume sample is loaded from the tail of the local chunk file, initially requesting **64 bytes**.
- On the successful file-size/read path, its length is `min(64, local file length)`.
- A nonempty successful load arms the comparison flags and resets the received-sample count to zero.
- The session mismatch counter starts at zero in the session constructor.
- Another object has a write at the same +0x184 offset; that write is not a reset of the session counter.

## Expected sample loading

`0x5657b0` sets connection +0x194 to `0x40`, then invokes file helper `0x56f070` with the filename, buffer at connection +0x154 and a pointer to the requested length. Assembly confirms that the helper's file-I/O context is connection +0x10; the sample buffer and length remain connection fields.

The helper closes/resets existing file state, opens the named file with read/write access and OPEN_ALWAYS semantics, queries its size, clamps the requested amount when the file is shorter, seeks to `file size - sample length`, and reads exactly that amount. On a successful nonempty read it seeks back to end-of-file and leaves the file open.

This is not a read-only verifier: the inspected code can create a missing file. It was not executed during this pass. Failed/short reads close the file state and return an error. The zero-byte path succeeds without ReadFile. The clamp is established for a successful size query; a failed query does not establish a valid sample length.

Derived examples, not new experiments:

| Local file length | Sample length | Read offset |
| --- | --- | --- |
| 0 | 0 | 0; no sample read |
| 23 | 23 | 0 |
| 64 | 64 | 0 |
| 1000 | 64 | 936 |

The preparation routine takes its positive-success branch only when the helper succeeds and the resulting length is nonzero. It sets connection flags +0xde and +0xdf to 1 and clears +0x198. This routine does not clear all pre-existing flags on failure; complete caller cleanup on error remains unresolved.

## From local bytes to a re-fetched comparison

| Connection offset | Role in this path |
| --- | --- |
| +0x154 | Expected sample read from the local file's tail |
| +0x194 | Loaded sample length, initially requested as 64 |
| +0x114 | Buffer receiving re-fetched bytes |
| +0x198 | Accumulated received-sample count, initially zero |
| +0xde / +0xdf | Flags enabling/selecting the buffered comparison path |

The [previous trace](idm-overlap-validation.md) showed the request builder moving its start backward by +0x194 under flag +0xdf. The receive pipeline calculates the remaining check-stage amount using `+0x194 - +0x198`, accumulates re-fetched bytes at +0x114, and invokes `0x565940` when that stage completes. The comparator checks +0x198 bytes against +0x154.

This connects the backward request, local expected bytes and comparison. The 64-byte maximum is established for this loader, not for every content-recognition or media path.

One saved path in reader `0x58db10` demonstrates preparation during continuation: it stops a connection, folds progress into its logical start, clears progress, updates state, closes the file state and calls `0x5657b0` with a chunk-node filename. Other callers include `0x55cd40`, `0x55e4b0`, `0x572cd0`, `0x5820c0` and `0x584060`; those callers were not all expanded here. No actual sample bytes or private download files were inspected.

## Counter initialization and different objects at the same offset

Readers previously identified a mismatch counter at session +0x184 through a connection's session pointer. They increment it on internal status -12 and compare the result with 3.

Constructor `0x5b8ea0`, labeled with `Session::vftable` by Ghidra, initializes its own +0x184 to zero. Assembly confirms `XOR EDI, EDI` at `0x5b8eee` and the store from EDI at `0x5b9044`.

Derived constructor `0x594ec0` calls that base constructor, installs `HTTPSession::vftable` and stores the supplied download descriptor at session +0x0c. This establishes a construction-time zero for the session counter used by the recovered HTTP-session readers.

A separate write in `0x599b90` clears descriptor +0x184 and +0x180 when descriptor flag +0x34a is clear. Its base is the pointer loaded from session +0x0c, not the session itself. Matching numeric offsets in these two objects do not establish matching fields. This write is therefore not evidence that the session mismatch counter resets on every transfer start. The descriptor fields' exact meanings remain unresolved.

**Lifetime boundary:** initialization on construction is established. A reset on each resume, retry, successful comparison or worker creation is not. Persistence across object reuse or reconstruction is also not established. The earlier threshold must still be described as shared session state, not three failed user actions.

## Evidence and coverage

| Observation | Address | REA evidence ID |
| --- | --- | --- |
| Sample preparation | 0x5657b0 | `ev_966717f3d8e7993da55719354a255c9f3a0c64ae7932196264bb8a410e53f0a3` |
| Preparation assembly / I-O subobject | 0x5657b0 | `ev_d5c46fabc15f622c12a5ca8066730f1d12cd4a700d392c5089340e14069832d5` |
| Tail read, clamp and seeks | 0x56f070 | `ev_fc236762976b0b3f9bba5e320d3e751ba5dd7736ccdb9137b064b32498d80583` |
| Session initialization | 0x5b8ea0 | `ev_8bc02253f505426fb406ad025136bceb37a7d8fbd976ae87c49d562abf8d5033` |
| Constructor assembly | 0x5b8ea0 | `ev_e2c6d5d8303b89eb7d9a3e08cf1d439159b0f21e0655086264eb7fcb5e0bed36` |
| Derived HTTP-session construction | 0x594ec0 | `ev_8104c74a4e1eca90a93546e08744fd1b5c0898c5bceedc463d57d892e47a30e7` |
| Separate descriptor resets | 0x599b90 | `ev_078ef60602434b531e6544db06d40bef570025c48e638e9e9fa50b1b9a038b88` |

The selected sample-buffer LEA pattern search covered initialized analyzed bytes from `0x550000` through `0x5c0000`, locating preparation and comparison references. An immediate-zero store pattern at +0x184 found none; a register-store pattern then found the two different-object writes above. These are bounded opcode searches, not exhaustive field-write analysis across all encodings, aliases, bulk initialization or other code regions.

Search evidence: `ev_d53a324527e8f3c850bc5fdae3eeffa60e2b8dd061e3b1bd32e734d7c4056e7f`, `ev_a8723f424923cdb106f2bb01522e8c35da51715656a8ad4d3ae3b0d2cf04ceb6`, `ev_0a9cad69dc69c88b9a8c4ca719632777d0bbb33c79915b40ca3c867732ca2809`.

Extracts are `analysis/idm/native/sample-*.txt`. Complete retained records are in `sample-state-evidence.json`, with a snapshot saved before target closure. [The index](../analysis/idm/native/sample-state-index.json) records file identities. Earlier evidence is preserved. Original types/names and complete calling conventions remain limited.

Follow-up: [session lifetime](idm-session-lifetime.md) now traces fresh HTTP-session allocation, normal destruction and reconstruction on one outer retry path. Exact UI pause/resume dispatch, sample-flag cleanup on error and other possible counter writes remain explicit gaps. Scheduler mode and connection-cap behavior remain a separate pass.
