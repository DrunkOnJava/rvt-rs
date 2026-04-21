# RE-09 synthesis — partition-chunk header hypothesis refuted + tag-scan signal analysis

**Date:** 2026-04-21
**Scope:** Probe partition-chunk header structure (RE-09), scan chunks for schema-tag occurrences (RE-11 preliminary), enumerate tagged schema classes.

## TL;DR

The 16-byte partition-chunk header hypothesis from RE-01 was **wrong**. Gzip chunk boundaries are compression-artificial, not semantic. Element records aren't aligned to chunks, so the "header" bytes I saw were actually random bytes of element payload that happened to land at chunk boundaries.

Concatenating all gzip chunks of a partition into one logical buffer and scanning for u16-LE schema tags yields *many* hits (68,546 in 3 MB on Einhoven's Partitions/0) but is too noisy — most are coincidental byte patterns. The u16-tag-scan approach needs a stronger envelope (length prefix, magic bytes) to distinguish real element-record starts from random binary data.

## Verified facts (new since RE-01)

**F10** — Schema has 405 classes but only **80 carry `tag` values**. The other 325 are tagless — abstract parents, type-only definitions, subclass relationships. *Source:* `examples/probe_tagged_classes.rs`.

**F11** — **Generic `Wall`, `Floor`, `Door`, `Window`, `Column`, `Beam`, `Roof`, `Ceiling`, `Level`, `Grid`, `FamilyInstance`, `Room` all lack tags.** Their tagged instances appear under subtype names like `ArcWall` (0x0191), `VWall` (0x0192), `WallCGDriver` (0x0197), `ArcWallRectOpening` (0x019c). *Source:* probe_tagged_classes output.

**F12** — The "16-byte chunk header" hypothesis from RE-01 is refuted. u32[0] is NOT monotonically increasing across chunks in one partition (21/33 unique on Partitions/0). u32[2] does NOT equal the chunk body length (differences of thousands to millions of bytes). *Source:* `probe_partition_chunk_header.rs` output, specifically:
```
u0 monotonic: false, unique u0 values: 21/33  (Partitions/0)
u0 monotonic: false, unique u0 values:  4/10  (Partitions/5)
```
The 4 unique u0 values across 10 chunks of Partitions/5 is a strong indicator: consecutive chunks share the same u0 because they're mid-element when the gzip boundary hits. Gzip chunk starts don't imply logical record starts.

**F13** — Concatenating all chunks and scanning the resulting buffer for u16 LE schema tags:

| Partition | Decomp | Hits | Hits/KB |
|---|---:|---:|---:|
| Einhoven Partitions/0 | 3 MB | 68,546 | 22 |
| Einhoven Partitions/5 | 587 KB | 12,224 | 21 |
| 2024 Partitions/46 | 98 MB | 1,802,270 | 18 |

Tag hits per KB are dense enough to indicate that u16-value coincidences dominate the signal. The most-frequent tags are classes with small tag numbers (0x00ff = AnalyticalLevelAssociationCell, 0x0061 = AbsCurveGStep) — i.e., bytes likely to appear randomly in geometric data.

**F14** — **HostObjAttr (0x006b) is a legitimate high-frequency signal.** 5600 hits in Einhoven Partitions/0, 1453 in Partitions/5. This aligns with the 6227-candidate count from `scan_candidates` in Global/Latest and suggests HostObjAttr genuinely appears in the element payload as a real class marker, not just coincidence.

**F15** — **Cross-version partition-chunk structure differs substantially.** Einhoven 2023 Partitions/0: 33 chunks, 21 unique u0. 2024 Core Interior Partitions/46: 925 chunks, 721 unique u0 (78% unique). On 2024, u0 values are quasi-monotonic (1810, 64397, 81127, 81549, …) for most chunks after the first, suggesting 2024 moved closer to one-chunk-per-element. *Source:* probe_partition_chunk_header.rs output.

## Updated hypotheses

**H5 (confidence 0.5, supersedes H1)** — Element data lives in Partitions/*, but finding element boundaries requires either:
(a) An ElemTable side-index that maps ElementId → (partition_num, byte_offset), not yet located;
(b) A per-record envelope (length prefix or magic bytes) preceding each element within the concatenated partition buffer;
(c) Category-specific framing where different partition numbers hold different element types with different wire formats.

**H6 (confidence 0.3)** — Revit 2024 changed the partition layout to one-chunk-per-element. The gzip-chunk framing IS the record boundary on 2024 files. Would explain the 925 chunks on 2024 Partitions/46 matching roughly the 1411 total elements (with some multi-chunk elements and some no-element partitions).

**H7 (confidence 0.7)** — Tagless classes (Wall, Floor, Door) use their *parent-in-schema-tree* tag on the wire. Need to walk class `parent` chain in the schema to find the first ancestor with a tag, then scan for that ancestor's tag as a proxy for the child.

## Open questions

**Q5** — What is the per-element envelope that separates one element from the next inside a partition? Fixed magic bytes? Length prefix? Category-specific separators?

**Q6** — Is there a separate per-partition or per-file index that maps ElementId → location that I haven't found yet? `Global/ContentDocuments` (30 KB on 2023, 243 KB on 2024) is untouched — might be relevant.

**Q7** — Why does 2024 have 925 chunks in Partitions/46 when Einhoven has 33 chunks in Partitions/0? Purely file-size difference, or real structural difference?

## Decisions

**D3** — RE-09's narrow "16-byte header" hypothesis is refuted; the task is expanded in scope. The old task remains "completed" in the sense that the question was investigated, but the result is a negative finding.

**D4** — Before continuing with RE-10/RE-11, investigate `Global/ContentDocuments` (Q6). This is a stream I haven't opened yet that might hold the missing element location index.

**D5** — For RE-09's target of "pin the chunk header layout," the empirical answer is: **gzip chunks have no semantic chunk header**. The first 16 bytes I examined are the tail of logical records from the previous chunk or the start of arbitrary payload.

## Artifacts produced

- `examples/probe_partition_chunk_header.rs` — prints first 16 B of each chunk as u32[0..4]
- `examples/probe_tagged_classes.rs` — lists all 80 tagged schema classes
- `examples/probe_chunk_body_tags.rs` — scans chunk first-256B for interesting class tags
- `examples/probe_partition_tag_density.rs` — concatenates all chunks of a partition and counts tag frequencies
- `reports/element-framing/RE-09-synthesis.md` (this file)

## Recommended next steps

Prioritized:

1. **Inspect `Global/ContentDocuments`** — 30KB on 2023, 243KB on 2024, previously untouched. May contain the element-location index RE-02 looked for in ElemTable unsuccessfully.

2. **Walk class `parent` chains** — build a tagged-ancestor map so untagged Wall/Floor/Door can be located via their parent's tag.

3. **Spend 2024's monotonic u0 signal** — on 2024 Core Interior Partitions/46, u32[0] values like 1810, 64397, 81127, 81549, … might be ElementId values. Cross-reference against the 1411 declared ElemTable IDs to test.

4. **Look at non-first partition-chunk content** — the strongly-hypothesized "chunk header" might be an individual chunk's compression artifact. Try parsing chunk bodies as proper [length][record] streams.
