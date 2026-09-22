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
**Reference:** none — Autodesk ships no IFC export of these files, so
nothing here is compared element for element.

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
RE-21 fail-closed tests (`partition_element_records::decode_at`):

- **OK** — bbox marker `46 01 ff ff ff ff ab 05` at `+0x50` and a
  `u64` ElementId at `+0x00` declared in `Global/ElemTable`: decodes.
- **A** — the marker is at `+0x50` but `+0x00` is not an ElementId
  (`ffffffff_ffffffff` on 505 of the 1,124 architectural walls,
  `ffffffff_00000006` on 611).
- **undeclared** — the Core Interior shape (`0x059f` at `+0x0c`) with an
  ElementId that `Global/ElemTable` does not declare. RE-26 already found
  Revit leaving superseded frames in place; these are consistent with
  that, and rejecting them is the join doing its job.
- **no marker** — the category bytes occur but no record frame follows.

| file | category | OK | A | undeclared |
|---|---|---:|---:|---:|
| architectural | `OST_Walls` | 6 | 1,124 | 34 |
| architectural | `OST_Floors` | 0 | 177 | 1 |
| architectural | `OST_Doors` | 0 | 225 | 48 |
| architectural | `OST_Windows` | 0 | 144 | 15 |
| architectural | `OST_Rooms` | 0 | 54 | 0 |
| architectural | `OST_Columns` | 0 | 172 | 11 |
| structural | `OST_Walls` | 8 | 46 | 4 |
| structural | `OST_Floors` | 3 | 21 | 0 |
| structural | `OST_Doors` | 7 | 0 | 0 |
| structural | `OST_Windows` | 8 | 1 | 0 |
| structural | `OST_StructuralFraming` | 122 | 967 | 29 |
| structural | `OST_StructuralColumns` | 0 | 94 | 0 |
| structural | `OST_StructuralFoundation` | 0 | 84 | 3 |

The A records are elements, not noise. From `+0x12` to the end of the
bbox they are byte-identical in shape to OK records (category, sentinel
padding, placement kind `0xffffef7f`, `0x0e55` at `+0x4a`, marker), the
1,124 architectural wall frames carry 1,122 distinct bounding boxes with
wall proportions (for example 1.1 ft × 18.9 ft × 24.7 ft), and 1,097 of
them carry a counted reference list whose every slot is a declared
ElementId.

## 4. Where the A-record ElementId is not

Three places were tested and ruled out:

1. **`+0x00`.** By definition of A. The first `0x12` bytes hold `0xFF`
   runs and 16-bit words (`0x03c1`, `0x07eb`, `0x04fe`, `0x066f`) where
   OK records hold the ElementId, a flags word and `0x059f`.
2. **The last reference slot.** On OK records the reference list ends
   with the record's own ElementId (2454523, 2455133, 2455140 on the
   first three architectural walls). On the 1,124 A walls the last slot
   takes only 579 distinct values (2084165 on 105 of them), so it is not
   a per-element id.
3. **A fixed offset before the record.** Scanning `u32` values at every
   even offset from `−512` to `+0x12` across 1,842 A records (walls,
   doors, windows, floors, columns), the best offset holds a declared id
   on 127 of 1,842 — no offset is an ElementId slot.

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
  `IfcFooting`) stay unmapped. The 122 decodable framing records are the
  only instances available and there is no reference export to hold the
  RE-21 instance rule to for that category.

## 6. Next measurement

Locating the A-record ElementId needs a file where the answer is known:
a Revit 2024 project that has both A-shaped records and an IFC export
from Revit with `Tag` populated, so that a bounding box can be matched
to an ElementId and the matching bytes searched for. The Snowdon files
have the first and not the second.
