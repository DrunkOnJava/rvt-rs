# RE-31 — `Global/ElemTable` records name an owner element in the field the layout detector anchors on

**Issue:** #152 (`Refs` RE-152, RE-30)
**Date:** 2026-09-22
**Result:** **positive** on the field: it is an ElementId, set on most
records, and the element's own partition record corroborates it.
**Partial** on its meaning: the relationship it encodes is attributed by
category for the four largest groups and not claimed beyond them.
**Corpus:**

| file | release | sha256 | layout |
|---|---|---|---|
| `2024_Core_Interior.rvt` | 2024 | `c805df445d613b408e37337765572021265e3f5dfdc7d1fa53b22ba1600b8014` | 40 B |
| `Revit_IFC5_Einhoven.rvt` | 2023 | magnetar-io, MIT | 28 B |
| `Snowdon Towers Sample Architectural.rvt` | 2024 | `33271010919acb2a0d0d040851393a511d86ea7fd9a1f4c054797ae03e5e44a1` | 40 B (local only) |

**Category names:** Autodesk's published `BuiltInCategory` values as
listed in the public API stubs (`OST_CurtainWallPanels` −2000170,
`OST_CurtainWallMullions` −2000171, `OST_CurtainGridsWall` −2000321,
`OST_IOSModelGroups` −2000095, `OST_SketchLines` −2000045).

-----

## 1. The field

`detect_layout` anchors on a run of `0xFF` bytes in each record: four at
`+0` on the 28-byte layout, eight at `+4` on the 40-byte layout. RE-30
found that the run is missing on some records. Read as an integer of the
run's width, the field holds an ElementId whenever it is not all `0xFF`:

| file | records | field set | set values declared in the same table | id density |
|---|---:|---:|---:|---:|
| Einhoven (28 B, `u32` at `+0`) | 2,614 | 338 | 338 | 0.301 |
| Core Interior (40 B, `u64` at `+4`) | 26,425 | 22,368 | 22,368 | 0.198 |
| Snowdon architectural (40 B) | 47,233 | 29,177 | 29,131 | 0.019 |

*Id density* is the declared ids divided by the span from the smallest to
the largest, which is the rate at which an arbitrary in-range number would
also be declared. RE-152 noted that interior words of the 40-byte record
coincide with declared ids at 88–92 % and set the observation aside for
want of an independent oracle, because the id space is dense. On Snowdon
the space is not dense (1.9 %), and the field is still declared 99.8 % of
the time.

## 2. The oracle: the element's own partition record

The partition element record (RE-21) of the same ElementId carries a
counted reference list at `+0x88`. It is an independent stream, written
by a different part of the file. For every framed record with a
declared ElementId:

| file | records | field set | value in the element's own reference list | equals `+0x32` container |
|---|---:|---:|---:|---:|
| Core Interior | 23,620 | 21,848 | 19,015 (87 %) | 24 |
| Snowdon architectural | 2,092 | 1,648 | 1,527 (93 %) | 0 |

A reference list holds a few dozen of the file's tens of thousands of
ids, so a coincidental hit rate would be well under 1 %. The field is
therefore a reference the element itself records. It is not the `+0x32`
container, which matches only 24 times.

## 3. What it points at

Each pair below takes the category and placement kind of the record
holding the field and of the record the field names, on Core Interior
(counts ≥ 50):

| holder | named element | count |
|---|---|---:|
| `OST_CurtainWallMullions`, placed | `OST_Walls`, placed | 6,142 |
| `OST_CurtainGridsWall`, placed | `OST_Walls`, placed | 5,104 |
| `OST_CurtainWallPanels`, placed | `OST_Walls`, placed | 4,743 |
| `OST_SketchLines`, placed | category −1, symbol | 2,944 |
| `OST_Walls`, placed | `OST_Walls`, placed | 697 |
| `OST_CurtainWallPanels`, placed | `OST_IOSModelGroups`, placed | 687 |
| `OST_Walls`, placed | `OST_IOSModelGroups`, placed | 143 |
| category −1, symbol | category −1, symbol | 128 |
| `OST_Floors`, placed | `OST_Floors`, placed | 124 |
| `OST_Columns`, placed | `OST_IOSModelGroups`, placed | 116 |
| `OST_Rooms` | `OST_IOSModelGroups`, placed | 158 |
| `OST_Doors`, placed | `OST_IOSModelGroups`, placed | 53 |

Three readings follow directly from the public category names:

- **Curtain wall parts name their curtain wall.** 15,989 mullions,
  grids and panels name an `OST_Walls` record. The public API exposes
  the same relation for panels as the panel's owner wall.
- **Sketch lines name their sketch.** 2,944 `OST_SketchLines` records
  name an uncategorised symbol record. RE-25 found that the same lines'
  second reference list names the slab the sketch belongs to, which is
  consistent with a line → sketch → slab chain.
- **Grouped elements name their model group.** Columns, rooms, doors,
  walls and curtain panels name an `OST_IOSModelGroups` record. A panel
  inside a grouped curtain wall names the group, not the wall.

It is **not claimed** what `OST_Walls` → `OST_Walls` (697),
`OST_Floors` → `OST_Floors` (124) or symbol → symbol (128) mean, or what
Revit calls the field. It is exposed as `ElemRecord::owner_id`, and its
documentation says only what is measured here.

## 4. What this changes

- `elem_table::ElemRecord::owner_id` reads the field (`None` when it is
  all `0xFF`, and always on the implicit family layout).
  `elem_table::read_layout` returns the detected layout.
- `rvt-elem-table` prints the real layout ("8-byte owner field at record
  +4") instead of sniffing the first record for `0xFF`, which failed on
  any file whose first record names an owner. It also prints how many
  records name an owner and how many of those are declared. `--json`
  adds `layout`, `records_with_owner` and each record's `owner_id`.
- `tests/elem_table_corpus.rs` pins 338 / 2,614 on Einhoven and
  22,368 / 26,425 on Core Interior, each set value a declared ElementId.
- #152's hypothesis of an ownership reference in the table body is
  **supported, at a different field** from the trailing word RE-152
  falsified.

Nothing in the IFC export uses the field yet. Revit's slim export of
Core Interior writes no `IfcCurtainWall`, `IfcPlate`, `IfcMember` or
`IfcGroup`, so there is no reference output to hold a curtain-wall
aggregation or a group assignment to.
