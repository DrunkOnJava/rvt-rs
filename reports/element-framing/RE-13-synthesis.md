# RE-13 synthesis — ArcWall decoder is Revit-2023-only, 2024 needs re-RE

**Date:** 2026-04-21
**Scope:** Verify the ArcWall decoder from RE-14.3 generalizes to Revit 2024 (2024_Core_Interior.rvt). Cross-version drift analysis on tag value + record envelope.

## TL;DR

**The 2023 ArcWall decoder does NOT generalize to 2024.** Two independent changes between releases:

1. **Tag drifted**: Revit 2023 ArcWall = `0x0191`; Revit 2024 ArcWall = `0x019c`.
2. **Record envelope changed**: 919 filtered 2024 ArcWall occurrences on
   Partitions/46 have ZERO records with variant marker `0x07fa`. The 2023
   "standard record" format does not appear at all in 2024.

The decoder in `src/arc_wall_record.rs` has scope "Revit 2023" only. DEC-05
pass on Einhoven (2023) is real; the same decoder yields zero IFCWALL on
2024_Core_Interior.

## Evidence

### Tag drift

Probe `probe_arcwall_2024.rs` resolves ArcWall's tag from the 2024
schema's `Formats/Latest` parse:

```
=== 2024_Core_Interior.rvt — ArcWall tag in 2024 schema: 0x019c ===
```

2023 ArcWall = 0x0191 → 2024 ArcWall = 0x019c. 0x019c - 0x0191 = 11, consistent
with Autodesk inserting 11 new classes alphabetically between `ArcTopology` and
`ArcWall` between the 2023 and 2024 schemas.

### Record envelope drift

With the correct tag 0x019c applied to the record-prefix filter (bytes +2/+3 == 0x00/0x00):

| File | Filtered count | Has variant 0x07fa? |
|---|---:|---|
| Einhoven Partitions/5 (2023) | 32 (before filter: same) | 26 records (100%) |
| 2024 Core Interior Partitions/46 | 919 | **0 records** |

The variant marker distribution on 2024 is completely different:

| Variant at +0x10 | 2024 record count |
|---:|---:|
| 0x0000 | 259 |
| 0xffff | 97 |
| 0x0003 | 80 |
| 0x000d | 35 |
| 0x053b | 32 |
| 0x004e | 24 |
| 0xfe68 | 24 |
| 0xaaaa | 21 |
| 0x1083 | 17 |
| 0xc000 | 17 |

None of these are 0x07fa. The 2023 record envelope (`variant=0x07fa`,
`fixed_header_0=0x00088004`, 6 f64 coords × 2) is a 2023-specific format.

### What DID stay stable between 2023 and 2024

From probe output, the fixed_header check couldn't run because zero records
had the 2023 variant. What we can observe empirically:

1. **The record-prefix filter (buf[+2..+4] == 0x0000) still fires** — 919
   filtered occurrences on 2024.
2. **The class name "ArcWall" still exists in the 2024 schema** — just with
   a different tag.

What is confirmed changed:
- Tag value (0x0191 → 0x019c)
- Record variant marker distribution at +0x10
- Zero 0x07fa records means the "standard record" format itself changed

## Hypotheses

**H17 (new, conf 0.8)** — Revit 2024's ArcWall records use a different
variant scheme. The top-distribution values on 2024 (0x0000, 0xffff, 0x0003,
0x000d, 0x053b) aren't the "envelope variant" as in 2023 — they may be a
u16 length prefix, a content-type ID, or some other field in a
restructured record header.

**H18 (new, conf 0.7)** — The 2024 record envelope is *larger* than 2023.
2023 had 115 B fixed core; the 2024 "smaller variants" (0xffff, 0x0003)
might only be valid u16 values at certain positions within a larger record
that starts at a different offset.

**H19 (new, conf 0.6)** — The 2023 decoder's record-prefix filter
`buf[+2..+4] == 0x0000` is too broad for 2024 (919 hits vs expected
~30-60 walls in a ~97 MB file is way too many). The 2024 filter may need
adjustment to isolate real walls.

## Decisions

**D23** — Ship the current decoder with documented scope "Revit 2023 only".
Do NOT overclaim 2024 compatibility. The existing DEC-05 test is valid
for 2023 Einhoven but will correctly report "no 2024 ArcWalls" on a 2024
file (which is accurate — we cannot decode them yet).

**D24** — Schedule RE-14.4: repeat the RE-14.3 hex-dump methodology on
2024 Core Interior with tag 0x019c and variant 0x0000 (or 0x053b — need
sample to decide). This is a re-RE effort, not a patch.

**D25** — Add version-keyed tag resolution to the decoder API. A future
`ArcWallRecord::decode(schema, buf, offset)` would look up ArcWall's tag
per schema version instead of using the 0x0191 constant.

**D26** — Update `src/arc_wall_record.rs` doc comments to clearly state:
"Supports Revit 2023 standard-variant records only. 2024 record format
has drifted and is not yet implemented (see RE-13)."

**D27** — Update `tests/arc_wall_corpus.rs` and
`tests/walker_to_ifc_integration.rs` comments to note these tests gate
only on the 2023 Einhoven file; they would yield zero IFCWALLs on 2024.

## Open questions

**Q18** — What is the 2024 ArcWall record envelope? 919 candidates; variant
distribution has no clear peak (top bucket 0x0000 at 259/919 = 28%).
Maybe the 2024 format ditched the variant-marker discriminator and uses
a different indicator (e.g. length prefix).

**Q19** — Does the 2024 record body still carry 6 f64 coordinates, or did
Autodesk refactor the geometry representation (e.g. location line as a
reference ID)? Hex-dump of a sample 2024 record can answer this.

**Q20** — How much of the rest of the ArcWall family (Wall, WallType,
HostObjAttr, etc.) changed between 2023 and 2024? If the variant scheme
is a global Revit format change, many decoders will need re-RE.

## Artifacts

- `examples/probe_arcwall_2024.rs` — 150-line cross-version probe
- `reports/element-framing/RE-13-synthesis.md` (this file)
- H17/H18/H19 (new); D23-D27 decisions

## Recommended next steps (prioritized)

1. **D26** — Update decoder doc comments for scope clarity. Low-risk,
   avoids future overclaim.
2. **RE-14.4** — Hex-dump 2024 ArcWall records (tag 0x019c, variant
   dominant 0x0000) to find the 2024 envelope. Same methodology as
   RE-14.3.
3. **D25** — Refactor decoder to accept a schema-resolved tag parameter.
4. **Cross-version audit of other concrete classes** — Q20. Does
   HostObjAttr (tag 0x006b in 2023, likely 0x006c-0x0070 in 2024)
   also have envelope drift? Needs a sampling probe.

## Coverage matrix (after this session)

| File | Revit version | ArcWall tag | Records | Decoder | IFCWALL emitted |
|---|---|---|---:|---|---:|
| Revit_IFC5_Einhoven.rvt | 2023 | 0x0191 | 32 | works | 24 |
| 2024_Core_Interior.rvt | 2024 | 0x019c | 919* | does NOT work | 0 |
| racbasicsamplefamily-2024.rfa | 2024 | (not yet probed) | ? | expected: 0 | ? |
| racbasicsamplefamily-2026.rfa | 2026 | (not yet probed) | ? | expected: 0 | ? |

*919 filtered occurrences; variant structure differs from 2023 so 0 of
these match the current decoder.

rvt-rs shipping claim: "ArcWall decoder works on Revit 2023 model files."
Not "works on all Revit versions" — that's a separate deliverable blocked
on RE-14.4.
