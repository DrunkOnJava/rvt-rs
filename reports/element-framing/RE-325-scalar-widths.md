# RE-325 — The schema's base-2 and base-3 scalars are one and two bytes wide

**Date:** 2026-11-04
**Status:** positive. A field whose `Formats/Latest` descriptor has base byte 2 (the C++ `char`) occupies 1 byte in an instance, and base byte 3 (the C++ `short`) occupies 2. `FieldType::decode` gave them 2 and 4.
**Issues:** #1196 (classify the schema's field encodings from byte evidence); builds on RE-323 (#1309), which found the native graph decoder reading these two bases at 1 and 2 bytes and decoding 97,000 of 97,005 records of Revit 2027's sample projects with them
**Probe:** `probes/examples/probe_re325_scalar_widths.rs`
**Artefacts:** the `phi-ag/rvt` family corpus at bec25470413fbe659c3ab30d4cfd029b73d63fc3, the 2014 maker family and Autodesk's 58 sample projects of Revit 2015 to 2027, the files and sha256 of RE-323 (`reports/element-framing/RE-323-residual-field-encodings.md` lists the family hashes; `research/autodesk-sample-projects.tsv` the projects'). Measure fetches them for each run and never stores them.
**Runs:** Measure 41762248491 (families), 41762555375 (samples 2015 to 2020), 41762559148 (2021 to 2026), 41762563120 (2027), all of probe commit `6f099c91` (the classification as it was before this change); 41765293473 (families, the probe on the head `3084c431`: the scan's kind-2 and kind-3 primitives are size 1 and 2 in all 13 files, with the same counts as before); 41765290302 (head against `main`, §6)
**Result:** the 58 sample projects (13 releases, 2015 to 2027) hold 402,652 native records with an instance of a base-2 or base-3 field. 402,293 of them decode without the extensible-storage catalog (the other 359, in the 2024 projects, need it), and every one of those is read completely with 1 and 2 bytes. Reading the same bytes with a base-2 field at 2 or 4 bytes, or a base-3 field at 1 or 4, leaves a record whole only when it holds no non-empty instance of that field: none of the 683,178 non-empty instances (683,250 with 72 empty byte buffers) is read at another width.

## 1. What was written, and what the decoders did

`FieldType::decode` (`formats.rs`) gave `Primitive { kind: 0x02 }` a size of 2 ("u16") and `Primitive { kind: 0x03 }` a size of 4 ("deprecated i32-alias, seen 2016 to 2018 only, fields `UserID.m_id` and `ElementRegenerationInfo.m_nAtomType`"), and `walker::read_field_by_type` read the first as a `u16`. The native graph decoder (`native_parameters`, `native_index`) reads base 2 as one byte and base 3 as an `i16`. Nobody had shown, field by field, which is right; RE-323 found only that the native decoder's totals were high.

## 2. The schema names the types

In a `std::pair< A, B >` class the fields `first` and `second` have the types the class name gives. The probe tallies the base byte of every such slot whose type is not a class (`pair-slot`):

| Base | C++ type of the pair slots | Slots in 2026 (other releases: 19 to 21 for `char` and `short`, a few more or fewer for the rest) |
|---:|---|---:|
| 1 | `bool` | 25 |
| 2 | `char` | 21 |
| 3 | `short` | 21 |
| 4 | `int`; `unsigned int` in 2025 to 2027 | 100; 4 |
| 5 | `unsigned long` | 3 |
| 6 | `float` | 8 |
| 7 | `double` | 16 |
| 11 | `int64_t` | 21 |

The map is one to one in all 13 files (the 2014 maker family and the 2016 to 2027 families): base 2 carries `char` and nothing else, base 3 `short` and nothing else. That is Revit's own name for a 1-byte and a 2-byte scalar, and it is corroboration, not the proof: §4 is.

## 3. Which fields have each base, in each release

The exact grammar (`schema_registry`) lists, per release, the fields below. Every project of one release has the same census (the 58 projects give 13 distinct ones). Nested counts the `short[3]` arrays, which sit in a composite (base 13) field with an empty name in `FacetedTopology0`, `FacetedTopology2` and `FacetedTopology4`.

| Release | base 2 scalar | base 2 container (`0x50`) | base 3 scalar | base 3 `[3]` array | Scan: `Primitive` kind 2 | Scan: `Container` kind 2 | Scan: `Primitive` kind 3 |
|---|---:|---:|---:|---:|---:|---:|---:|
| 2014 (maker family) | 34 | 9 | 24 | 3 | 17 | 6 | 8 |
| 2015 | 34 | 12 | 25 | 3 | 17 | 9 | 9 |
| 2016 | 34 | 12 | 25 | 3 | 17 | 9 | 9 |
| 2017 | 34 | 20 | 25 | 3 | 17 | 17 | 9 |
| 2018 | 36 | 21 | 25 | 3 | 19 | 18 | 9 |
| 2019 | 36 | 25 | 25 | 3 | 19 | 22 | 9 |
| 2020 | 36 | 31 | 25 | 3 | 19 | 28 | 9 |
| 2021 | 36 | 31 | 25 | 3 | 19 | 28 | 9 |
| 2022 | 36 | 32 | 25 | 3 | 19 | 29 | 9 |
| 2023 | 36 | 32 | 25 | 3 | 19 | 29 | 9 |
| 2024 | 38 | 31 | 27 | 3 | 19 | 28 | 9 |
| 2025 | 38 | 31 | 27 | 3 | 19 | 28 | 9 |
| 2026 | 38 | 31 | 27 | 3 | 19 | 28 | 9 |
| 2027 | 38 | 32 | 27 | 3 | 19 | 29 | 9 |

The base-3 scalars are `UserID::m_id`, `ElementRegenerationInfo::m_nAtomType`, `Viewer::m_viewerFlags`, `Clipboard::m_index`, `LinePattern::m_pixelPattern`, `OneDependencyPropagationFilterData::m_atomType` and the pair slots (`std::pair< short, K >::first`, `std::pair< K, short >::second`, 19 to 21 of them). The base-2 scalars are the `*_flags` and `m_data` fields (`Dimension::m_flags`, `GEdge::m_flags`, `EdgeGeometry::m_flags`, `GeomStepList::m_flags`, `WitnessRefInfo::m_constrFlags`, `MullionFamRef::m_version`, `StairsAttributes::m_dummy`, ...) and the `char` pair slots; the containers are byte buffers (`ARasterImage::m_compressedImage`, `RtfData::m_arrRtfStream`, `GRichText::m_arrRtfRawData`, `ByteData::m_data`, `*ImagePng`, `FabricationDatabase::m_mem`, `SignatureMarker::m_privateSignature`, ...).

The deprecated-alias entry was wrong on width and on release. `0x03` is in every release 2014 to 2027, not 2016 to 2018, with 24 to 27 scalar fields. The scan (`parse_schema`) files 8 or 9 of them (`UserID::m_id`, `ElementRegenerationInfo::m_nAtomType`, `Viewer::m_viewerFlags`, `Clipboard::m_index`, `LinePattern::m_pixelPattern`, `OneDependencyPropagationFilterData::m_atomType` and three `ZoningAppInfo::second`) in every release; the old note listed two of them. The scan misses the pair slots and the arrays inside composites (grammar-only: 43 of the 58 to 61 scalar names through 2023, 48 of 65 from 2024), and files a few under an enclosing class (RE-323 §1: the same bytes under a different class name).

## 4. The instances

The instance reader is the native graph decoder (`native_document::extract_graphs`): a record's body decodes only if the whole of it is consumed, so a field read at a wrong width fails the record or leaves bytes over. The 13 family files cannot be read by it (`invalid increment episode range`), so the instances come from Autodesk's 58 sample projects. For every record whose graph is complete and holds an instance of a base-2 or base-3 field the probe decodes the same bytes again with `native_parameters::decode_graph`, on a copy of the registry in which the base byte of the field (or of every such field) is rewritten so that the decoder reads another width: base 2 read as 2 bytes is base 3, as 4 bytes base 4; base 3 read as 1 byte is base 2, as 4 bytes base 4. The control, the schema as it is, must decode the record without the extensible-storage catalog (the probe has none): 359 of the 402,652 records, all in the 2024 projects, need it and are left out, so 402,293 are tested.

**Every width at once** (`hyp`; "char 2, short 4" is `FieldType`'s reading). Records decoded by the native reader, over the 58 projects:

| Release | Projects | Records | Complete | Hold an instance | Control | char 2, short 4 | char 2, short 2 | char 4, short 2 | char 1, short 4 | char 1, short 1 | Failed records tried | read under any width |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 2015 | 6 | 121,147 | 119,527 | 40,217 | 40,217 of 40,217 | 6 | 276 | 276 | 39,342 | 39,342 | 1,620 | 0 |
| 2016 | 3 | 39,363 | 39,040 | 16,065 | 16,065 of 16,065 | 3 | 220 | 220 | 15,690 | 15,690 | 323 | 0 |
| 2017 | 3 | 39,473 | 39,150 | 16,660 | 16,660 of 16,660 | 3 | 220 | 220 | 16,285 | 16,285 | 323 | 0 |
| 2018 | 3 | 39,668 | 39,658 | 16,973 | 16,973 of 16,973 | 3 | 220 | 220 | 16,598 | 16,598 | 10 | 0 |
| 2019 | 6 | 109,087 | 106,783 | 46,333 | 46,333 of 46,333 | 6 | 394 | 394 | 45,446 | 45,446 | 2,304 | 0 |
| 2020 | 3 | 39,101 | 39,091 | 16,740 | 16,740 of 16,740 | 3 | 220 | 220 | 16,365 | 16,365 | 10 | 0 |
| 2021 | 3 | 38,951 | 38,948 | 16,345 | 16,345 of 16,345 | 3 | 220 | 220 | 15,969 | 15,969 | 3 | 0 |
| 2022 | 3 | 39,208 | 39,205 | 16,345 | 16,345 of 16,345 | 3 | 220 | 220 | 15,969 | 15,969 | 3 | 0 |
| 2023 | 3 | 39,205 | 39,202 | 15,793 | 15,793 of 15,793 | 3 | 220 | 220 | 15,417 | 15,417 | 3 | 0 |
| 2024 | 10 | 218,367 | 218,246 | 80,409 | 80,050 of 80,050 | 17 | 635 | 635 | 78,134 | 78,134 | 121 | 0 |
| 2025 | 5 | 96,842 | 96,837 | 40,033 | 40,033 of 40,033 | 5 | 360 | 360 | 39,231 | 39,231 | 5 | 0 |
| 2026 | 5 | 97,189 | 97,184 | 40,033 | 40,033 of 40,033 | 5 | 360 | 360 | 39,231 | 39,231 | 5 | 0 |
| 2027 | 5 | 97,005 | 97,000 | 40,706 | 40,706 of 40,706 | 5 | 360 | 360 | 39,904 | 39,904 | 5 | 0 |
| all | 58 | 1,014,606 | 1,009,871 | 402,652 | 402,293 of 402,293 | 65 | 3,925 | 3,925 | 393,581 | 393,581 | 4,735 | 0 |

The records that are not complete are RE-323's (§2 there). The wrong readings complete only the records that hold no instance of what they rewrite. `FieldType`'s reading completes 65 of 402,293 records, and they are the 58 + 7 whose only base-2 instances are empty byte buffers (`FabricationDatabase::m_mem`, `StructuralConnectionHandlerType::m_detailedParametersBuffer` and `m_rangesOfApplicabilityBuffer`: a count of 0 occupies the same bytes at any width). Reading base 3 at 1 or at 4 bytes completes 393,581, and the 8,712 it leaves are exactly the records that hold a base-3 instance (4,846 `Viewer::m_viewerFlags`, 3,782 `LinePattern::m_pixelPattern`, 84 `FacetedTopology0` arrays). Of the 4,735 records the native reader cannot complete, none is read under any width.

**One field at a time** (`field-alt`): for each field with an instance, the same records are decoded with only that field rewritten (at most 2,000 records per field and width in each file):

| Field | Base | Releases | Instances | Alternative widths tried | Records tried (per width) | Complete |
|---|---:|---|---:|---|---:|---:|
| `ARasterImage::m_compressedImage` | 2, container | 2015 to 2027 | 13,517 (62,389,775 bytes) | 2, 4 | 13,517 | 0 |
| `RtfData::m_arrRtfStream` | 2, container | 2017 to 2027 | 8,067 (1,696,164 bytes) | 2, 4 | 7,285 | 0 |
| `GRichText::m_arrRtfRawData` | 2, container | 2019 to 2027 | 90 (18,398 bytes) | 2, 4 | 90 | 0 |
| `Dimension::m_flags` | 2 | 2015 to 2027 | 35,255 | 2, 4 | 34,935 | 0 |
| `GEdge::m_flags` | 2 | 2015 to 2027 | 111,985 | 2, 4 | 2,861 | 0 |
| `EdgeGeometry::m_flags` | 2 | 2015 to 2027 | 15,340 | 2, 4 | 3,282 | 0 |
| `GeomStepList::m_flags` | 2 | 2015 to 2027 | 285,513 | 2, 4 | 103,519 | 0 |
| `WitnessRefInfo::m_constrFlags` | 2 | 2015 to 2027 | 70,358 | 2, 4 | 33,257 | 0 |
| `WitnessRefInfoLazy::m_constrFlags` | 2 | 2024 | 134 | 2, 4 | 41 | 0 |
| `MullionFamRef::m_version` | 2 | 2015 to 2027 | 63,337 | 2, 4 | 46,749 | 0 |
| `MullionSymFamRef::m_flags` | 2 | 2015 to 2027 | 8,710 | 2, 4 | 8,710 | 0 |
| `MullionTrimNeighbor::m_flags` | 2 | 2015 to 2027 | 1,630 | 2, 4 | 1,221 | 0 |
| `DatumPlaneRec::m_endBubbleOverrides` | 2 | 2015 to 2027 | 3,889 | 2, 4 | 691 | 0 |
| `StairsAttributes::m_dummy` | 2 | 2015 to 2027 | 149 | 2, 4 | 149 | 0 |
| `Viewer::m_viewerFlags` | 3 | 2015 to 2027 | 4,846 | 1, 4 | 4,846 | 0 |
| `LinePattern::m_pixelPattern` | 3 | 2015 to 2027 | 3,782 | 1, 4 | 3,782 | 0 |
| `FacetedTopology0` (`short[3]`, nested) | 3 | 2015 to 2027 | 56,576 (169,728 shorts) | 1, 4 | 84 | 0 |
| `FabricationDatabase::m_mem` and 2 more | 2, container | 2015 to 2027 (2024) | 58 + 14, all empty | 2, 4 | 72 | 72: a count of 0 |

The values fit the widths: the base-2 flags take 0 to 215 (a `u8`; `Dimension::m_flags` 8 to 140, `GEdge::m_flags` 6 to 15), `MullionFamRef::m_version` is 1 in all 63,337 instances, `Viewer::m_viewerFlags` 0 to 7, `LinePattern::m_pixelPattern` 0 in all 3,782, and `FacetedTopology0`'s shorts 0 to 1,157 (indices into its point array).

**The byte buffers** hold files, and a file's own framing gives the count (`blobs`, `signatures`): the 13,517 `ARasterImage::m_compressedImage` arrays are 13,407 PNGs, 97 JPEGs and 13 GIFs by their first four bytes, and all 13,407 PNGs end with the IEND chunk's CRC (`49 45 4e 44 ae 42 60 82`) at exactly the declared count, one byte per element; the 8,067 `RtfData::m_arrRtfStream` and 90 `GRichText::m_arrRtfRawData` arrays all open with `{\rt`. At two bytes per element the same count would run past the file's end.

**Scope of the instances.** They were measured in the native records of the `Partitions/*` streams (channel 102) only; none comes from `Global/Latest`, which the walker reads, and no field of the walker's `ADocument` (bases 1 and 14) is a `char` or a `short`. Of the 9 base-3 scalars the scan files, 2 have instances (`Viewer::m_viewerFlags`, `LinePattern::m_pixelPattern`); the grammar's 24 to 27 base-3 scalars and three arrays have 3 fields with instances (those two and the `FacetedTopology0` array). For the others base 3 at 2 bytes rests on the base byte (which every instance of those three fields confirms) and the pair-slot names of §2. Of the 34 to 38 base-2 scalars 11 have instances, and of the 9 to 32 byte buffers 3 have non-empty ones.

## 5. What changed

- `FieldType::decode`: `Primitive { kind: 0x02 }` is 1 byte and `Primitive { kind: 0x03 }` 2 (`scalar_size`). Nothing else in the classification moves; `tests/field_type_coverage.rs` keeps its `RESIDUAL` lists. The doc table of `FieldType`, the `Primitive` and `Vector` docs, `docs/format.md`, `docs/extending-layer-5b.md` and `docs/container-wire-format-2026-04-21.md` carry the corrected widths, and `docs/format-reconnaissance.md` keeps its §Q5.2 entry with a correction note above it.
- `walker::read_field_by_type` and `write_field_by_type` read and write `Primitive { 0x02, 1 }` as a `u8` (was a `u16`) and `Primitive { 0x03, 2 }` as an `i16` (was bytes of an unread 4). A `0x02` vector or container, which the walker could only skip the count of (leaving the payload to be read as the next fields), is read whole with its count, `count` bytes, as `InstanceField::Bytes`, and writes back verbatim. The native decoder's conventions (base 2 `u8`, base 3 `i16`) are the walker's.
- `native_document::Options::keep_body` and `Record::body` hand a probe the bytes of a record, so that it can decode them under another reading of the schema. Off by default.
- Not changed, and a gap: `FieldType::decode` still classifies `03 10 00 00` (the `short[3]` arrays) as `Unknown`, since `0x03` is not in `is_vector_capable`. Those arrays sit in a composite field with an empty name that the scan never files, so no scan field is affected.

## 6. Identity

Measure 41765290302, `-f base=main -f nordic=true -f r2025=true -f samples=true -f holdout=true -f families=true`, head `3084c431` (the commits after it change a doc comment, a reset of `keep_body` in a pass no measured reader uses, this report and the changelog) against `main` at `5aa4da77`: 124 items, which are the reference models, the BIM4LCA office as built and at permit, puzzbobb's R2025 models, the held-out models (counts only), the 58 sample projects and the 13 families. The 222 `ifc.sha256`, 186 `diagnostics.json`, 212 `info.json` and 212 `info.exit`, 186 `schema.txt` and the scorer outputs of 68 exports and the Core Interior witnesses are byte-identical between base and head, and the scorer comparison is empty: no export, no `rvt-info` output and no score moved. Measure 41764287400 (reference models and families, head `88da3002` against `main` at `8b8f1fee`) says the same. Nothing the measurement runs reads a `char` or `short` field through the walker: `ADocument`'s fields are bases 1 and 14, and the typed element decoders' classes (`Level`, `WallType`, `Floor`, `Material`) have base-4 fields and no base-2 or base-3 one. What moves is `rvt-schema`'s JSON, where the kind-2 and kind-3 `Primitive` sizes become 1 and 2 (run 41765293473 above); Measure does not compare it.

## 7. Reproduce

```sh
gh workflow run measure.yml --ref main -f ref=6f099c911ec77abe0c9d443ba4950e6600f8b6b7 -f base=none \
  -f families=true -f models=none -f scorers=none -f probe=probe_re325_scalar_widths -f probe_scorer=-
gh workflow run measure.yml --ref main -f ref=6f099c911ec77abe0c9d443ba4950e6600f8b6b7 -f base=none \
  -f samples=true -f sample_years="2015 2020" -f models=none -f scorers=none \
  -f probe=probe_re325_scalar_widths -f probe_scorer=-
# the same with sample_years="2021 2026" and sample_years=2027
```

`6f099c91` is the probe on `main` at `cebe1635`, before RE-323 merged. The probe prints, per file, the census of every base-2 and base-3 grammar field (`census`, `census-shape`), the slot types of the pair classes (`pair-slot`), the scan's reading (`scan-census`, `scan-field`, `grammar-only`, `scan-only`), the native totals (`native-records`), each field's instances with value ranges and blob signatures (`field-width`), and the decodes under the other widths (`field-alt`, `hyp`). A family file prints everything but the native lines (`invalid increment episode range`).
