# RE-316 — Rooms whose stored solid the walk did not close

**Date:** 2026-11-03
**Issues:** #1260 (rooms first); follows RE-101 and RE-102 (a room's outline from its stored solid's faces), RE-313 (every `IfcSpace` scored against Revit's export) and RE-314
**Artefacts:** the BIM4LCA office as built and at building permit (Revit 2024) with Revit 24.0.5's IFC4 exports; Autodesk's `rac_advanced`, `rac_basic`, `rme_basic` and `rme_advanced` sample projects (no Revit export of them is in CI); Core Interior and RE1 Architecture with Revit's exports; the R2025 models and the held-out models (counts only), all as Measure fetches them
**Probes:** `probes/examples/probe_re316_room_solid_breaks.rs`, `probe_re316_room_native_faces.rs`, `probe_re316_solid_vertices.rs` and `probe_re316_room_closure.rs`
**Runs:** Measure [41670081379](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41670081379) and [41673400892](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41673400892) (what breaks), [41673755000](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41673755000) (the native record), [41676245766](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41676245766) (the topology before the first face), [41686457484](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41686457484) (the rule, with the space scorer); the final change against `main`, unfiltered, in §3
**Result:** positive on the office, narrowed. Of the 13 rooms of the office as built whose solids did not close, 12 now take an outline. The scorer pairs 11 of those with Revit's: 9 have Revit's area to 0.1% and Revit's outline to 0.01 ft, and 2 are 0.5% and 0.7% off. At building permit 10 of the 12 take an outline, all 10 with Revit's area to 0.1% and Revit's outline to 0.01 ft. The record box was off Revit's area by up to 329% on these rooms. Room 1671995 (both files) and 3095831 (permit) keep their box, and so do 49 of the 50 `rme` rooms, each for a measured reason (§4).

## 1. Why the solids did not close

RE-101 reads a room's stored solid as a run of planar faces, each within 0x200 bytes of the one before, and joins the plan edges of the faces that reach the room's floor. `probe_re316_room_solid_breaks` printed, for every room that kept its box, the faces of the walk, the faces beyond it and up to the next solid's tag, and the vertices that had fewer or more than two edges. Six things stop the walk or the loops:

| Cause | Office as built | Office at permit | Autodesk samples |
|---|---|---|---|
| **A record longer than 0x200 bytes sits between two faces.** The walk stops there; the faces after it, up to the next solid's tag, are the room's own (a 626 byte record in room 1422302) | 1422302, 1444351, 1736726, 1832504 | 1436086, 1480292, 1743273, 1743715 | |
| **Two walls stand on lines 0.0007 to 0.0015 ft apart** and the room's faces leave a step with no face across it; the ends are further apart than the 1e-4 ft RE-101 joins (RE1 room 355008 had 3e-5) | 1383998, 1493531, 1681182, 1700966, 1736690, 2297732 | | |
| **A column overlaps a wall.** A face's box is its plane's, not its outline: a wall's face runs the whole wall and the column's footprint overlaps it, so a loop of the column is partly inside the wall's loop and RE-101 refuses it | 1820322, 1832504 (also), 2066902 | 1736676, 1736690, 1736720, 1743273, 1743279, 1743309, 1743715 (also), 2302311 | |
| **A solid longer than the 64 KB window**: room 1681182 has 115 faces and the last is 84 KB in | 1681182 | | |
| **Round columns.** A face whose record is 44 bytes longer than a plane's (202 bytes against 174 on 2024) is a cylinder's: the axis (a unit vector at right angles to the face's u and v) at +158, the radius at +182, and a uv box that is an angle and a distance along the axis. `face_at` read it as a plane and `kind()` calls it horizontal, so its arc was not an edge and the loops stayed open | | | `rac_advanced`, 6 of 91 rooms in every release 2016 to 2023: room 177330's arcs, worked out from the radius 0.4921 ft and the angles 0.2007 and 6.0825, end at (41.9941, -50.1639) and (41.9941, -50.3601), the ends of the planar necks between them and the wall, to the four decimals the probe prints |
| **Stepped floors and ceilings.** A room whose floor is raised (a 0.4921 ft step) has walls that start at two heights, a riser between them, and soffits that stop short of the ceiling; the walls that reach the floor do not enclose it | | | `rme_basic`, `rme_advanced`: 49 of 50 |

Two readings were measured and are negative:

- **The room's native record does not hold the solid.** On all 13 office rooms, 50 `rme` rooms and 6 `rac_advanced` rooms it holds two `Face` objects, the floor and ceiling planes, with no trim loop, and no `Edge` or `EdgeLoop` object (`probe_re316_room_native_faces`).
- **The topology before the first face holds no vertices.** Between the header (`01 00 00 00 03 00 00 00` and the solid's box twice) and the first face (4,788 to 64,547 bytes later) the solid lists its objects as `u16 class · u32 token` pointers after a counted list tag (`04 c0 08 00 · u32 count`); no run of three `f64` that is a point of the room's box appears in it other than the two boxes (`probe_re316_solid_vertices`).

## 2. The rule

Where the faces RE-101 walks close on the room's record box, nothing changes. Where they do not, `partition_room_boundaries` now reads every face up to the next solid's tag (`solid_faces_to_next_solid`; none if no tag follows within 4 MiB) and rules the solid out if any vertical face reaches outside the record box, which holds the whole solid (113 of room 1671995's 327 faces do). Each wall face's plan edge is directed so the room is on its left, which the face's own frame already does: on all 221 and 222 rooms of the office RE-101 keeps, on Core Interior's 116, RE1's 11 and the 26 to 85 each of the Autodesk samples keep, the directed edges join head to tail. A round column's cylinder is read as an arc, sampled to 0.0005 ft, run clockwise, and joined to the walls only where its ends meet theirs.

`region_boundary` cuts every segment where another crosses or touches it, reads the winding number to either side of each piece, keeps the pieces with the room on one side only, and gives up unless every point is left by one piece and entered by one. A column that overlaps a wall takes its footprint off the room where they overlap and adds nothing outside; a step of up to 0.0025 ft (Revit's short-curve tolerance is 1/32 inch) joins two ends; and the outline may fall up to 0.02 ft short of the record box where a column's back stands behind the wall's face (0.0164 ft on three office rooms), but never beyond it.

An outline resolved this way carries no prism marker: no room's volume is measured against Revit's for them.

## 3. Measured

### The office, against Revit's own IFC4 export

Room by room, the record box's area, Revit's area, and the outline's area (ft²), as built:

| Room | Box | Revit | Outline | Cause |
|---|---:|---:|---:|---|
| 1383998 | 1708.89 | 1020.46 | 1020.46 | step |
| 1422302 | 362.30 | 291.54 | 291.54 | record between faces |
| 1444351 | 362.31 | 291.56 | 291.56 | record between faces |
| 1493531 | 362.31 | 291.56 | 291.55 | step |
| 1681182 | 5963.96 | 3818.36 | 3818.36 | step, long solid |
| 1700966 | 4774.18 | 3259.86 | 3259.85 | step |
| 1736690 | 2846.73 | 2049.24 | 2049.25 | step |
| 1736726 | 367.14 | 293.43 | 293.43 | record between faces |
| 1820322 | 1882.11 | none (not paired) | 1865.97 | column overlaps wall |
| 1832504 | 465.18 | 440.31 | 437.31 | column overlaps wall |
| 2066902 | 1542.77 | 638.23 | 635.23 | column overlaps wall |
| 2297732 | 135.82 | 131.76 | 131.76 | step |
| 1671995 | 5228.48 | 1218.33 | box | the span runs into other elements |

The two rooms 0.5% and 0.7% off are the ones where the stored solid has a column that overlaps the wall by 0.0164 ft and Revit's export does not take it off: the outline is the solid's, with the 3.0 ft² notch, and the boundary distance to Revit's is the notch's depth, 1.329 ft. On seven rooms at building permit Revit takes the same kind of column off (1.722 ft² each), as the outline does. Room 1820322 is one of the two rooms whose top surface Revit's export gives no area, so it is not scored.

At building permit the 10 rooms 1436086, 1480292, 1736676, 1736690, 1736720, 1743273, 1743279, 1743309, 1743715 and 2302311 take outlines and all 10 have Revit's area to 0.1% (the largest, 1736720, 3265.29 against Revit's 3265.29). Rooms 1671995 (box 5228.48, Revit 1145.89) and 3095831 (box 6642.03, Revit 1864.74) keep their box.

`space_profiles_vs_ifc` (run 41686457484, `main` with this change):

| | Area within 0.1% | within 1% | Room, stored solid | Room, record box |
|---|---:|---:|---|---:|
| Office as built | 221 → 230 | 221 → 232 | 220 → 231 (229 within 0.1%) | 12 → 1 |
| Office at permit | 222 → 232 | 222 → 232 | 222 → 232 (231 within 0.1%) | 12 → 2 |
| Core Interior | 116 | 116 | 116 | 0 |
| RE1 Architecture | 11 | 11 | 11 | 0 |

The one stored-solid room at permit that is off (2T7gRjEFvBgRaNLZHAfExM, 66% in area) is the one RE-313 left unexplained and is unchanged.

### The Autodesk samples (no Revit export)

`probe_re316_room_closure` on the 2019 and 2023 saves (the other releases' counts are in the run summary):

| Project | Rooms | Closed by RE-101 | Closed now | Box |
|---|---:|---:|---:|---:|
| `rac_basic` | 14 | 14 | 14 | 0 |
| `rac_advanced` | 91 | 85 | 91 | 0 |
| `rme_basic`, `rme_advanced` | 76 | 26 | 27 | 49 |

The six `rac_advanced` rooms close because of their cylinders. Their arcs end at the ends of the planar necks, which is a check no guess of the radius would pass, but no Revit export of the sample says their area. 43 of the 85 rooms RE-101 keeps in `rac_advanced` have round columns it ignores: the outline with the columns taken off is 0.2% to 1.2% smaller (room 177304: 3531.7 ft² against 3521.8). They are left as they were.

### Everything else

PENDING: Measure unfiltered against `main` (run 41688293656).

## 4. What keeps its box

- **Office room 1671995, both files**: the span to the next solid's tag holds 327 faces, 113 of which reach outside the room's box. Without that rule the same code gave an outline of 5154 ft² against Revit's 1218 (1146 at permit), spanning the box: a wrong outline. The room's solid is not bounded by its record.
- **Office room 3095831 (permit)**: 14 ends at y = 35.269 ft, in pairs 0.656 and 1.312 ft apart down a 100 ft wall, are not joined; the columns along it share the wall's line.
- **49 of the 50 `rme` rooms**: stepped floors and ceilings. The faces that reach the ceiling instead of the floor close for 33 of the 50, and 24 of those are the box itself, but 9 differ from it (8 by 1% to 11%, and room 575874 at 22%: 5105 ft² against a box of 23323), no Revit export of these rooms exists, and a bulkhead that hangs from the ceiling would be taken for a wall. They stay boxes.
- **The 44 family instances written as spaces** are #1262's.

## 5. Not claimed

- That a resolved outline equals Revit's outline on a release other than 2024: the evidence is one building, in two states. No room of another release is scored.
- That a cylinder's arc is Revit's on any room: `rac_advanced` has no export.
- Any volume of a resolved room (no prism marker is written).
- That 12 of 13 and 10 of 12 will hold on another building: the rules were found on these 25 rooms, and the held-out counts in §3 are the only test on others.

## 6. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=<sha> -f base=none -f samples=true -f sample_years="2019 2023" \
  -f nordic=true -f models="nordic-office-* core-interior re1-architecture" \
  -f probe=probe_re316_room_closure -f probe_scorer=- -f scorers=space_profiles_vs_ifc
gh workflow run measure.yml --ref main -f ref=<sha> -f base=main -f nordic=true -f r2025=true -f samples=true -f holdout=true
```
