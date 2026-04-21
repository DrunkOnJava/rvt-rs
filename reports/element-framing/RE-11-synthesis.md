# RE-11 synthesis — partition streams are a soup of sub-component tag records, not one-record-per-Wall

**Date:** 2026-04-21
**Scope:** Scan concatenated partition streams for u16-LE occurrences of every schema-tagged class, compute signal-over-random ratio per tag, identify which tagged classes actually appear as wire-format markers.

## TL;DR

Partition streams contain a soup of 13-16 tagged sub-component records,
not one record per architectural element. The most-frequent tags are
all geometric / analytical / host-object building blocks —
`AnalyticalLevelAssociationCell`, `AbsCurveGStep`, `HostObjAttr`,
`AbsDbViewPressureLossReport`, `AppearanceAsset`. A Wall or Floor on the
wire is not a single tagged record; it's a composition of several of
these subrecords.

This completes the methodological picture for partition decoding:
- We now know WHICH classes appear in partitions (13-16 classes, same
  set on every file modulo tag drift).
- We know SIGNAL STRENGTH (per-tag observed/expected ratios range from
  ~1 to ~600×).
- We do NOT yet know how subrecords are bounded (still open) or how
  they compose into element instances.

## Methodology

For each partition stream in the corpus:

1. Inflate all gzip chunks; concatenate into a single logical buffer.
2. For every u16-LE position `i`, read the value `v = le16(buf[i..i+2])`.
3. Count occurrences per `v`.
4. For every tagged class in the schema, compute:
   - `observed` = count of positions matching the class's tag
   - `expected` = total_positions / 65536 (uniform-random baseline)
   - `ratio` = observed / expected
5. Sort tagged classes by `log10(ratio)` descending.

Classes with ratio ≫ 1 are real. Classes with ratio ≈ 1 are byte-coincidence
noise. Classes with ratio ≪ 1 are absent from the stream.

## Data (4 partition streams, 2 corpus files)

**Einhoven 2023 Partitions/0** (3 MB decompressed, 3.02M u16 positions, expected 46/tag)

| Tag | Class | Observed | Ratio |
|---|---|---:|---:|
| 0x00ff | AnalyticalLevelAssociationCell | 21,906 | 478× |
| 0x0061 | AbsCurveGStep | 16,728 | 363× |
| 0x006b | HostObjAttr | 5,600 | 122× |
| 0x006d | AbsDbViewPressureLossReport | 5,212 | 113× |
| 0x0012 | ADTGridImportVocabulary | 4,594 | 100× |
| 0x0101 | AnalyticalLineAutoConnectData | 2,211 | 48× |
| 0x0062 | GeomStep | 1,996 | 43× |
| 0x013f | AnalyticalPanelPatternHelper | 1,824 | 40× |
| 0x0046 | ATFProvenanceBaseCell | 1,406 | 31× |
| 0x000d | A3PartyAImage | 1,313 | 29× |

**Census:** 13 strong (>10× random), 22 noise (0.5×-2×), 0 absent.

**Einhoven 2023 Partitions/5** (587 KB, 587K u16 positions, expected 9/tag)

| Tag | Class | Observed | Ratio |
|---|---|---:|---:|
| 0x00ff | AnalyticalLevelAssociationCell | 3,631 | 403× |
| 0x0061 | AbsCurveGStep | 3,430 | 381× |
| 0x006b | HostObjAttr | 1,453 | 161× |
| 0x006d | AbsDbViewPressureLossReport | 995 | 111× |
| 0x0062 | GeomStep | 615 | 68× |

**Census:** 11 strong, 16 noise, 9 absent.

**2024 Core Interior Partitions/46** (98 MB, 97.9M u16 positions, expected 1494/tag)

| Tag | Class | Observed | Ratio |
|---|---|---:|---:|
| 0x0100 | AnalyticalLevelAssociationCell | 877,098 | 587× |
| 0x0187 | AppearanceAsset | 113,870 | 76× |
| 0x0061 | AbsCurveGStep | 92,960 | 62× |
| 0x0012 | ADTGridImportVocabulary | 55,576 | 37× |
| 0x000d | A3PartyAImage | 54,622 | 37× |
| 0x006d | AbsDbViewPressureLossReport | 45,472 | 30× |
| 0x006b | HostObjAttr | 44,475 | 30× |
| 0x0102 | AnalyticalLineAutoConnectData | 28,964 | 19× |
| 0x0110 | AnalyticalLoadCaseParamElem | 26,222 | 18× |
| 0x0062 | GeomStep | 23,990 | 16× |

**Census:** 16 strong, 25 noise, 0 absent.

## Observations

**O1** — The *same* ~10 tag names dominate every partition on both 2023 and 2024 files:
`AnalyticalLevelAssociationCell`, `AbsCurveGStep`, `HostObjAttr`, `AbsDbViewPressureLossReport`,
`ADTGridImportVocabulary`, `AnalyticalLineAutoConnectData`, `GeomStep`,
`AppearanceAsset`, `ATFProvenanceBaseCell`, `A3PartyAImage`. The tag values drift
(0x00ff → 0x0100 for AnalyticalLevelAssociationCell) but the *class names* and
their relative frequencies are stable across the 2023→2024 transition. This is
strong evidence that these are real on-disk structures, not coincidence.

**O2** — None of the dominant classes are "architectural elements" in the
UI sense. `AnalyticalLevelAssociationCell`, `AbsCurveGStep`, `GeomStep`,
`HostObjAttr`, `AppearanceAsset` are all **sub-component records** — geometry
cells, host-object attributes, appearance references, provenance tags.

**O3** — A Wall on the wire must be a *composition* of subrecords, not a
single `Wall` record. The 80 tagged classes include architectural types like
`ArcWall` (0x0191) and `VWall` (0x0192) but these appear with lower frequency
— plausibly because there are fewer walls in the file than there are geometry
cells making up walls.

**O4** — `AnalyticalLevelAssociationCell` is the #1 tag by a large margin on
every partition. 21,906 hits in 3 MB = 1 per 137 bytes on average. If each
real occurrence represents ~100-150 bytes of record body, this class alone
could account for essentially all of the decompressed partition data. More
likely: it appears many times per element (indexing/linkage structure).

**O5** — `HostObjAttr` (tag 0x006b, stable across versions) matches exactly
what RE-09 F14 flagged as "legitimate high-frequency signal" (5600 on Einhoven
Partitions/0). Cross-verified: `scan_candidates` in Global/Latest found 6227
HostObjAttr candidates — within 10% of the 5600 partition hits. These numbers
are consistent with "one HostObjAttr record per architectural element with
~10-15% indexing overhead in Global/Latest."

**O6** — Some tag hits are inflated by common-byte patterns rather than real
occurrences. `0x00ff` (LE 0xff 0x00) and `0x0100` (LE 0x00 0x01) are both
naturally common byte sequences in aligned binary data. The 587× ratio for
`AnalyticalLevelAssociationCell` is likely real-signal-plus-padding-inflation.
`HostObjAttr` at `0x006b` (LE 0x6b 0x00, where 0x6b is ASCII 'k') is less
likely to be inflated and is a more trustworthy signal for "real element
records."

## Updated hypotheses

**H5'** (confidence 0.8, was 0.5) — Partition streams are *sequences of
subrecord instances*, each beginning with a u16 class tag. The tag at a given
byte position is the class discriminator for the record that starts there.
The record body length is class-specific (not uniform 40 bytes, not
length-prefixed at the tag level). Element instances (Wall, Floor, Door, etc.)
are composed of multiple subrecord instances, typically including one
`HostObjAttr`, several `AbsCurveGStep`s, one or more `AppearanceAsset`s, etc.

**H6'** (confidence 0.4) — Gzip chunk boundaries on 2024 may or may not
correlate with element boundaries. RE-09 F15 noted 2024 Partitions/46 has 925
chunks with 721 unique u0 values (78% unique), consistent with
one-chunk-per-element *or* one-chunk-per-subrecord-group. Needs direct
byte-offset inspection with tag context to decide.

**H8** (confidence 0.7, new) — There's a per-subrecord length field immediately
following each class tag. Finding its location + encoding would close the
"record envelope" gap. Heuristic: examine the 16 bytes following each
high-signal tag occurrence and look for patterns (consistent byte values,
length-like u32 that matches distance to next tag).

## Decisions

**D9** — RE-11 is now closed. Finding: partition streams contain
subrecords, not element records. The 10-16 dominant tag names are the
wire-format building blocks. Element instances are compositions.

**D10** — The *empirical tag set* for partition scanning is now known.
Subsequent tag-scan probes should filter to these 10-16 classes to cut
noise by 80-95%.

**D11** — Before attempting `walker::iter_partition_elements()` (RE-15),
RE-12 needs to happen: build empirical ElementId → (partition, chunk,
offset) map so we know how the 80-tag subrecords group into element
instances. Without that grouping, iterating "elements" from partitions
is meaningless — we'd be iterating subrecords.

**D12** — Record-length discovery becomes the next leverage point.
Pick one high-signal tag (say `HostObjAttr` 0x006b on Einhoven
Partitions/0, 5600 occurrences) and hex-dump the 64 bytes following
each occurrence. Look for: a length prefix, a consistent sentinel
byte distance, a fixed field block. Label this **RE-14.1** under the
existing RE-14 "Document Partitions/* wire format" task.

## Open questions

**Q10** — What's the body layout of a `HostObjAttr` record? This is the
best candidate for first real subrecord decode (122× signal on 2023, 30×
on 2024 — high enough that coincidence is unlikely; frequent enough that
we have thousands of samples to average across).

**Q11** — How does a partition chunk relate to element boundaries on 2024?
If chunks are one-per-element on 2024 (H6'), chunk bodies are directly
parseable as element subrecord sequences. If chunks are just gzip-framing,
we need another mechanism to find element boundaries.

**Q12** — The ElemTable declares N elements per file. If a file has N=1411
elements and Partitions/46 contains 877,098 `AnalyticalLevelAssociationCell`
occurrences, that's ~620 cells per element. Plausible for an analytical
model representation. But if Partitions/46 *also* has 44,475 `HostObjAttr`
occurrences, that's 31× the element count — which doesn't fit "one
HostObjAttr per element" unless 1411 is not the right "element count" and
the real element count is ~44,000. Needs cross-reference with ContentDocuments
ID count and the 40-byte record count in ElemTable itself.

## Artifacts produced

- `examples/probe_tag_signal_noise.rs` — 130-line signal/noise scanner
- `reports/element-framing/RE-11-synthesis.md` (this file)
- Updated hypotheses H5' (0.8), H6' (0.4 revised), H8 (0.7 new)

## Recommended next steps (prioritized)

1. **Record-length discovery (RE-14.1)** — pick `HostObjAttr` (0x006b,
   stable across versions, clean byte pattern, 5600 samples on Einhoven
   Partitions/0), hex-dump 64 B after every occurrence, look for
   distance-to-next-tag invariants.
2. **Tag-grouped histogram (RE-14.2)** — for each of the 10 dominant
   tags, compute the histogram of byte-distances to next-same-tag and
   next-any-tag. Fixed-width subrecords → sharp histogram peak.
   Variable-width with length prefix → bimodal.
3. **ElemTable count × tag count coherence (Q12)** — does the ElemTable
   declared count reconcile with the subrecord counts? If not, Q12's
   asymmetry is the clue.
4. **DEFER RE-15 (walker::iter_partition_elements)** — as stated in D11,
   without Q12's answer this API can't do useful work. Wait for
   RE-14.1/14.2 results before implementing.
