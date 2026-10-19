# IDM segmentation investigation

Date: 2026-10-18. Analysis only; no Rust implementation, builds or new download experiments.

Target SHA-256: `c7c80b420579d774f445b6b3f9282c0850760ef790373c0ece0f856a0a05d1d5` (`IDMan.exe`, x86).

## Existing runtime evidence

The preserved endpoint log contains nine geometrically matching pairs with exactly 64 bytes of overlap. Pairs were selected from bounded-range start records with the same URL and fixture version, excluding self-checks, with request start times less than one second apart. These are request relationships, not observed native worker identities.

Examples (inclusive HTTP endpoints):

| URL | Left request/end | Right request/start | Calculation |
| --- | --- | --- | --- |
| ranges.bin, size 1677721 | 18 / 962618 | 16 / 962555 | 962618 - 962555 + 1 = 64 |
| mutable.bin, delay 0.5 | 27 / 4476630 | 26 / 4476567 | 4476630 - 4476567 + 1 = 64 |
| browser.bin, size 1048576 | 47 / 658670 | 44 / 658607 | 658670 - 658607 + 1 = 64 |
| queue-a.bin, delay 0.1 | 128 / 8522990 | 126 / 8522927 | 8522990 - 8522927 + 1 = 64 |

Two successive requests for the initial ranges.bin test start at exact floor midpoints of the preceding request's interval:

| Earlier interval | Calculated floor midpoint | Next observed start |
| --- | --- | --- |
| [1605992, 1677720] | 1641856 | 1641856 (request 20) |
| [1641856, 1677720] | 1659788 | 1659788 (request 21) |

Requests 19–21 were aborted with zero recorded response bytes. Their geometry establishes requested ranges, not successful ownership or completion of those portions. The fixture payload is periodic; these observations cannot prove a universal allocator or arbitrary-data integrity.

Exact pairs, selection boundary and source-log SHA-256 are in [segmentation-runtime.json](../analysis/idm/native/segmentation-runtime.json).

## Request builder already recovered

`FUN_00589430` (`analysis/idm/native/0x589430.c`, retained evidence `ev_d7aca5af011cb3debb8cba2457f8315b6bec33bb7a174c06c1a7d9c924b5c566`) has several distinct range-building paths.

In one bounded-range path, it calls `FUN_0057ecf0`, then, under connection-context flags, caps the end against a 64-bit limit at connection offset `+0x58` minus one and may extend the calculated boundary by `0x40`. It stores the resulting end at `+0x68` and formats `Range: bytes=start-end`. An alternate flag uses a one-byte extension. Other paths use open-ended requests, fixed special-case intervals, and start adjustments. Therefore a 64-byte extension is conditional, not a property of every IDM request.

Provisional connection fields, based on use rather than original names:

| Offset | Observed role | Limit |
| --- | --- | --- |
| +0x58/+0x5c | A 64-bit upper limit, decremented before one HTTP-end calculation | Not yet established as the allocator's own segment end |
| +0x60/+0x64 | Logical start used by selection/insertion and request construction | Adjusted on some branches; not necessarily the wire start |
| +0x68/+0x6c | Calculated requested inclusive end | Some paths initially set -1 |
| +0x78/+0x7c | Inclusive logical end used by selection/insertion; fixed endpoint in other paths | Not identical to the final wire end when overlap is added |
| +0x80/+0x84 | Progress-like count subtracted by selection and folded into start during alternative attachment | No claim that it represents durable bytes |
| +0xa8 | Pointer to this connection's chunk-list node | Recovered pointer role; original type unavailable |
| +0x32a0/+0x32a4 | Actual start supplied to a selected formatted request | May differ from +0x60 because of branch-specific adjustments |

The +64 calculation and 64-byte request intersections corroborate each other. Whether this overlap is used for cross-segment verification requires tracing data comparison as well as endpoint selection.

## Native allocation trace

The transport routine `0x5abda0` calls the request builder and boundary helper. Its direct-caller list is empty, but xrefs include a data-table entry at `0x6ba3e4`; absence of a direct call is not absence of use. This path alone did not identify the allocator.

The string `Unknown error during ChunksList::NeedNewConnection()` at `0x6b1014` leads to catch handler `0x559ab9`. Nearby eligibility routine `0x559a20` calls selector `0x5586d0`; the same selector is called by insertion routine `0x55efc0`. The error string supplies a naming lead, not original debug symbols for all these functions.

### Candidate selection and eligibility

`0x5586d0` walks the list rooted at object offset `+0x88`, following node offset `+0x58`. In its ordinary byte-count branch, it skips entries whose node field `+0x14` is nonzero and keeps the node with the largest positive adjusted remaining amount. Equal amounts do not replace the previously selected node.

Using descriptive labels for recovered fields:

> adjusted remaining = inclusive end − progress − start + 1 − allowance

Here connection `+0x78/+0x7c` supplies the inclusive end, `+0x80/+0x84` supplies a progress-like 64-bit count, and `+0x60/+0x64` supplies the start. The allowance is the 32-bit product of connection `+0x98` and the selector's second argument. Its units are not established; it must not be described as a speed penalty without further evidence. Modes 4/5 and another object-state branch use different selection calculations.

Gate `0x559a20` immediately permits one special state when object `+0xcc` is nonzero. Otherwise it asks the selector for a candidate and checks its amount against `2 × object_u64(+0xc0) + 3`. The origin/default of that configurable-looking field is still unknown.

### Splitting and donor update

`0x55efc0` contains the diagnostic `Insert next chunk in list`. In a common branch it selects a candidate, allocates a new list node, carries the donor's inclusive end into the new connection, and calculates:

> new start = donor end − floor(adjusted remaining / 2)

It normally shortens the donor's logical end to `new start − 1`, with a guard that can cancel that endpoint update. It then inserts the new node after the donor, links the new connection to that node and initializes its saved start fields. This is static evidence for splitting an existing remaining interval rather than preallocating equal fixed-size pieces once.

The ordinary insertion path declines amounts below `0x5b43` (23,363) when its second argument is zero and another state field is clear. A nonzero third argument adds another size guard and alignment arithmetic. These constants describe this branch, not a universal minimum request size; later observed requests can be smaller or aborted.

There are meaningful exceptions:

- `0x558560` explicitly logs `FindChunkWithZipCentralDir`; an eligible branch targets the interval containing a stored ZIP-directory position instead of simply selecting the largest remaining interval.
- An early first-node branch may cap the midpoint-derived new start at `old start + 10 × (progress + 11680)`. This biases where an initial extra connection begins.
- Another restricted first-node branch subtracts 11,680 or 23,360 from the donor end instead of using the ordinary half calculation.
- Other mode/state branches use different calculations. Their complete protocol/configuration meanings have not been recovered.

The assembly for caller `0x581e90`, at `0x582001–0x582011`, pushes three zero arguments followed by the connection pointer before calling `0x55efc0`. Its call to the eligibility gate also passes zero. For this insertion call, the optional allowance/alignment argument is therefore zero. Ghidra's pseudocode omits these arguments at the call site, so the assembly was checked rather than trusting that recovered signature. The enclosing caller has its own mode/state guards; this is not proof that every HTTP path takes it.

### Alternative connection attachment

The same caller can invoke `0x5631f0` instead of inserting a new split. That routine logs `Add alternative connection`, copies the existing interval's start/end to another connection, clears its progress fields and links alternative-node pointers. If its preceding file-size condition is positive, another branch first advances the old connection start by its progress, clears that progress and updates list entries.

This distinguishes an alternative-connection attachment from creating a new midpoint split. Which scheduler conditions select an alternative connection is not fully established; it is not yet a complete retry or work-stealing policy.

### Neighbor boundary and overlap

Helper `0x57ecf0` walks forward from connection node `+0xa8` through node `+0x58`, skipping empty entries and connections whose progress-like count is below 51 on the inspected path. It uses another connection's start minus one when calculating a boundary, with fixed-end/state exceptions. The request builder's conditional +64 can therefore produce a wire end at the next start +63, explaining the measured 64-byte intersections.

This corroborates overlap construction. It does **not** locate the reader's byte-comparison routine or prove why every overlap is requested.

### Persistent topology, observed without reading user records

The insertion routine conditionally writes a 16-byte registry record through its supplied handle. The decompiled stack layout and diagnostic `ID, startPos, nextID` support a record containing a 32-bit ID, 64-bit start and 32-bit successor ID. It also updates the donor's successor record before changing in-memory links. Alternative attachment has a related record-name path ending in `a`.

This recovers a piece of the stored chunk topology. The root key, complete checkpoint format, crash consistency and recovery reader remain unresolved. No live registry download records were inspected or modified in this pass.

## Evidence references

| Role | Address | REA evidence ID |
| --- | --- | --- |
| Transport caller | 0x5abda0 | `ev_b0f85fe222b1bab71685ee8649b3e567fd75f73bbd75894b1fe1e190144c6ff3` |
| Neighbor-boundary helper | 0x57ecf0 | `ev_252e0a6098684ab0e16689d65406ac724b62b9b39338e9beeecbda8d2a6a30d1` |
| Candidate selector | 0x5586d0 | `ev_1ba97414bc44ff17d85566ad9c06e0517c58ccb3679117be3ad2370e0004896d` |
| Eligibility gate | 0x559a20 | `ev_6b424294d78c6785529b923289141a3d6f9beb162c15e0c09e9c5bc6e5da7360` |
| Chunk insertion and donor update | 0x55efc0 | `ev_00501b7dfa51b1321fea5ded4f1ea1ca6783bffa2ab08ee0f9c55b8c666f2100` |
| ZIP-directory candidate | 0x558560 | `ev_e306afb3db7b7b5b61a8c13d1c6666f15f8fe26ce4d394bdf435c5f4b98de381` |
| Enclosing caller, assembly | 0x581e90 | `ev_4ff6354303208996da4d0a3f56796440027917d7f3e9940d38423a4ba264e04c` |
| Alternative attachment | 0x5631f0 | `ev_4f94d82ad7943011203d6ea024f0019d859db07e84d4c09456101ea9646f7186` |

Decompiled text and the checked assembly are saved as `analysis/idm/native/segment-*.txt`. [The follow-up index](../analysis/idm/native/segmentation-index.json) records their identities. Full REA records and limitations are preserved in `segmentation-evidence.json`, with a separately saved snapshot. The earlier evidence was retained rather than overwritten. No metadata annotations or target-binary changes were made.

## Remaining questions

Follow-up: [overlap validation and recovery](idm-overlap-validation.md) identifies the buffered resume-sample comparator, neighboring-chunk comparison and their separate mismatch statuses. The questions below retain the segmentation pass's original boundary; expected-sample initialization and complete scheduler policy are still unresolved.

- Which scheduler paths call these routines, and how do global/per-origin caps constrain them?
- What initializes object +0xc0 and the optional allowance field, and which modes activate the special branches?
- Which conditions favor alternative attachment versus a new split, and how are canceled donor updates reconciled?
- Which reader compares overlapping bytes, and what exact conditions trigger restart versus single-connection fallback?
- Which branch-specific limits are protocol/media exceptions rather than the general HTTP allocator?
