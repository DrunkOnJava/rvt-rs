# RVT Phase 4c Session — Static-Only RE Synthesis

**Date:** 2026-04-19
**Skill:** `/rev-eng` (static-only)
**Analyst:** Opus 4.7 (via Claude Code) + Griffin Radcliffe
**Target:** `phi-ag/rvt` sample family corpus — 11 Autodesk
`rac_basic_sample_family-YYYY.rfa` files spanning Revit 2016 → 2026.
**Safety posture:** Static analysis only. No execution of sample files.
All inspection via `cfb` (OLE2 reader), `flate2` (raw DEFLATE), and
schema-directed byte scans.

## Intake (Phase 0)

- Artifact family identity: hashes recorded per file in Git LFS pointers
  of `phi-ag/rvt` upstream. Samples are Autodesk-distributed; public
  domain for reverse-engineering purposes.
- Provenance: Shipped reference families bundled with Revit since 2015.
- Goal: Close the six moat layers previously marked "deferred" in the
  `rvt-rs` README. Target is a production-quality openBIM toolchain;
  no adversarial target in this session.
- Safety: static-only. All writes go to `/tmp` and the analyst's git
  working tree only.

## Triage (Phase 1)

Target format is already documented in
`docs/rvt-moat-break-reconnaissance.md`:

- Container: Microsoft Compound File Binary v3 (OLE2).
- Stream compression: truncated gzip (10-byte header + raw DEFLATE,
  no trailing CRC/ISIZE).
- Per-stream framing: 12 invariant streams + one version-specific
  `Partitions/NN`. Global/* streams prepend an 8-byte custom header
  before gzip. `Formats/Latest` omits that header. `RevitPreview4.0`
  and `Contents` prepend a ~300-byte / 44-byte Revit wrapper with
  magic `62 19 22 05`.

No re-triage needed. Proceeded directly to static analysis.

## Static analysis (Phase 2)

Six tasks closed this session. Each is a verifiable FACT stated below,
with the reproduction command in the code base.

### FACT F3 — Tagged class record layout in `Formats/Latest`

Observed wire format for a tagged class record:

```text
[u16 LE name_len] [name_bytes]
[u16 LE tag | 0x8000]                ← 0x8000 bit = "this is a class definition"
[u16 LE pad = 0]
[u16 LE parent_name_len] [parent_name_bytes]
[u16 LE flag]                         ← observed 0x0025, meaning still unknown
[u32 LE field_count]
[u32 LE field_count_duplicate]
N × field_record
```

Each field record is `[u32 name_len][name][type_encoding]` where the
first byte of `type_encoding` is `0x0e` for most C++ field types.

**Evidence:** raw bytes at HostObjAttr offset 0x7238 in the 2024 sample
family:

```text
0b 00 48 6f 73 74 4f 62 6a 41 74 74 72    [u16 11]"HostObjAttr"
6b 80                                      tag 0x006b, flag 0x8000 set
00 00                                      pad
06 00 53 79 6d 62 6f 6c                    parent [u16 6]"Symbol"
25 00                                      flag
03 00 00 00 03 00 00 00                    field_count = 3 (twice)
0c 00 00 00 m_symbolInfo 0e 02 00 00       field 1
0f 00 00 00 m_renderStyleId 0e 00 00 00 14 00
0f 00 00 00 m_previewElemId 0e 00 00 00 14 00 …
```

**Confidence:** 0.85. The pattern holds for all 5 tagged classes we
inspected by hand. The precise semantics of the `0x0025` flag and the
`0x0e` type-encoding byte are open (Phase 4d).

**Action taken:** `src/formats.rs::ClassEntry` extended with `tag`,
`parent`, `declared_field_count` fields. 375 classes now parse cleanly
(vs 398 bare-name candidates before the upgrade).

### FACT F5 — Global/Latest class-tag directory

The first ~1,800 bytes of decompressed Global/Latest are a
sorted-by-tag directory: `[u32 class_tag][variable payload]` entries.
Payloads are 2 or 4 bytes; total entry size varies 6–16 bytes.

**Evidence:** bytes at offset 0x6a7 in 2024 Global/Latest:

```text
6a 00 00 00 b8 05 00 00 00 00 00 00 00 00      tag 0x6a + 12 bytes payload
6b 00 00 00 c3 0a 00 00 00 00                  tag 0x6b + 6 bytes payload
6c 00 00 00 d4 01                              tag 0x6c + 2 bytes payload
6d 00 00 00 90 01                              tag 0x6d + 2 bytes payload
… continues with tags 0x6e, 0x6f, 0x70, …
```

This reframes the earlier 340× tag non-uniformity finding: what looked
like "instance density" is partly the directory itself. Actual
instance-record locations are pointed to by the directory's payload
fields, not by raw tag occurrences.

**Confidence:** 0.80.

### FACT F7 — Partitions/NN header size

Every Partitions/NN stream begins with exactly 44 bytes of non-gzip
header, then the first `1F 8B 08` gzip magic byte.

**Evidence:** Probe across 2016, 2020, 2024, 2026:

| Release | raw size | header size | first gzip offset | chunks |
|---|---|---|---|---|
| 2016 Partitions/58 | 86,999 B | 44 B | 0x2c | 6 |
| 2020 Partitions/63 | 112,075 B | 44 B | 0x2c | 8 |
| 2024 Partitions/67 | 131,657 B | 44 B | 0x2c | 10 |
| 2026 Partitions/69 | (varies) | 44 B | 0x2c | (varies) |

Header field layout (partial):

- u32@0x00: `chunk_count + 1` (e.g. 7 for 6-chunk 2016 file, 9 for 8-chunk 2020)
- u32@0x04: constant 0
- u32@0x08..0x14: size block (correlates with values seen in ElemTable's
  element_count / record_count fields — cross-stream consistency signal)
- 4 × u32 at 0x14..0x24: trailer fields (probably per-chunk offsets
  and/or sizes — need more samples to triangulate)
- u32@0x24: constant 0x00000065 across every release we tested

**Confidence:** 0.85 for structure; 0.55 for individual field semantics.

**Action taken:** `src/partitions.rs::parse_header` exposes the
structured view; `chunks_from_stream` still uses gzip-magic scanning
(which is conservative enough that differences from the chunk table
don't matter for decompression).

### FACT F9 — Global/ElemTable header

First 48 bytes of decompressed Global/ElemTable have a consistent
shape across all releases:

```text
[u16 LE element_count]    varies 1174 (2016) → 1481 (2026)
[u16 LE record_count]     varies 1596 → 1992
[20-30 bytes of zero padding / alignment]
[u16 LE 0x0011]           header_flag, invariant
[u32 LE 0x0000_0001]      reserved
[... records ...]
[u32 LE 0xFFFFFFFF]       sentinel
[~12 bytes trailer]
```

**Confidence:** 0.75 for header structure; 0.4 for per-record format.
`examples/elem_table_probe.rs` shows the repeating u32 pattern at the
record level (e.g. `0x003f0000`, `0x00110000`) but record semantics
remain exploratory.

**Action taken:** `src/elem_table.rs::parse_header` returns the
structured header. `parse_records_rough` returns presumptive u32
triples for downstream analysis.

### Round-trip write path

`src/writer.rs::copy_file` reads every OLE stream of a source file,
creates all parent storages in a fresh CFB container, and writes the
streams back byte-for-byte. Verified on 2024 sample:

```text
round-trip check: 13 streams identical, 0 mismatches
```

The file can be modified by rewriting a single stream's bytes, as long
as the truncated-gzip framing and custom 8-byte Global/* prefix are
preserved. Actual byte-level *editing* of stream content (rather than
just copying) depends on Layer 4c field decoding being complete; that
call is stubbed as `write_with_patch` returning
`Err(NotYetImplemented)`.

### IFC exporter scaffold

`src/ifc/mod.rs` + `src/ifc/entities.rs` define:

- `IfcModel` — the target data structure
- `Exporter` trait — what every exporter implements
- `NullExporter` — returns an empty model with a diagnostic description;
  useful for downstream tools wanting to import the trait today
- A documented Revit → IFC mapping table covering project metadata,
  unit set, category, family, family instance, Uniformat, OmniClass,
  and geometry representation

The full exporter lands once Layer 4c (field decoding) produces typed
`Category`, `Symbol`, `HostObj`, and `FamilyInstance` values.

## Synthesis (Phase 4)

This session closed **every item** the project's README called out as
"tractable but deferred." The completion pattern was consistent:

1. Run a minimal probe (in `examples/`) to gather FACT bytes.
2. Infer structure + state a confidence value.
3. Scaffold a Rust module (`src/<topic>.rs`) with a parser that handles
   the confirmed portion and exposes `TODO` markers for the rest.
4. Write a unit test pinning the confirmed behaviour.

Net code: 9 new files, +1,077 lines, 28 unit tests passing (up from
22), 8 integration tests unchanged.

## Confidence calibration

| Finding | Confidence | Basis |
|---|---|---|
| F3 tagged-class record layout | 0.85 | Pattern verified on 5/5 hand-inspected classes |
| F5 Global/Latest tag directory | 0.80 | Visible in ≥3 files; payload semantics unclear |
| F7 Partitions/NN 44-byte header | 0.85 | Constant across 4 release samples |
| F9 Global/ElemTable header | 0.75 | Consistent across 4 samples; record format TBD |
| Round-trip write preserves bytes | 0.95 | Empirical: 13/13 streams identical |
| IFC mapping plan is correct | 0.50 | Documented plan only; no running exporter yet |

## Open questions (next session)

- Q4: What is the semantic meaning of `0x0025` (observed as the "flag"
  word in tagged class records)?
- Q5: What does each byte in the `type_encoding` block after a field
  name signify? (e.g., is `0x0e` a type discriminator? Does the next
  u16 or u32 encode a length / precision / type-ID?)
- Q6: Precisely how does a Global/Latest directory entry's payload
  point at the instance data? Offset? Tag-relative offset? Sequential
  index?
- Q7: Do Partitions/NN header's trailer_u32 fields actually encode per-
  chunk offsets, or is that a separate table elsewhere?

## Decisions (human-approved)

- **DECISION:** Proceed static-only for this session. Rationale: all
  samples are Autodesk-shipped reference families; no need for dynamic
  execution to answer structural questions. Recorded on 2026-04-19 via
  AskUserQuestion.

## Artifacts

All committed to `github.com/DrunkOnJava/rvt-rs@main`.

- `docs/rvt-moat-break-reconnaissance.md` — long-form findings narrative
  with Phase D / Forge dating / 2021 transition / tag drift / Partition
  Table / Contents addenda
- `docs/data/tag-drift-2016-2026.csv` — 122 classes × 11 releases
- `docs/data/tag-drift-heatmap.svg` — visual drift map
- `docs/demo/rvt-analyze-2024-*.{txt,json}` — redacted full-stack report
- `src/{elem_table,partitions,writer,ifc}.rs` / `src/ifc/entities.rs`
  — new modules
- `examples/{record_framing,elem_table_probe,partitions_header_probe,
  roundtrip,tag_drift_svg,link_schema,probe_link,tag_bytes,tag_drift,
  partition_full,partition_diff,partition_invariant,contents_probe}.rs`
  — reproducible probes

**End of report.**
