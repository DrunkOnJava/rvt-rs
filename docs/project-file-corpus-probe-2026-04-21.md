# Project-file corpus probe — 2026-04-21

Q-01 partial unblock. Three publicly-redistributable real `.rvt` project
files were located and probed through the shipped CLIs. This document
records what survived vs what got corrected vs what opens brand-new
research threads.

## The corpus

All three are project files (`.rvt`), not family files (`.rfa`). They
represent different Revit releases and project types, giving us the
first cross-release project-file data we've had:

| File | Revit | Size | Source (redistributable) | Notes |
|---|---|---|---|---|
| `Revit_IFC5_Einhoven.rvt` | 2023 | 913 KB | [magnetar-io/revit-test-datasets](https://github.com/magnetar-io/revit-test-datasets) | Small IFC5-export test fixture |
| `2024_Core_Interior.rvt` | 2024 | 34 MB | [magnetar-io/revit-test-datasets](https://github.com/magnetar-io/revit-test-datasets) | Full interior project, 6 `Partitions/NN` streams |
| `MyRevitProject.rvt` | 2025 | 459 KB | [SSeelos/MyRevitProject](https://github.com/SSeelos/MyRevitProject) | Small sample — no license declared, probe-only |

None of these are committed to the repo (the magnetar files are Git LFS
in an MIT-licensed repo; the Seelos file has no license declared). The
probe operates on the files in place via a local clone / download.

## What held

The thesis-level claims are preserved across the project-file jump:

- **Schema → data linkage (Phase D Finding #1).** On
  `2024_Core_Interior.rvt`, tag `0x0061` (`AbsCurveGStep`) hits 20,035
  times in the ~1 MB decompressed `Global/Latest`, at **323× the
  uniform-random rate**. Family-file finding was "340× in 938 KB
  of decompressed Global/Latest". The linkage mechanism is intact; the
  exact ratio shifts slightly by file.
- **Tag assignment is the live type dictionary.** Top-5 tags on the
  real project file (`0x0061` `AbsCurveGStep`, `0x006b`
  `HostObjAttr`, `0x006d` `AbsDbViewPressureLossReport`, `0x0062`
  `GeomStep`, `0x0046` `ATFProvenanceBaseCell`) are the same
  class families that dominate family-file Global/Latest. The schema
  is the type dictionary for both file variants.
- **100% field-type classification generalises (Q5.2).** Both
  project files decode every field in their `Formats/Latest` schema
  with **zero `Unknown`** variants:

  | File | Fields | Unknown | Classified |
  |---|---|---|---|
  | `Revit_IFC5_Einhoven.rvt` | 1161 | 0 | 100.00 % |
  | `2024_Core_Interior.rvt` | 1114 | 0 | 100.00 % |

  The `FieldType` enum's 11 discriminator bytes mapped against the
  family corpus generalise cleanly to project files. See
  `examples/probe_project_coverage.rs`.
- **ADocument walker works on Revit 2023 project files (L5B-11
  partial).** Previously documented as "reliable on 2024–2026;
  2016–2023 entry-point detection pending" — but both the 2023 and
  2024 project files emit a clean walk:

  | File | Fields read | Entry offset | Diagnostics |
  |---|---|---|---|
  | `Revit_IFC5_Einhoven.rvt` (2023) | 13 | 0x1ee5 | 0 |
  | `2024_Core_Interior.rvt` (2024) | 13 | 0x157e0 | 0 |

  L5B-11 remains open for the 2016–2022 range (no project corpus),
  but the 2023 block is now covered. See
  `examples/probe_project_walker.rs`.
- **Schema table size.** 398 classes in the 2024 project vs 395 in
  the 2024 family sample — within measurement noise. The class
  inventory doesn't meaningfully differ between file variants.
- **`forbid(unsafe_code)` + bounded inflate.** Every security
  invariant held — no panics, no OOMs, no overruns across all 20
  streams of the 34 MB file. The Q-04 fuzz-regression harness
  predicted this and it came out clean.
- **IFC4 STEP export emits valid scaffold on real project files.**
  `./target/release/rvt-ifc <real.rvt> -o out.ifc` produces
  35-line ISO-10303-21 output with valid `FILE_SCHEMA(('IFC4'))`,
  `IfcUnitAssignment` (mm/m²/m³/rad), `IfcOwnerHistory`, and spatial-
  tree placeholders on both the 913 KB and 34 MB project files. The
  STEP is syntactically valid for IfcOpenShell ingestion. The
  per-element content (walls, doors, slabs, etc) isn't yet wired
  through to the emission path — that's the known 'document-level
  scaffold' scope in the README. What matters for this probe: no
  crashes, no invalid STEP, no unit errors, no placement errors
  under the existing emission paths.
- **`Formats/Latest` is near-byte-invariant across variants.**
  Running `rvt-corpus` across the 2023 project, 2024 project, and
  2024 family sample finds **17,266 bytes byte-for-byte identical**
  and another 406,434 bytes low-variance (differ in small regions)
  across all three. The schema Revit ships is the same architectural
  artefact for family and project files — the 364-byte run starting
  at offset 0 (class-name table) is identical across all variants.

## What needed correction

### The "stable Revit format-identifier GUID" claim is scope-narrower

**README Finding #5** says:

> `Global/PartitionTable` is 167 bytes decompressed, and **165 of
> those bytes are byte-for-byte identical across every Revit release
> 2016-2026** (98.8% invariant). The invariant region contains a
> never-before-published UUIDv1: `3529342d-e51e-11d4-92d8-0000863f27ad`.

That claim is true **within family files** across 2016-2026. Project
files do NOT carry that GUID. Three project files → three distinct
GUIDs:

| File | Format ID GUID | Invariant region |
|---|---|---|
| Family corpus (2016-2026) | `3529342d-e51e-11d4-92d8-0000863f27ad` | 165 / 167 bytes |
| `Revit_IFC5_Einhoven.rvt` (2023 project) | `6a6261fd-00b0-4ba3-87c6-a29f5a378756` | 85 / 87 bytes |
| `2024_Core_Interior.rvt` (2024 project) | `552368c6-d221-4814-abfb-423e900d87f8` | 85 / 87 bytes |
| `MyRevitProject.rvt` (2025 project) | (zero / different layout entirely) | — |

The invariant-region size itself is different (85 bytes vs 165),
strongly suggesting project files use an entirely different
`PartitionTable` layout, not just a different GUID in the same
layout.

**Correction needed** in README + `docs/rvt-moat-break-reconnaissance.md`:
the "stable GUID" moat-layer anchor is a *family-file* anchor, not a
universal Revit-file anchor. File-type sniffers that rely on it will
reject real `.rvt` projects.

### The `inflate_at(..., 8)` hard-code broke on `.rvt`

First observed on `MyRevitProject.rvt`: the `rvt-history` CLI failed
with "DEFLATE at offset 8: corrupt deflate stream" because its
Global/History stream doesn't have the family-file 8-byte custom
prefix. Already fixed in commit for ADR-002 — `inflate_at_auto`
probes for the first gzip magic and inflates from there, falling
back to offset 8 only if no magic is present.

## What's genuinely blocked on more RE

### Walker → IFC element emission (option A from the
tonight's decision prompt)

`rvt-ifc` produces a valid IFC4 STEP **scaffold** (IfcProject +
IfcSite + IfcBuilding + IfcUnitAssignment) on real project files,
but the per-element walk into IFC entities (IfcWall, IfcSlab,
IfcDoor, etc.) is not wired up yet on the exporter side. Tracing
through the code:

- `RvtDocExporter::export` in `src/ifc/mod.rs` has the comment
  *"other entity types are wired in the walker-expansion phase"* —
  explicit TODO for the per-element wiring.
- `ElementDecoder` trait + 80 registered decoders + a full
  `from_decoded::*` helper suite all exist, but no top-level
  `walker::iter_elements(rf) -> Vec<DecodedElement>` function
  orchestrates them yet.
- The ADocument root on the 2023 project has 13 fields, all
  pointers (`m_elemTable = Pointer [2097249, 49]`, etc). Following
  those pointers requires parsing `Global/ElemTable` record
  semantics.
- `elem_table::parse_records_rough` works on family files (45
  records on the 2024 sample) but returns only **2 records** on
  the 2024 project file even though the header reports 26,425
  records in a 1 MB decompressed stream. The sentinel-scan
  early-terminates on a `0xFFFF_FFFF` pattern that appears in the
  project file's record body, not just as a trailer — project-file
  records evidently pack u32 data differently from family files.

One bounds fix landed in this commit: `parse_records_rough` now
starts its sentinel scan at offset `0x30` (past the header) rather
than offset 0, closing a trivial bug where the header region
contained byte patterns that looked like the sentinel. That's a
strict improvement but doesn't solve the deeper record-semantics
gap — it's a genuine RE follow-up needing more project-file corpus
to triangulate.

Concretely unblocking walker → IFC needs:

1. Real-world project-file **ElemTable record layout** (not yet
   known — 26,425 records × the actual byte pattern per record).
2. Top-level **`walker::iter_elements(rf)` orchestration** (not
   shipped — dispatch pattern from `all_decoders()` exists, but
   the iteration spine needs the record layout first).
3. **Class-tag → IFC entity mapper** in `ifc::from_decoded` (parts
   shipped — the per-element geometry helpers exist, the top-level
   `DecodedElement → IfcEntity` switch doesn't).

That's a real multi-session feature. Documented honestly so the
next contributor knows where to start.

## New research threads

### Multiple `Partitions/NN` streams in project files

Family files have exactly one `Partitions/NN` stream. Both magnetar
project files have six:

  `Partitions/46, 48, 51, 53, 55, 59`  (2024 file — 170 MB decomp)
  `Partitions/0, 1, 2, 3, 4, 5`        (2023 file — 3.7 MB decomp)

The numbering patterns differ sharply (2024 uses 46/48/51/53/55/59
— non-contiguous; 2023 uses 0-5 contiguous). This mirrors the known
family-file version-numbering pattern (2016=58, 2018-2026=60-69,
skipping 59) but at project scale. The partition-NN-number is
presumably a release marker for a per-partition piece of state
(workset? view phase? design option?) — needs further probing across
more files.

### Global/Latest on the 2025 sample

`MyRevitProject.rvt` (Revit 2025) had `Global/Latest` with **no gzip
magic at any offset** (raw=68069, "no gzip magic found" per
rvt-dump). The other two project files' `Global/Latest` decompressed
cleanly as normal truncated-gzip. Three possibilities:

1. The 2025 file is corrupted (upload artifact).
2. Revit 2025 introduced a new compression format for
   `Global/Latest` that we haven't seen in 2023/2024.
3. The file was saved with a newer Revit variant (e.g. a cloud
   workshared model) that uses a different stream body.

Can't disambiguate with three files. Needs more 2025 samples.

## Q-01 status

Unblocked for:

- L5B-11 walker extension to 2016-2023: has real 2023 project data now
- L5B-59 graceful degradation: shown to hold on 3 new files
- Stream-framing research: uncovered the project-file variation

Still blocked on:

- Multi-megabyte benchmarks (Q-07): have 34 MB file, but benchmark
  harness needs adjustment for non-LFS corpus path
- CFB structural writer (WRT-10): can now test against a real project
- Community-sourced corpus at scale (Q-01 proper): 3 files is a good
  start, want 10+

## Repro

```bash
# Checkout magnetar corpus (MIT licensed, LFS-tracked)
git clone https://github.com/magnetar-io/revit-test-datasets /tmp/magnetar

# Probe each
for f in /tmp/magnetar/Revit/*.rvt; do
  ./target/release/rvt-analyze --redact "$f" | head -50
done
```
