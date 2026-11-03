# RE-316 — Rooms whose stored solid the walk did not close

**Date:** 2026-11-03
**Issues:** #1260 (rooms first); follows RE-101 and RE-102 (a room's outline from its stored solid's faces), RE-313 (every `IfcSpace` scored against Revit's export) and RE-314
**Artefacts:** the BIM4LCA office as built and at building permit (Revit 2024) with Revit 24.0.5's IFC4 exports; Autodesk's `rac_advanced`, `rac_basic`, `rme_basic`, `rme_advanced` and Snowdon Towers sample projects (no Revit export of them is in CI); Core Interior and RE1 Architecture with Revit's exports; the R2025 models and the held-out models (counts only), all as Measure fetches them
**Probes:** `probes/examples/probe_re316_room_solid_breaks.rs`, `probe_re316_room_native_faces.rs` and `probe_re316_solid_vertices.rs` (the first finds what breaks; the other two are measured negatives, §1), `probe_re316_solid_extent.rs`, `probe_re316_room_closure.rs` and `probe_re316_hostile_solid.rs`
**Runs:** Measure [41670081379](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41670081379) and [41673400892](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41673400892) (what breaks), [41673755000](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41673755000) (the native record), [41676245766](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41676245766) (the topology before the first face), [41707078214](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41707078214) (where a solid ends, §2), [41710248227](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41710248227) (the rule, with the space scorer and the crafted solids), and the change against `main`, unfiltered, [41712392709](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41712392709) (§3)
**Result:** positive on the office, and the 323% that round 3 of the review found is the scorer's, not the outline's. Of the 13 rooms of the office as built that kept their record box, all 13 take an outline: 9 are `stored_solid_faces`, equal to Revit's area to 0.1% by every metric (their boxes were 3% to 67% over it), and 4 are `stored_solid_approximate`, 0.45% to 0.68% under Revit's floor area (1671995, 1820322, 1832504, 2066902). At building permit 11 of the 12 take an outline, all `stored_solid_approximate`: 10 equal to Revit's floor area to 0.1% and room 1671995 0.45% under; room 3095831 keeps its box. Room 1671995 is not 323% over Revit's area: that is the area of Revit's upward faces, a ceiling Revit's export draws mostly off the horizontal, 1218.33 ft² in 9 flat patches; Revit's floor under it is 5177.37 ft² and ours 5154.12 (§3). None of the four guards the review proposed separates it from the rooms that agree (§3), so none is applied; the label `stored_solid_approximate` says what the file cannot settle.

## 1. Why the solids did not close

RE-101 reads a room's stored solid as a run of planar faces, each within 0x200 bytes of the one before, and joins the plan edges of the faces that reach the room's floor. `probe_re316_room_solid_breaks` printed, for every room that kept its box, the faces of the walk, the faces beyond it and up to the next solid's tag, and the vertices that had fewer or more than two edges. Six things stop the walk or the loops:

| Cause | Office as built | Office at permit | Autodesk samples |
|---|---|---|---|
| **A record longer than 0x200 bytes sits between two faces.** The walk stops there; the faces after it are the room's own (a 626 byte record in room 1422302) | 1422302, 1444351, 1736726, 1832504 | 1436086, 1480292, 1743273, 1743715 | |
| **Two walls stand on lines 0.0007 to 0.0015 ft apart** and the room's faces leave a step with no face across it; the ends are further apart than the 1e-4 ft RE-101 joins (RE1 room 355008 had 3e-5) | 1383998, 1493531, 1681182, 1700966, 1736690, 2297732 | | |
| **A column overlaps a wall.** A face's box is its plane's, not its outline: a wall's face runs the whole wall and the column's footprint overlaps it, so a loop of the column is partly inside the wall's loop and RE-101 refuses it | 1820322, 1832504 (also), 2066902, 1671995 | 1736676, 1736690, 1736720, 1743273, 1743279, 1743309, 1743715 (also), 2302311, 1671995 | |
| **A solid longer than the 64 KB window**: room 1681182 has 115 faces and the last is 84 KB in | 1681182 | | |
| **Round columns.** A face whose record is 44 bytes longer than a plane's (202 bytes against 174 on 2024) is a cylinder's: the axis (a unit vector at right angles to the face's u and v) at +158, the radius at +182, and a uv box that is an angle and a distance along the axis. `face_at` read it as a plane and `kind()` calls it horizontal, so its arc was not an edge and the loops stayed open | | | `rac_advanced`, 6 of 91 rooms in every release 2016 to 2023: room 177330's arcs, worked out from the radius 0.4921 ft and the angles 0.2007 and 6.0825, end at (41.9941, -50.1639) and (41.9941, -50.3601), the ends of the planar necks between them and the wall, to the four decimals the probe prints |
| **Stepped floors and ceilings.** A room whose floor is raised (a 0.4921 ft step) has walls that start at two heights, a riser between them, and soffits that stop short of the ceiling; the walls that reach the floor do not enclose it | | | `rme_basic`, `rme_advanced`: 49 of 50 |

Two readings were measured and are **negative**:

- **The room's native record does not hold the solid** (`probe_re316_room_native_faces`, run 41673755000). On all 13 office rooms, 50 `rme` rooms and 6 `rac_advanced` rooms it holds two `Face` objects, the floor and ceiling planes, with no trim loop, and no `Edge` or `EdgeLoop` object.
- **The topology before the first face holds no vertices** (`probe_re316_solid_vertices`, run 41676245766). Between the header (`01 00 00 00 03 00 00 00` and the solid's box twice) and the first face (4,788 to 64,547 bytes later) the solid lists its objects as `u16 class · u32 token` pointers after a counted list tag (`04 c0 08 00 · u32 count`); no run of three `f64` that is a point of the room's box appears in it other than the two boxes. The list's count is what bounds the solid (§2).

## 2. The rule

**The walk comes first, per room.** A room none of whose solids the walk closes on the record box is the only room read again: `attach_room_outlines` calls the RE-101 reader over all of the room's solids, as before, and only if it gives nothing resolves them (`resolved_outline_of_solids`). A room RE-101 keeps is therefore read exactly as it was, with its `NetVolume` marker, whatever a second solid of the room holds. The cap on the faces the walk reads (512, the largest solid measured lists 136) is the one change to it.

**Where a solid ends.** The span from the walk's stop to the next room solid's tag is not the solid: it runs on into the faces of other elements, and the first version of this change read them (113 of room 1671995's 327 faces lay outside its box, and a foreign face inside the box would have passed). A solid lists its objects after its two boxes, `04 c0 08 00 · u32 count · u32 4 · count × (u16 class · u32 token)`, and the count includes the faces `face_at` does not read, a solid's floor and ceiling among them. `probe_re316_solid_extent` (run 41707078214: the office both files, Core Interior, RE1, Autodesk's 2019 and 2024 samples, 1,597 solids, 1,328 of them closing under the walk):

- the list is present on all 1,597 with the 4 after the count, the count is never below the faces the walk reads, and is at most 136;
- the count minus the walk's faces is 0 on 472 of the solids the walk closes, 1 on 456, 2 on 360, 3 on 15, 4 on 24 and 12 on one;
- on the 47 solids whose count equals the walk's faces and which have a valid face after the walk, that face is another element's, 6,826 bytes or more on (the nearest is the office as built's, then Core Interior's at 6,858);
- the faces that are a solid's own beyond the walk's 0x200 byte gap lie 592 to 732 bytes on (the office and Autodesk's samples; 908 and 1,406 on two of Snowdon Towers' unresolved rooms).

So the span is the faces from the first, each within 0x400 bytes of the one before (`SPAN_FACE_GAP`), and no more than the list counts (`solid_object_count`). Where the count equals the walk's faces nothing is added. Where it is larger, at most the difference could be another element's, and only if it follows within 1,024 bytes of a solid's last face: no solid measured does that, but none is measured with a difference above 4 and a face within 6,826 bytes. That is why a span-read outline is not `stored_solid` (below). The rme room 576007, which has no Revit export, has a count of 21 and 20 faces in the walk and the span alike: its outline is its own faces'.

**From faces to an outline.** The wall faces that reach the floor give plan edges, ends within 0.0025 ft joined: the steps measured are 0.0007 to 0.0015 ft and the shortest real edge 0.0052 ft; the value is chosen from these, not copied, since revit-ifc takes the short-curve tolerance from the Revit API at run time (`IFCProject.cs` sets it from `application.ShortCurveTolerance`) and has no constant. Two readings, in this order, each spanning the record box to 0.001 ft as RE-101's does:

- **`stored_solid_faces`**: the edges close as the walk of RE-101 reads them (`plan_profile_from_lines`: every end shared by two edges, one outer loop, the loops inside it nested) and no edge crosses or overlaps another. A face of another element would have to close a loop of its own, nested and clear of every wall, inside the box and inside the gap and the count above, to be read.
- **`stored_solid_approximate`**: otherwise the outline is the boundary of the region the edges wind around (`region_boundary`). Each plan edge is directed so the room is on its left, which the face's own frame already does: on all 221 and 222 rooms of the office the walk keeps, Core Interior's 116, RE1's 11 and the 14 to 85 each the Autodesk samples keep, the directed edges join head to tail (run 41710248227). `region_boundary` cuts every segment where another crosses or touches it, reads the winding number to either side of each piece, keeps the pieces with the room on one side only, and gives up unless every point is left by one piece and entered by one. A column whose loop lies partly inside a wall's takes its footprint off the room where they overlap and adds nothing outside it. The outline may also fall up to 0.02 ft short of the box on a side, which Griffin chose (2026-11-03): a column's back stands 0.0164 ft behind the wall's face on three office rooms.

The first version labelled every span-read room `stored_solid_faces` whatever its edges did; the split above is the narrowing the standing rule asks for.

**What a resolved room says.** `SpaceOutlineSource`:

| Value | Read | Area, against Revit's floor area (downward faces) |
|---|---|---|
| `stored_solid` | RE-101's walk closed on the box | as RE-313: 221 of 221 as built and 222 of 222 at permit within 0.1% (§3) |
| `stored_solid_faces` | the span's edges close as the walk reads them | within 0.1% on 9 of 9 (office as built) |
| `stored_solid_approximate` | the boundary of a region, a column taken off, or up to 0.02 ft short of the box | within 0.1% on 10 of 15 room scores (4 as built, 11 at permit), 0.45% to 0.68% under on the other 5 (four rooms; 1671995 in both files): Revit's export does not take the column off |

The exports count them (`exported.resolved_outline_spaces`, `exported.approximate_outline_spaces`; `rvt-ifc` prints a line for each), and the room schedule gives an area for `stored_solid` and `stored_solid_faces` rooms and none for approximate ones. A solid with a cylinder is not resolved, and a resolved outline carries no prism marker, so no `NetVolume`.

**Bounded work.** A partition is untrusted input. The walk reads at most 512 faces (a solid with more is left to its box); the span at most the list's count, never above 512; `region_boundary` takes at most 512 segments and gives up before it builds more than 4,096 pieces, however often they cross. `probe_re316_hostile_solid` builds crafted solids in memory and times both readers (run 41710248227, GitHub's runner):

| Solid | Walk | Resolve |
|---|---:|---:|
| grid 10 x 10 (20 faces) | 0.0 ms | 0.1 ms |
| grid 40 x 40 (80 faces, 1,600 crossings) | 0.0 ms | 9.1 ms |
| grid 100 x 100 (200 faces) | 0.1 ms | 0.4 ms |
| grid 256 x 256 (512 faces, 65,536 crossings) | 0.5 ms | 0.8 ms |
| 512 faces overlapping on one line | 0.5 ms | 0.0 ms |
| 64 concentric squares (256 faces) | 0.2 ms | 0.8 ms |
| a list counting 100,000 | 0.0 ms | 0.0 ms |
| no list | 0.2 ms | 0.0 ms |
| 5,000 faces | 0.1 ms | 6.1 ms |

Every one gives its outline or none and takes under 10 ms. The review measured the first version, which took the faces up to the next solid's tag and cut them without a bound on the pieces, at about 10 s on about 500 crafted faces.

## 3. Measured

### The office, against Revit's own IFC4 export

As built, the rooms that kept their box (ft²). Revit's area is its floor (the faces of its mesh that point down), which is the room's plan; its upward faces are in §3 below:

| Room | Box | Revit | Outline | `SpaceOutlineSource` | Why the edges did not close as the walk reads them |
|---|---:|---:|---:|---|---|
| 1383998 | 1708.89 | 1020.46 | 1020.46 | `stored_solid_faces` | a step, 0 holes |
| 1422302 | 362.30 | 291.54 | 291.54 | `stored_solid_faces` | a record between faces, 1 hole |
| 1444351 | 362.31 | 291.56 | 291.56 | `stored_solid_faces` | a record between faces, 1 hole |
| 1493531 | 362.31 | 291.56 | 291.55 | `stored_solid_faces` | a step, 1 hole |
| 1681182 | 5963.96 | 3818.36 | 3818.36 | `stored_solid_faces` | a step, a long solid, 1 hole |
| 1700966 | 4774.18 | 3259.86 | 3259.85 | `stored_solid_faces` | a step, 1 hole |
| 1736690 | 2846.73 | 2049.24 | 2049.25 | `stored_solid_faces` | a step, 3 holes |
| 1736726 | 367.14 | 293.43 | 293.43 | `stored_solid_faces` | a record between faces, 1 hole |
| 2297732 | 135.82 | 131.76 | 131.76 | `stored_solid_faces` | a step |
| 1671995 | 5228.48 | 5177.37 | 5154.12 | `stored_solid_approximate` | a column overlaps a wall: 6 crossing pairs |
| 1820322 | 1882.11 | 1877.97 | 1865.97 | `stored_solid_approximate` | 9 crossing pairs |
| 1832504 | 465.18 | 440.31 | 437.31 | `stored_solid_approximate` | 2 crossing pairs |
| 2066902 | 1542.77 | 638.23 | 635.23 | `stored_solid_approximate` | 3 crossing pairs |

The first nine have no edge crossing or overlapping another (`probe_re316_room_closure`, run 41728640203) and Revit's area to 0.1% by the upward faces, the floor and every face. The four approximate rooms are 0.45% (1671995: 23.25 ft²), 0.64% (1820322: 12.00), 0.68% (1832504: 3.00) and 0.47% (2066902: 3.00) under Revit's floor area, the 3.0 ft² being a column's 3.001. Revit's export does not take those columns off the room, where it takes off the 1.722 ft² columns of eight rooms at permit.

At building permit 11 rooms are `stored_solid_approximate`: 1671995 (6 crossing pairs; 5153.60 against Revit's floor of 5176.85, 0.45% under); 1736676, 1736690, 1736720, 1743273, 1743279, 1743309, 1743715 and 2302311 (1 to 3 pairs of edges overlapping along a line, no crossing: the columns); and 1436086 and 1480292 (neither crossing nor overlap: an end lies on another edge). Ten have Revit's floor area to 0.1%: their boxes were 2.6% to 58% over it. Room 3095831 keeps its box: 14 ends at y = 35.269 ft, in pairs 0.656 and 1.312 ft apart down a 100 ft wall, are not joined, since the columns along it share the wall's line.

Rooms per outline source (the export's own counts, [41728355227](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41728355227); the scorer pairs 276 of the as-built file's 308 spaces and 278 of the permit's):

| | `stored_solid` | `stored_solid_faces` | `stored_solid_approximate` | `record_box` | family instances, `record_box` |
|---|---:|---:|---:|---:|---:|
| Office as built (234 rooms) | 221 | 9 | 4 | 0 | 44 |
| Office at permit (234 rooms) | 222 | 0 | 11 | 1 | 44 |

`space_profiles_vs_ifc` (the existing scorer, which compares the faces of Revit's mesh that point up), `main` against this change:

| | Area within 0.1% | `stored_solid` scored | `stored_solid_faces` scored, area within 0.1% | `stored_solid_approximate` scored, area within 0.1% | Room, record box |
|---|---:|---|---|---|---:|
| Office as built | 221 → 230 | 220, all | 9, 9 | 3, 0 | 12 → 0 |
| Office at permit | 222 → 232 | 222, 221 | 0 | 11, 10 | 12 → 1 |
| Core Interior | 116 | 116 | 0 | 0 | 0 |
| RE1 Architecture | 11 | 11 | 0 | 0 | 0 |

### Room 1671995, and what the scorer's top surface is

Round 3 of the review found room 1671995 at 323% over Revit's area as built and 350% at permit, by this scorer. It compares the plan union of the faces of each mesh that point up, within 25 degrees of vertical (`normal.z > 0.9 · length`). Room 1671995's ceiling is mostly not that flat: Revit's export has 9 flat patches of 1218.33 ft² (1145.89 at permit) and 1 hole in its top surface. `tools/re/space_overlaps_vs_ifc.py` measures each space's plan three ways, in Revit's export and in rvt-rs's (run 41723543204, the office):

| Room 1671995 | upward faces | floor (downward faces) | every face projected |
|---|---:|---:|---:|
| Revit, as built | 1218.33 | 5177.37 | 5178.07 |
| rvt-rs, as built | 5154.12 | 5154.12 | 5154.12 |
| Revit, at permit | 1145.89 | 5176.85 | 5178.38 |
| rvt-rs, at permit | 5153.60 | 5153.60 | 5153.60 |

The room is 5177 ft² in Revit's export, and rvt-rs is 0.45% under: its span's outer loop alone is 5176.72 ft² (run 41673400892) and the stored solid takes off the 6 holes and the notches of the columns that overlap its walls (22.6 ft²), which Revit's floor does not. Its box was 1.0% over. By the floor, over the whole office:

| Area within 0.1% of Revit's | upward faces | floor | every face |
|---|---:|---:|---:|
| `stored_solid`, as built | 220 of 221 | 221 of 221 | 221 of 221 |
| `stored_solid`, at permit | 221 of 222 | 222 of 222 | 222 of 222 |
| `stored_solid_faces`, as built | 9 of 9 | 9 of 9 | 9 of 9 |
| `stored_solid_approximate`, as built | 0 of 4 | 0 of 4 | 0 of 4 |
| `stored_solid_approximate`, at permit | 10 of 11 | 10 of 11 | 10 of 11 |

RE-313's 66% room at permit (2T7gRjEFvBgRaNLZHAfExM, room 2611271, STAIRCASE) is the one `stored_solid` room that the upward faces miss; by the floor it is within 0.1% of Revit's like the other 221. The scorer's top surface reads a room with a sloped or stepped ceiling as a fraction of itself. This change does not alter `space_profiles_vs_ifc` or its CI gates; the floor counts are in the new scorer, which Measure runs on every model, and the held-out models' floor counts are kept by the scrub.

### A guard for 1671995: the four candidates

The review asked for a discriminator that rejects 1671995 (both stages) and keeps every agreeing room, in four candidates. Measured on every room the span reads, and on the baseline of rooms the walk reads:

1. **Wall overlap** (`space_overlaps_vs_ifc`, runs 41722753269 and 41723543204): the area of a room's top surface inside the top surfaces of the walls on its level, each shrunk by 0.02 ft. 1671995 has 1.65 ft² (0.03%) as built and 0.99 ft² (0.02%) at permit. The nine rooms that agree have 0.10% to 7.59% (7.59% is room 2297732, 10.01 ft²) and the 221 rooms the walk reads have 0.1% or more on 43, 1% on 34, 5% on 10 and up to 23.03%. The wall plans of rvt-rs's own export overlap the rooms that agree by more than they overlap 1671995. Not separating. The same test against the outlines of the other rooms on the level (what Revit's rooms cannot do) is 0.00% on every room of every source in both files.
2. **Shell closure**: a closed shell needs the floor and ceiling faces, and `face_at` reads one or more on 3 of the 13 rooms as built (1383998: 1, 1671995: 1, 2297732: 2; `probe_re316_solid_extent`, run 41707078214). In plan form, every end shared by two edges, it holds on all 24 resolved rooms or they would not close. Not testable, and not separating where it is.
3. **Volume**: the divergence-theorem volume needs the same faces. Not testable on 10 of the 13.
4. **Containment**: every outline vertex inside the record box to 0.01 ft holds on all 24 (an outline spans its box to 0.001 ft, or up to 0.02 ft short of it). Not separating.

None works, so no guard is applied. The reason none can: 1671995's outline is not wrong. The residual risk the review named, a face of another element within 1,024 bytes after the last listed face, is not closed by candidate 1 or 2 either: it would need a closed loop of the foreign faces inside the box and clear of every wall to be read as `stored_solid_faces`, and any foreign face that crosses or overlaps a wall makes the room `stored_solid_approximate`. No solid measured has a face of another element inside 6,826 bytes of its last where the list counts the walk's faces exactly.

### Revit's cached room areas

RE-101 §4 found a cache of each room's computed area in its partition (`u64 · ff×8 · u32 0 · i32 · f64`) and 40 of 40 non-rectangular Core Interior rooms' outline areas in it. `probe_re316_room_closure` repeats the check (run 41710248227). It does not confirm the span areas, and does not contradict them either:

| | Cache holds the outline's area: non-rectangular rooms the walk keeps | exact span outlines | approximate ones |
|---|---:|---:|---:|
| Core Interior | 40 of 40 | | |
| RE1 Architecture | 1 of 2 | | |
| Office as built | 71 of 103 | 0 of 10 | 0 of 3 |
| Office at permit | 20 of 116 | 2 of 11 | |
| `rac_advanced` 2019 | 0 of 21 | | |
| `rac_basic` 2019 | 0 of 7 | | |

(Counted under the first version's labels; the rooms are the same.) For rooms 1422302, 1444351 and 1493531 the cache holds the record box's area (362.3 ft²), not the 291.5 ft² of Revit's export and of the outline. The cache is not there on every release (`rac_advanced`: none of 21 on 2019, 19 of 21 on 2024), so it is not used as evidence.

### The Autodesk samples (no Revit export)

`probe_re316_room_closure` on the 2019 and 2024 saves (runs 41726486862 and 41728640203):

| Project | Rooms | Closed by RE-101 | Resolved now | Box |
|---|---:|---:|---:|---:|
| `rac_basic` | 14 | 14 | 0 | 0 |
| `rac_advanced` | 91 | 85 | 0 | 6 |
| `rme_basic`, `rme_advanced` | 76 | 26 | 1 (`stored_solid_approximate`, room 576007: 1887.2 ft² against its box's 1917.9; 2 crossing and 2 overlapping pairs) | 49 |
| Snowdon Towers (architectural, 2024) | 54 | 47 | 0 | 7 |

### Everything else

Measure unfiltered, `-f base=main -f nordic=true -f r2025=true -f samples=true -f holdout=true` ([41728355227](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41728355227): this change at a00e705d against `main` at 7fd339e2; every scorer including `space_profiles_vs_ifc` and the new `space_overlaps_vs_ifc`). Every scorer's output is identical on every model but five files, and the exports' hashes move on exactly the models with a room that now closes:

- **Office as built and at permit**: the rows above. `exported.bounding_box_spaces` goes from 57 to 44 and from 56 to 45 (the 44 family instances, and room 3095831 at permit); the export counts 9 `stored_solid_faces` and 4 `stored_solid_approximate` as built, and 0 and 11 at permit.
- **`rme_basic` and `rme_advanced`, 12 files over the 6 releases 2015, 2019, 2024, 2025, 2026 and 2027**: one room each (576007, 1887.2 ft² against its box's 1917.9) takes an outline, `stored_solid_approximate`, and box spaces go 50 → 49; no Revit export of them exists.
- **Held out, counts only (BIM Speed's Revit 2021 project, Warmond)**: the one model outside the office whose scorer output changes. Area within 0.1%: 174 → 179 of the 179 paired spaces, off 5 → 0; outline and boundary within 0.01 ft 174 → 179; rooms on their record box 8 → 2 (both within 0.1%: rectangles), `stored_solid_faces` 6 (all within 0.1% and 0.01 ft, by the upward faces and by the floor), `stored_solid` 171 unchanged. Of the 8 rooms that kept their box, 5 were off in area and 6 now take an outline: the sixth was a rectangle already equal to Revit's. Vertices within 0.01 ft go from 127 to 126: that room's outline has vertices the box had not. This model was not read while the rule was found.
- **Core Interior, RE1 Architecture, the R2025 models, Vitoria's spent pair and the other held-out models**: unchanged, so no witness observation moves. `rac_advanced`, `rac_basic` and Snowdon Towers' architectural sample in every release: unchanged.
- **Refused**: no guard is applied, so no room is refused for a guard's reason. The rooms that keep their box are in §4.

## 4. What keeps its box

- **Office room 3095831 (permit)**, above.
- **The six `rac_advanced` rooms** (177330, 214970, 215140, 215177, 216415, 217813): round columns. With their arcs the six close (the probe's `with arcs` column). 43 of the 85 rooms RE-101 keeps in the same file have round columns it ignores, its outline 0.2% to 1.2% larger than the one with the columns taken off (room 177304: 3531.7 ft² against 3521.8). One of those two readings is wrong and no Revit export of the sample says which, so both are left as they are.
- **49 of the 50 `rme` rooms**: stepped floors and ceilings. The walls that reach the ceiling instead of the floor close for 18 of the 49 (run 41710248227; `rme_basic` and `rme_advanced` are the same), 13 of those equal the box and 5 differ from it by 1% to 11%; no Revit export of these rooms exists, and a bulkhead that hangs from the ceiling would be taken for a wall. They stay boxes.
- **Snowdon Towers' seven rooms** (#90: 828537, 828548, 830902, 830912, 2181757, 2181762, 2285535, in Autodesk's 2024 sample): their lists count more objects than the walk reads on three of them (25, 20 and 61 more: 828537, 828548 and 2181762), and no Revit export of the sample is in CI.
- **The 44 family instances written as spaces** are #1262's.

## 5. Not claimed

- That a `stored_solid_faces` or `stored_solid_approximate` outline equals Revit's on a release other than 2024: the evidence is one building, in two states, and one held-out project.
- That the span holds no face of another element: the rule admits at most the list's difference within 1,024 bytes of the last listed face, and neither candidate 1 nor 2 of the review closes that (§3).
- That the area of an approximate room is Revit's: 5 of the 15 office room scores are 0.45% to 0.68% under Revit's floor area, and 10 are equal. Whether Revit takes a column off the room is not in the file.
- That a cylinder's arc is Revit's on any room: `rac_advanced` has no export.
- Any volume of a resolved room (no prism marker is written).

## 6. Reproduce

```bash
gh workflow run measure.yml --ref feat/rooms-unclosed-solids -f ref=b714122a343e659dc2834269fd0c217569069f8d -f base=none -f samples=true -f sample_years="2019" \
  -f nordic=true -f models="nordic-office-as-built nordic-office-permit" -f probe=probe_re316_room_closure -f probe_scorer=- -f scorers=space_overlaps_vs_ifc
gh workflow run measure.yml --ref feat/rooms-unclosed-solids -f ref=7d7c098c8ede1ae8bb55d559a4bfdf270a5e70fc -f base=none -f samples=true -f sample_years="2019 2024" \
  -f nordic=true -f models="nordic-office-* core-interior re1-architecture" \
  -f probe="probe_re316_solid_extent probe_re316_room_closure probe_re316_hostile_solid" -f probe_scorer="- - -" -f scorers=space_profiles_vs_ifc
gh workflow run measure.yml --ref feat/rooms-unclosed-solids -f ref=a00e705d76ca728dbdbc3033df7c68aad94cd69e -f base=main -f nordic=true -f r2025=true -f samples=true -f holdout=true
python3 tools/re/space_overlaps_vs_ifc.py model.ifc revit-export.ifc   # the floor and every-face areas, the walls and rooms a room covers
```
