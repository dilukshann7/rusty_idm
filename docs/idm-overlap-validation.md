# IDM overlap validation and mismatch recovery

Date: 2026-10-18. Static follow-up on the same x86 IDMan.exe, SHA-256 `c7c80b420579d774f445b6b3f9282c0850760ef790373c0ece0f856a0a05d1d5`. No implementation code, builds, application launches, new downloads, or live registry changes were made.

## Result

Follow-up: [sample loading and counter initialization](idm-sample-state.md) establishes a tail sample of up to 64 bytes for the identified resume path and a session-construction zero. Counter lifetime during session reuse and complete error-path cleanup remain unresolved.

Two distinct byte-comparison paths are now identified:

| Check | Routine | Data compared | Mismatch status |
| --- | --- | --- | --- |
| Re-fetched connection/resume sample | `0x565940` | Buffered bytes at connection +0x114 versus saved bytes at +0x154, for a count at +0x198 | `-12` / `0xfffffff4` |
| Boundary against another chunk | `0x567ee0` | Buffered bytes at +0x3ac versus bytes read from a neighboring chunk file, with a fallback filename path | `-11` / `0xfffffff5` on one successful-write recovery branch |

The shared receive pipeline `0x5690d0` dispatches the checks according to connection flags. Reader `0x58db10` has separate branches for these negative status values. Another reader, `0x5785f0`, has corresponding recovery branches. These are internal statuses, not HTTP status codes or Windows error numbers.

## 1. Re-fetched sample comparison

The previously recovered request builder can subtract a stored amount at connection +0x194 from a request's start when flag +0xdf is set. It can therefore request bytes preceding the next logical position.

In the shared receive pipeline, flag +0xdf selects an accumulation path: received bytes are placed at +0x114, accumulation is tracked at +0x198, and the check is invoked when that receive stage completes. Routine `0x565940` compares the buffer against +0x154 using four-byte comparisons and a trailing one-to-three-byte check. Equality sets its output status to zero and returns success; inequality logs `Check w_e_of_cn failed!`, writes `-12` to its output argument and returns failure.

The mismatch-status write and buffer offsets were checked against assembly, not only pseudocode. The current evidence does not establish where the expected +0x154 sample is initially populated or the universal sample length. The two buffer bases are 64 bytes apart, but this alone does not prove that every check compares exactly 64 bytes.

### Recovery selected by a reader

For `-12`, `0x58db10` stops the affected connection and, when its download-state object exists, increments a counter at that object's +0x184.

- If the updated counter is below 3, a branch closes/resets file handling, adjusts accounting and start-related fields, reopens/reinitializes the chunk, clears progress and requests another transfer stage. It does not immediately display the restart warning.
- At or above 3, the reader calls `0x594ad0` with 200, which under its state conditions sets an internal mode to 2 and stops/clears secondary connection slots. If another primary-chunk/start-position condition holds, it calls `0x594df0` to stop active connections and clear/recreate download state.
- It then starts the warning callback with argument 2, selecting the message that says downloading restarted from the beginning.

The larger reset helper `0x594df0` can separately ask whether to download from the beginning when its +0x148 state is clear. Declining that separate prompt sets a stop flag and returns before clearing the download. This must not be confused with the corruption warning's later question about opening a help page.

Do not describe this as three failed user resumes. The counter belongs to shared download state; multiple connections or receive events can reach it. Its initialization/reset lifetime and the complete meaning of every guard have not been traced. The warning's wording also does not replace tracing the state conditions: the larger reset helper is called only under the observed primary/start condition.

`0x5785f0` independently shows an analogous `-12` counter check and reset/message selection. Its virtual recovery calls and object layout have not been fully typed.

## 2. Comparison against a neighboring chunk file

Routine `0x567ee0` follows the current chunk node at connection +0xa8. When a successor at node +0x58 exists, it obtains that node's filename at +0x10. Otherwise it uses a fallback filename field on the current node. It opens the file read-only and reads up to the buffered count at connection +0x3f0, then compares the returned bytes against the connection buffer at +0x3ac.

Its comparisons also use four-byte units plus trailing bytes. On a mismatch it logs `Check w_st_o_n_cn failed!`. The routine attempts to write its buffered data into its own output path and updates accounting. A successful-write branch sets output status `-11`. A file-write failure has a separate `-2` path. If opening the comparison file fails or the read yields no bytes, the shown code does not take that mismatch branch; the larger retry/access-error policy remains unresolved.

The shared pipeline chooses this accumulation/check path under flag +0xdd. Combined with the [segmentation trace](idm-segmentation.md), this establishes actual comparison against neighboring chunk data, rather than treating the measured overlap as unexplained redundant traffic. It does not prove that every overlapping request runs this exact check or that a complete file is compared.

### One-connection fallback

Reader `0x58db10` handles `-11` by calling `0x594920(1)` before starting the warning callback with argument 1. That recovery helper sets an internal mode and, on its shown paths, stops secondary connection slots and clears their activity fields. Depending on primary-connection state, it preserves/rebuilds selected progress boundaries or clears/recreates chunk state. Thus fallback need not mean that all already received bytes are always discarded.

Reader `0x5785f0` handles the same status through `0x57af40`, then selects warning argument 1. That helper also manipulates primary/chunk state. Exact connection-limit precedence and future worker creation still require scheduler tracing; mode value 2 is not itself a recovered numeric connection limit.

## 3. The warning is a notification/help choice

The translation initializer at `0x4ec5c0` associates:

- Global `0x7676f0` with the one-connection corruption warning.
- Global `0x7676f4` with the restart-from-beginning corruption warning.

Callback `0x57f520` selects the first for argument 1 and the second otherwise. It reads a persisted notification/help counter and may suppress the dialog. If the user answers Yes, it opens IDM's data-corruption support page and increments that counter. This callback does not perform the download reset. Answering No declines the help page; it is not a choice to reject the recovery already selected by the reader.

The callback is passed to `AfxBeginThread`, explaining why a direct-caller query was empty even though actual code references exist. The string initializer was useful for identifying the globals, but was not mistaken for the corruption detector.

## 4. Separate checks that should not be conflated

Routine `0x565ab0`, reached by a query inside its body at `0x565bc0`, uses `0x5652d0` to compare incoming data with stored file-start data. Its diagnostics include `Strart data for this chunk is matched with start of the file!!!`, and it can produce statuses such as -7/-8/-9/-10 depending on state. This is another response/content recognition path. It is not the `-11` or `-12` comparator identified above.

These routines compare byte samples; the inspected paths are not full-file cryptographic hash verification. The earlier investigation's 11 SHA-256 checks were external checks of the test outputs, not evidence that IDM performs those hashes internally.

## 5. Relation to the previous runtime test

The user previously paused mutable.bin, the fixture changed both its bytes and its validators, and the user reported the restart corruption warning. The final output matched the replacement fixture. The newly recovered `-12` recovery/message path is consistent with that observation.

No call stack, internal counter or comparator buffer was captured during that test. Therefore the past run is not retrospectively labeled a proven execution of this exact routine, and changing both validators and bytes still prevents isolating the original triggering condition. No additional runtime experiment was performed in this follow-up.

## Evidence and limitations

| Evidence | Routine or observation | REA ID |
| --- | --- | --- |
| Resume/re-fetched sample comparator | 0x565940 | `ev_269b679697d4a2144e68c149dd7689f4031fb6753b166e54bb68d618b34d570f` |
| Its assembly | 0x565940 | `ev_8c26e3cc3ac62f5233aa35c19ffee0b055c15779a32073b9d9b90083003f6127` |
| Neighbor-file comparator | 0x567ee0 | `ev_ae8cc9900f458e1a0df2f02756e92cb57d625db43518d3d4d19024c48fcde0f1` |
| Its assembly/status write | 0x567ee0 | `ev_285c79f52c0bf141d1813f2a8ab00dfc097236fa322700a26a20074c04c0d2e2` |
| Shared receive pipeline | 0x5690d0 | `ev_414b493a0be256b497ba16d9654dfdb854c1213d0bcec0a29ff025bb9793f359` |
| Reader dispatch | 0x58db10 | `ev_e06de84a5f46ecce6e40a2f6363d56e297f393a1836e08e18dd823b473b20a53` |
| Additional reader | 0x5785f0 | `ev_75ee0d47e580cb055c9c58effa934348b442fa77a722117eb66cc5f942fff4b8` |
| One-connection recovery helper | 0x594920 | `ev_8d34d64a05a7969f8512a51b19c2c03aa80108f657fa3b60cef9e59077e6d05a` |
| Conditional larger reset | 0x594df0 | `ev_d363907f348bb2935eab4d8d2ef0af9ef9d4f0b073449e45b5a196233124ee19` |
| Warning callback | 0x57f520 | `ev_a15461e5207ed46970bcd7a3b183d9d335c3d69268862602a2a9feeca184b467` |
| Warning/global assignment assembly | 0x4ec5c0 | `ev_f32f9eae7f38203a9c64c51c127bd5a5103da5614014cc34de4e29ec12e1f9d0` |

Saved extracts are `analysis/idm/native/overlap-*.txt`; complete retained records and limitations are in `overlap-evidence.json`, with `overlap-snapshot.json` saved before closing the target. [The follow-up index](../analysis/idm/native/overlap-index.json) records their file hashes. Prior evidence and snapshots were preserved.

Original symbols/types are unavailable. Ghidra reconstructs some shared helpers as void even where callers use a result; return-value/cleanup conventions are not fully recovered. The small comparators' output-status writes were independently checked in assembly, but the complete ABI and every protocol/virtual-call variant were not reconstructed. No target patches or metadata annotations were applied.

Remaining work: trace expected-sample loading and lengths; initialize/reset the mismatch counter; recover complete file-access-failure handling; confirm protocol variants and mode guards; and connect recovery mode to global/per-origin scheduler caps. A future controlled experiment should change only sample bytes or only validators and capture the internal path separately.
