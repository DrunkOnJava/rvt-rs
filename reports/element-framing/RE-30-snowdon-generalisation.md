# RE-30 — element records do not generalise to Autodesk's Snowdon Towers samples: most use a second prologue with no ElementId where the first keeps it

**Date:** 2026-09-22
**Result:** **positive** on the `Global/ElemTable` layout (a stride bug,
fixed). **Measured negative** on element recovery beyond it: on both
Snowdon files most element records of every recovered category carry a
second prologue whose own ElementId is not at `+0x00`, not the last slot
of the reference list, and not at any fixed offset before the record.
Recovery stays fail-closed, so those elements are not emitted.
**Corpus (local only, not redistributable, never committed):**

| file | sha256 | build | `Unique Document Increments` |
|---|---|---|---|
| `Snowdon Towers Sample Architectural.rvt` | `33271010919acb2a0d0d040851393a511d86ea7fd9a1f4c054797ae03e5e44a1` | `20230405_1134_fb02163c5b4b` | 71 |
| `Snowdon Towers Sample Structural.rvt` | `6dc833f9fe9fa702607265f2e3d581a6d2a928ea56133db8225c9b5e0f79a44f` | Revit 2024 | 22 |

Both are Autodesk's published 2024 sample projects. Autodesk's help pages
for Revit 2025, 2026 and 2027 link the same 2024 files; no public 2025+
project sample was found. The recorded edge every RE-21…RE-29 result was
measured on is `2024_Core_Interior.rvt` (magnetar-io, MIT, increments 66).
**Reference:** none for §1–§6. §7 (added the same day) uses a Revit 2024
IFC4 export of the identical architectural file, found in a public test-model
repository.

-----

## 1. Why this was measured

RE-21 to RE-29 recover walls, doors, windows, columns, slabs and rooms
from Revit 2024 partition element records with tolerance 0 against
Revit's own export — on one file. The support matrix scopes the claim to
that edge, but its user-facing line read as if any 2024 project would
behave the same. These two files are the only other public 2024 projects,
so they are the test of that reading.

Before this work `rvt-ifc` exported, from the architectural file, 64
`IfcSlab` named `Floor-unnamed` with no ElementId and no body, and 37
string-scanned `IfcSpace`; from the structural file, 8 walls, 3 slabs and
15 spaces, 11 of them with a body.

## 2. `Global/ElemTable`: the stride was wrong (fixed)

The architectural file's ElemTable is 1,889,350 bytes with
`record_count` 47,233 and `(1,889,350 − 30) / 40 = 47,233` exactly — the
40-byte Revit 2024 layout. `detect_layout` measured 120, because it took
the distance between the first two `0xFF` runs and records 1 and 2 have
none. At 120 bytes every one of 15,744 parsed rows read ElementId 0, the
declared set was `{0}`, and no element record could join — which is why
the export fell back to the plan-loop heuristic, whose cap is exactly 64
floors.

The run is not a marker. Read as a `u64` at record `+4`, the field is an
ElementId whenever it is not `-1`:

| file | records | field set | set values declared in the same table |
|---|---|---|---|
| `2024_Core_Interior.rvt` | 26,425 | 22,368 | 22,368 |
| Snowdon architectural | 47,233 | 29,177 | 29,131 |

The fix (`elem_table::detect_layout`) keeps the measured spacing when it
tiles the stream flush against `record_count` and otherwise tries the
known 40- and 28-byte strides that divide it and do. The architectural
file now declares 47,233 records (47,232 distinct ElementIds); every other
corpus file detects exactly as before. The export becomes 5 `IfcWall` and
1 `IfcSlab`, all with a Revit ElementId and a body, in place of the 64
guesses; the 37 spaces are unchanged (still string-scanned, §4).

## 3. The measurement

Every offset of every inflated `Partitions/*` stream where the 8-byte
little-endian `BuiltInCategory` id sits at `+0x12` was classified by the
RE-21 fail-closed tests (`partition_element_records::decode_at`). The
streams are the bytes `RevitFile::inflated_partition` returns, which is
what `rvt-dump` writes since #291; the dump before it lost every gzip
member that straddled a checksum page (10.5 MB of `Partitions/68` here),
and counts taken from it undercount every row below.

- **OK** — bbox marker `46 01 ff ff ff ff ab 05` at `+0x50` and a
  `u64` ElementId at `+0x00` declared in `Global/ElemTable`: decodes.
- **A** — the marker is at `+0x50` but `+0x00` is not an ElementId
  (`ffffffff_ffffffff` on 555 of the 1,216 architectural walls,
  `ffffffff_00000006` on 661).
- **undeclared** — a `u64` in ElementId range at `+0x00` that
  `Global/ElemTable` does not declare. RE-26 already found
  Revit leaving superseded frames in place; these are consistent with
  that, and rejecting them is the join doing its job.
- **no marker** — the category bytes occur but no record frame follows.

| file | category | OK | A | undeclared |
|---|---|---:|---:|---:|
| architectural | `OST_Walls` | 6 | 1,216 | 37 |
| architectural | `OST_Floors` | 1 | 198 | 1 |
| architectural | `OST_Doors` | 0 | 245 | 49 |
| architectural | `OST_Windows` | 0 | 150 | 17 |
| architectural | `OST_Rooms` | 0 | 54 | 0 |
| architectural | `OST_Columns` | 0 | 180 | 17 |
| structural | `OST_Walls` | 8 | 50 | 5 |
| structural | `OST_Floors` | 3 | 24 | 0 |
| structural | `OST_Doors` | 7 | 0 | 0 |
| structural | `OST_Windows` | 8 | 1 | 0 |
| structural | `OST_StructuralFraming` | 295 | 965 | 29 |
| structural | `OST_StructuralColumns` | 30 | 87 | 0 |
| structural | `OST_StructuralFoundation` | 3 | 95 | 3 |

Across the six categories the exporter recovers, the architectural file
has 7 decodable frames out of 2,171 and the structural file 26 of 106.

The A records are elements, not noise. From `+0x12` to the end of the
bbox they are byte-identical in shape to OK records (category, sentinel
padding, placement kind `0xffffef7f`, `0x0e55` at `+0x4a`, marker), the
1,216 architectural wall frames carry 1,214 distinct bounding boxes with
wall proportions (for example 1.1 ft × 18.9 ft × 24.7 ft), and 1,200 of
them carry a counted reference list whose every slot is a declared
ElementId.

## 4. Where the A-record ElementId is not

Three places were tested and ruled out:

1. **`+0x00`.** By definition of A. The first `0x12` bytes hold `0xFF`
   runs and 16-bit words (`0x03c1`, `0x07eb`, `0x04fe`, `0x066f`) where
   OK records hold the ElementId, a flags word and `0x059f`.
2. **The last reference slot.** On OK records the reference list ends
   with the record's own ElementId — on all 6 architectural walls (the
   first three are 2454523, 2455133, 2455140). On the 1,216 A walls the
   last slot takes only 605 distinct values (2084165 on 109 of them), so
   it is not a per-element id.
3. **A fixed offset before the record.** Scanning `u32` values at every
   even offset from `−512` to `+0x12` across 1,989 A records (walls,
   doors, windows, floors, columns), the best offset holds a declared id
   on 128 of 1,989 — no offset is an ElementId slot.

The 16-bit words also do not match save increments (the files report 71
and 22; the words run to 0x08aa). ElemTable's own 40-byte rows carry
words in the same range at `+0x18`, `+0x1c` and `+0x20`, but no join
between them and the A prologue was found.

Rooms are the one category where the negative changes nothing: all 54
architectural room frames are A, so the 37 spaces stay string-scanned
exactly as before.

## 5. What this changes

- `elem_table::detect_layout` no longer trusts the first two sentinel
  runs (§2). That is a fix on any file whose first records reference
  another element.
- The support matrix's `element-record-instance-recovery` row stays
  `verified` on the recorded edge; its user-facing line now says that on
  the Snowdon samples most element records use the A prologue and are
  not recovered.
- Nothing is emitted from A records. Emitting them would mean either
  inventing an ElementId or shipping id-less geometry, and RE-21's rule
  is that a random byte match cannot become an element.
- Structural categories (`OST_StructuralFraming` → `IfcBeam`,
  `OST_StructuralColumns` → `IfcColumn`, `OST_StructuralFoundation` →
  `IfcFooting`) stay unmapped. The 295 decodable framing records are the
  only instances available and there is no reference export to hold the
  RE-21 instance rule to for that category.

## 6. Next measurement

Locating the A-record ElementId needs a file where the answer is known:
a Revit 2024 project that has both A-shaped records and an IFC export
from Revit with `Tag` populated, so that a bounding box can be matched
to an ElementId and the matching bytes searched for. The Snowdon files
have the first and not the second.

## 7. Addendum: the ElementId is in the reference list (oracle, 2026-09-22)

An oracle turned up after §1–§6 were written. `bldrs-ai/test-models`
(`ifc/autodesk/snowdon/`) holds the architectural `.rvt`, byte-identical
to the one above (sha256 `33271010…`), next to an IFC4 export of it:

| file | sha256 | size | writer |
|---|---|---:|---|
| `Snowdon Towers Sample Architectural_IFC4.ifc` | `ecfcb04e3e818090cf818873fe76fba8b447742bbf4d19dad6d590f09cbb985a` | 83,153,231 | `Autodesk Revit 24.0.20.20 (ENG) - IFC 24.0.20.20`, `ReferenceView_V1.2` |

That repository declares no licence, so the file is used for measurement
only, kept beside the `.rvt` under the gitignored `_corpus_candidates/`, and
never committed. Every exported element carries its Revit ElementId as
`Tag`: 1,078 `IfcWall`, 132 `IfcDoor`, 68 `IfcWindow`, 227 `IfcSlab`,
118 `IfcColumn`, 60 `IfcCurtainWall`, 462 `IfcPlate`, 1,625 `IfcMember`.

**Placing the two in one frame.** The export uses shared coordinates.
World boxes from IfcOpenShell 0.8.5 (`use-world-coords`, converted to
feet) against the record boxes of the five decodable walls whose ElementId
is also a `Tag` give a rigid transform: rotation 26.0156°, translation
(1,370,151.83, 258,247.98) ft, and z + 780.5 ft. The five walls sit at two
positions, so the fit is exact rather than over-determined. The match rate
below is what validates it.

**Matching.** Each class-A wall frame's box was transformed and compared
with every `IfcWall` box, requiring a centre within 1 ft and bottom and
top faces within 0.6 ft:

| class-A wall frames | one match | ambiguous | none | distinct ElementIds |
|---:|---:|---:|---:|---:|
| 1,216 | 1,077 | 26 | 113 | 1,044 |

A single element can own more than one frame (RE-26), which is why 1,077
frames name 1,044 ids.

**Where the id is.** In **1,044 of the 1,077** matched frames (97 %), the
matched ElementId is one slot of the frame's own counted reference list at
`+0x88`. It does not appear anywhere else within ±4 KB of the frame, nor in
the second counted list. All 1,044 lists are strictly ascending, and every
id after the own one is larger. So §4.2's "last slot" is the special case of
an element that is the newest id in its own list, which is every decodable
record on `2024_Core_Interior.rvt` and on these files. Across the 1,044,
the own id sits 1 to 11+ slots from the end (2nd from the end 177 times,
3rd 149, last 137).

**Which slot: ruled out so far.**

1. A frame word equal to a field of the own id's `Global/ElemTable` row
   (u32 slots at `+0x18`, `+0x1c`, `+0x20`). The best, the frame's u16 at
   `−8` against slot `+0x20`, agrees on 388 of 1,028 and is unique within
   the list on only 124.
2. The frame's 16-bit words read as `Global/ElemTable` row indices, with
   base 0 or 1: no hit at any offset from −64 to `+0x12`.
3. "The first slot that is not a context id", where a context id is one
   that appears in the lists of at least K frames: at most 196 of 1,044
   correct, for K = 10.
4. A byte or 16-bit word anywhere from −32 to `+0x88` holding the own
   slot's index (0- or 1-based, from either end): the best offset agrees on
   208 of 1,044, which is chance level for such small numbers. That covers
   the unattributed `+0x46` / `+0x4a` fields (#223).

So a class-A frame is not missing its ElementId. It is missing the byte
rule that says which of its references is itself. Until that rule is
found and measured, the decode stays fail-closed: the Tags that locate the
id here come from Revit's export, which a reader of an arbitrary file does
not have. This result is tracked in #295.
