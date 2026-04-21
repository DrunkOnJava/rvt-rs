# RE-17 synthesis — Global/ContentDocuments is the element index

**Date:** 2026-04-21
**Scope:** Inspect `Global/ContentDocuments`, the untouched stream flagged by RE-09 as a candidate for the missing element-location index.

## TL;DR

**Global/ContentDocuments contains a structured linked-list of element records.** Each record on 2024 is ~40 bytes with a u64 ID, two u32 count fields, markers, and back-pointers to previous IDs. Monotonic ID sequences (e.g. 40369 → 40370 → 40371) visible in the first 256 bytes of decompressed data.

This is *highly likely* the element sequence table that RE-09 predicted must exist but couldn't find in ElemTable. Confirming the exact record structure and id↔partition correlation is the remaining work.

## Size and structure

| File | Raw | Decomp |
|---|---:|---:|
| Einhoven 2023 | 30,961 B | 135,319 B |
| 2024 Core Interior | 243,084 B | 1,463,023 B |

Decompresses in a single gzip chunk per `inflate_at_auto` (prefix=8), no multi-chunk handling needed. Both files use the same stream name and layout.

## Observed record pattern (2024 Core Interior, bytes 0x85-0xf8)

Interpreting as 40-byte records starting at ~0x85:

```
0x85:  b1 9d 00 00 00 00 00 00    u64 = 0x9db1 = 40369     ← current element id
0x8d:  13 00 00 00                u32 = 19                 ← unknown count A
0x91:  13 00 00 00                u32 = 19                 ← unknown count B (same as A)
0x95:  ff ff ff ff                u32 = 0xFFFFFFFF marker
0x99:  b1 9d 00 00 00 00 00 00    u64 = 40369              ← id self-reference
0xa1:  ff ff ff ff ff ff ff ff    u64 = 0xFFFFFFFFFFFFFFFF marker ← no previous (first record)
0xa9:  00 00 00 00                u32 = 0
0xad:  b2 9d 00 00 00 00 00 00    u64 = 40370              ← next element id
0xb5:  13 00 00 00                u32 = 19
0xb9:  13 00 00 00                u32 = 19
0xbd:  ff ff ff ff                u32 = marker
0xc1:  b2 9d 00 00 00 00 00 00    u64 = 40370
0xc9:  b1 9d 00 00 00 00 00 00    u64 = 40369              ← prev_id back-pointer
0xd1:  00 00 00 00                u32 = 0
0xd5:  b3 9d 00 00 00 00 00 00    u64 = 40371
0xdd:  13 00 00 00                u32 = 19
0xe1:  13 00 00 00                u32 = 19
0xe5:  ff ff ff ff                u32 = marker
0xe9:  b3 9d 00 00 00 00 00 00    u64 = 40371
0xf1:  b2 9d 00 00 00 00 00 00    u64 = 40370              ← prev_id = 40370
```

**Record size: 40 bytes, repeating with back-pointer chain.**

## Record shape hypothesis (confidence 0.85)

```rust
#[repr(C, packed)]
struct ContentDocRecord {
    id:         u64,   // element id (monotonic, 64-bit)
    count_a:    u32,   // unknown (seen: 19)
    count_b:    u32,   // unknown, same as count_a in head
    marker:     u32,   // 0xFFFFFFFF
    id_again:   u64,   // repeats id (consistency check?)
    prev_id:    u64,   // previous record's id, or 0xFFFFFFFFFFFFFFFF on first
    trailing:   u32,   // 0 in observed records
}
// Total: 8 + 4 + 4 + 4 + 8 + 8 + 4 = 40 bytes ✓
```

## Cross-version observations

2023 Einhoven's ContentDocuments head (bytes 0x0-0x100) has different leading bytes:

```
00000000  65 03 ff ff ff ff 64 03 ff ff ff ff 1c 4b 44 55   |e.....d......KDU|
00000010  52 32 e6 40 87 ed 99 f5 15 00 a6 73 02 6e 00 00   |R2.@.......s.n..|
```

Leading u32 = 0x365 = 869, then ffff, then u32 = 0x364 = 868 — immediately suggests a pair of ids (869, 868). Very similar semantics to 2024 but **different record layout** (probably 32 bytes vs 40 bytes per record, since IDs on 2023 are u32 not u64).

## Key questions for RE-18+

**Q8** — ID space: on 2024 the first records show IDs 40369, 40370, 40371 — these exceed ElemTable's declared element_count (1411). Are ContentDocuments IDs in a different ID space, or do they include type/category/symbol entries that ElemTable excludes?

**Q9** — Does `id` correlate with a partition + offset? Try: for a ContentDocument id X, find chunks in `Partitions/*` whose u32[0] or u16 tag sequence contains X.

**Q10** — What does `count_a == count_b == 19` mean? Class-specific structural hint? Field count of an embedded record?

**Q11** — Record layout on 2023 is different (smaller IDs, probably u32 not u64, possibly 28-byte records). Full cross-version RE needed.

## Decision

**D6** — ContentDocuments is the RE breakthrough of this session. All subsequent RE work should build on this stream's record structure, not continue scanning partition chunks blindly.

**D7** — Stop the current RE loop at this synthesis. Follow-up work is substantial:
- Finalize record layout via structured probe (emit every record, check for layout drift)
- Cross-version: same probe on 2023 file, determine record size
- Correlate: ContentDocument IDs → ElemTable IDs → partition chunk offsets

## Artifacts

- `examples/probe_content_documents.rs` — dumps ContentDocuments head + ASCII scan
- `reports/element-framing/RE-17-synthesis.md` (this file)

## Revised task status

- RE-17 (#433) — **substantial progress made**; record structure sketched, layout confirmation pending (upgrade to completed once layout is byte-exact)
- Deferred: finalize 40-byte 2024 record schema, RE 2023 record schema, correlate IDs with partitions.
