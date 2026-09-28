# RE-101 — A room's partition stores its solid, and its wall faces give the outline

**Date:** 2026-09-28
**Issues:** #90 (room boundary polygons); overturns the negative in RE-29 §6
**Result:**
- A room's partition holds the room's solid as planar faces, after a header naming the room:

  ```text
  u64            the room's ElementId
  04 90 08 00    the solid's tag
  ...            the solid's box, twice, then its topology
  ```

  and then its faces, each within 0x200 bytes of the one before (174 bytes on, or 206 where the face carries more; other records sit between some of them):

  ```text
  +0    04 00 08 00   face tag
  +20   f64 × 4       u min, v min, u max, v max, feet
  +85   01
  +86   f64 × 3       origin, model feet
  +110  f64 × 3       unit u direction
  +134  f64 × 3       unit v direction
  ```

- A wall face of the room is vertical: one direction points up or down and the other is horizontal, and the face reaches down to the room's floor. Its plan edge runs from `origin + min · h` to `origin + max · h` along the horizontal direction `h`.
  - The wall faces' edges close into the room's outline, with a loop of their own around each column or shaft the room surrounds.
  - A vertical face that stops short of the floor is a step in the ceiling, inside the outline.
- The copy read is the one in the partition of the room record rvt-rs exports. It is kept only where its plan extent equals that record's box within 0.001 ft.
- RE-29 searched the partitions for each room's polygon as runs of packed vertices and found none. That was right: the vertices are not stored. The faces they bound are.

| room areas within 0.1% | main | RE-101 |
|---|---:|---:|
| Core Interior, against Revit's IFC4 export (116 rooms) | 76 | 116 |
| RE1 Architecture (Revit 2025), against Revit's IFC4 export (11 rooms) | 11 | 11 |
| Snowdon Towers, against the VIM export of its 2027 edition (54 rooms) | 10 | 45 |

-----

## 1. The solid

Room 25967 on `2024_Core_Interior.rvt` ("14 Nested 1") is U-shaped. Its record box spans 31.5 × 15.5 ft (488.25 ft²), and Revit's export gives it 248.25 ft².

- In Partitions/51, `6f 65 00 00 00 00 00 00 04 90 08 00` opens its solid at 0x26c9bc.
- Ten faces follow from 0x26d83e, 174 bytes apart:
  - the floor and ceiling, both horizontal;
  - eight walls, each with u = (0, 0, −1) and a height of 8 ft.
- Their v directions and extents, in order, are 5.5 ft −x, 12 ft −y, 20 ft −x, 12 ft +y, 6 ft −x, 15.5 ft −y, 31.5 ft +x and 15.5 ft +y.
- From (82.917, 49.25) that walk closes on itself and encloses 248.25 ft².

A face's origin is its plane's, not a point on the face. On Snowdon room 826593 the walls' origins sit at z 70.8 above a floor at −16.9, and the faces run 1.25 to 87.7 ft down from there.

## 2. Which copy

Core Interior writes most rooms' records and solids in two or three partitions, and the copies can disagree:
- Room 20832's record in Partitions/46 spans y 70.25 to 80.75, and its solid there encloses 86.625 ft².
- In Partitions/59 both stop at 80.6667, which encloses 85.9375 ft², Revit's area.

rvt-rs already exports the room from the Partitions/59 record, so the solid is read from the exported record's own partition. Every Partitions/59 solid closes on its record's box, and all 116 give Revit's area.

Room 25945's copies all have the same extent but not the same shape, which is why the partition decides and not the box alone.

## 3. What does not close

The outline is kept only where the loops close and span the room's record box within 0.001 ft. Otherwise the room keeps its box.

- **RE1 Architecture room 355008.** Two walls stand on lines 3e-5 ft apart, and the room's faces are cut from both with no face across the step. Wall ends within 1e-4 ft of each other are joined.
- **Seven Snowdon rooms keep their box** (828537, 828548, 830902, 830912, 2181757, 2181762, 2285535): their wall faces do not close at the floor. On 830902, a 0.005 ft face between two walls stops 18 ft above the floor, and records that are not planar faces sit among the walls. The other six were not taken further.

## 4. Measured

### Core Interior against Revit's IFC4 export

`tools/re/plan_profiles_vs_ifc.py --class IfcSpace` now also measures the outline: the largest distance from a vertex of either file's top surface to the other's outline, both ways. Revit splits an edge wherever the element bounding it changes. Corridor 55780 has 4 corners in rvt-rs and 11 points in Revit's export, 7 of them on straight edges.

| Core Interior, 116 rooms | main | RE-101 |
|---|---:|---:|
| area within 0.1% | 76 | 116 |
| outline within 0.01 ft | 76 | 116 |
| vertices within 0.01 ft | 40 | 42 |

### RE1 Architecture against Revit's IFC4 export

All 11 rooms keep their area and outline, and one more matches Revit's vertices (8 → 9 within 0.01 ft).

### Snowdon Towers against the VIM

Snowdon's IFC4 export holds no IfcSpace. `vimaec/vim-hackathon`'s `Snowdon.r2027.vim` (MIT repository, Autodesk's model content, local only) does. It is a later edition of the same model, and its `Vim.Room` table gives each room's area. `tools/re/room_areas_vs_vim.py`:

- **Within 0.1%:** 45 of 54 rooms (10 on main).
- **Two more differ from the VIM by 0.5% and 0.25%** (828534, 829817). Their rvt-rs areas (1484.790 and 2424.686 ft²) are stored verbatim in the 2024 file itself, in the area cache described below, so the difference is the later edition's.
- **The other seven** are the rooms in §3 that keep their box.

### A second check inside the file

Revit keeps each room's computed area in a cache, `u64 · ff×8 · u32 0 · i32 · f64 area`, several times over. Its leading id is not the room's, so it is not read. As a check, each non-rectangular room's outline area was searched for as an f64 in its own partition:

| file | found | not found |
|---|---:|---:|
| Core Interior | 40 of 40 | 0 |
| Snowdon Towers | 22 | 16 |

The box area was found for none of them. On Snowdon, the VIM confirms all 16 that were not found.

### Other files

Revit_IFC5_Einhoven, Snowdon Towers structural, the three RE1 MEP models, Projeto1, `teste_export_2025`, Exemplo_data, modelo_bim and the MIT house (2024, 2025) are byte-identical to main's apart from the timestamp.

Core Interior's witness observations change: 116 room rectangles become arbitrary closed profiles, and each room gains three profile properties. Both artifacts replay PASS with three matches.

## 5. End-to-end measurement

| file | sha256 |
|---|---|
| 2024_Core_Interior.rvt | `c805df445d613b408e37337765572021265e3f5dfdc7d1fa53b22ba1600b8014` |
| 2024_Core_Interior_slim.ifc | `bfdf36ffb0bb768f3409d818403990e64d4c262c6780603be87f8077387ad86d` |
| RE1-Architecture.rvt | `4a78bdd68e7f4dd7806060db07c222da9a810e20b3f303c17a443294b91c8ce4` |
| RE1-Architecture.ifc | `a9b5d36677aa6a8bb91b77d8bc354ed9028a7ca3e9e2491a47ba14cfd8e26200` |
| Snowdon Towers Sample Architectural.rvt (local only) | `33271010919acb2a0d0d040851393a511d86ea7fd9a1f4c054797ae03e5e44a1` |
| Snowdon.r2027.vim (local only) | `3e7de6364b4592f2281f7453053f7fa7b9f5ac0e703d41ff0cdd5c278eed387e` |

```bash
cargo run --profile ci --example probe_re101_room_solids -- 2024_Core_Interior.rvt
./target/ci/rvt-ifc 2024_Core_Interior.rvt -o core.ifc
python3 tools/re/plan_profiles_vs_ifc.py core.ifc 2024_Core_Interior_slim.ifc --class IfcSpace
./target/ci/rvt-ifc "Snowdon Towers Sample Architectural.rvt" -o snowdon.ifc
python3 tools/re/room_areas_vs_vim.py snowdon.ifc Snowdon.r2027.vim --list 10
```
