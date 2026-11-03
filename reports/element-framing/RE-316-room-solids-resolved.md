# RE-316 — Rooms whose stored solid the walk did not close

**Date:** 2026-11-03
**Issues:** #1260 (rooms first); follows RE-101 and RE-102 (a room's outline from its stored solid's faces), RE-313 (every `IfcSpace` scored against Revit's export) and RE-314
**Artefacts:** the BIM4LCA office as built and at building permit (Revit 2024) with Revit 24.0.5's IFC4 exports; Autodesk's `rac_advanced`, `rac_basic`, `rme_basic`, `rme_advanced` and Snowdon Towers sample projects (no Revit export of them is in CI); Core Interior and RE1 Architecture with Revit's exports; the R2025 models and the held-out models (counts only), all as Measure fetches them
**Probes:** `probes/examples/probe_re316_room_solid_breaks.rs`, `probe_re316_room_native_faces.rs` and `probe_re316_solid_vertices.rs` (the first finds what breaks; the other two are measured negatives, §1), `probe_re316_solid_extent.rs`, `probe_re316_room_closure.rs` and `probe_re316_hostile_solid.rs`
**Runs:** Measure [41670081379](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41670081379) and [41673400892](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41673400892) (what breaks), [41673755000](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41673755000) (the native record), [41676245766](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41676245766) (the topology before the first face), [41707078214](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41707078214) (where a solid ends, §2), [41710248227](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41710248227) (the rule, with the space scorer and the crafted solids), and the change against `main`, unfiltered, [41712392709](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41712392709) (§3)
**Result:** positive on the office, with one room that disagrees with Revit's export. Of the 13 rooms of the office as built that kept their record box, all 13 now take an outline: 10 are labelled `stored_solid_faces` and 3 `stored_solid_approximate`. Nine of the 10 have Revit's area to 0.1% and Revit's boundary to 0.01 ft (their boxes were 3% to 67% over Revit's area); the tenth, room 1671995, is 323% over Revit's area, where its box was 329% over. The two approximate rooms Revit's export pairs are 0.68% and 0.47% under its area, the third is not paired. At building permit 11 of the 12 take an outline, all `stored_solid_faces`: 10 have Revit's area to 0.1% and Revit's boundary to 0.01 ft, and room 1671995 is 350% over (its box, 356%). Room 3095831 keeps its box. No room carries `stored_solid`, the label of RE-101's walk, that the walk did not close.

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

So the span is the faces from the first, each within 0x400 bytes of the one before (`SPAN_FACE_GAP`), and no more than the list counts (`solid_object_count`). Where the count equals the walk's faces nothing is added. Where it is larger, at most the difference could be another element's, and only if it follows within 1,024 bytes of a solid's last face: no solid measured does that, but none is measured with a difference above 4 and a face within 6,826 bytes. That is why a span-resolved outline is not `stored_solid` (below). The rme room 576007, which has no Revit export, has a count of 21 and 20 faces in the walk and the span alike: its outline is its own faces'.

**From faces to an outline.** Each wall face's plan edge is directed so the room is on its left, which the face's own frame already does: on all 221 and 222 rooms of the office the walk keeps, Core Interior's 116, RE1's 11 and the 14 to 85 each the Autodesk samples keep, the directed edges join head to tail (run 41710248227). `region_boundary` cuts every segment where another crosses or touches it, reads the winding number to either side of each piece, keeps the pieces with the room on one side only, and gives up unless every point is left by one piece and entered by one. A column that overlaps a wall takes its footprint off the room where they overlap and adds nothing outside it. Ends within 0.0025 ft meet: the steps measured are 0.0007 to 0.0015 ft, and the shortest real edge 0.0052 ft; the value is chosen from these, not copied, since revit-ifc takes the short-curve tolerance from the Revit API at run time (`IFCProject.cs` sets it from `application.ShortCurveTolerance`) and has no constant. The outline must span the record box to 0.001 ft, as RE-101's does.

**What a resolved room says.** `SpaceOutlineSource`:

| Value | Read | Area |
|---|---|---|
| `stored_solid` | RE-101's walk closed on the box | as RE-313 |
| `stored_solid_faces` | the span above, spanning the box to 0.001 ft | Revit's to 0.1% on 19 of the office's 21 scored rooms; room 1671995 is off (§3) |
| `stored_solid_approximate` | the same, spanning the box to within 0.02 ft short of it on a side, tried only where the exact span fails | 0.68% and 0.47% under Revit's on the two rooms Revit's export pairs |

The approximate rooms are three as built: a column overlaps a wall and its back stands 0.0164 ft behind the wall's face, which leaves the outline 0.0164 ft short of the box on one side; the stored solid takes the column off the room and Revit's export does not take that column off, though it takes off the 1.722 ft² columns of eight rooms at permit; the two rooms it pairs differ from the outline by 3.0 ft², the column's 3.001. Griffin chose to close them, labelled (2026-11-03). The exports count them (`exported.resolved_outline_spaces`, `exported.approximate_outline_spaces`; `rvt-ifc` prints a line for each), and the room schedule gives an area for `stored_solid` and `stored_solid_faces` rooms and none for approximate ones.

A solid with a cylinder is not resolved, and a resolved outline carries no prism marker, so no `NetVolume`.

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

As built, the rooms that kept their box (ft²):

| Room | Box | Revit | Outline | `SpaceOutlineSource` | Cause |
|---|---:|---:|---:|---|---|
| 1383998 | 1708.89 | 1020.46 | 1020.46 | `stored_solid_faces` | step |
| 1422302 | 362.30 | 291.54 | 291.54 | `stored_solid_faces` | record between faces |
| 1444351 | 362.31 | 291.56 | 291.56 | `stored_solid_faces` | record between faces |
| 1493531 | 362.31 | 291.56 | 291.55 | `stored_solid_faces` | step |
| 1681182 | 5963.96 | 3818.36 | 3818.36 | `stored_solid_faces` | step, long solid |
| 1700966 | 4774.18 | 3259.86 | 3259.85 | `stored_solid_faces` | step |
| 1736690 | 2846.73 | 2049.24 | 2049.25 | `stored_solid_faces` | step |
| 1736726 | 367.14 | 293.43 | 293.43 | `stored_solid_faces` | record between faces |
| 2297732 | 135.82 | 131.76 | 131.76 | `stored_solid_faces` | step |
| 1671995 | 5228.48 | 1218.33 | 5154.12 | `stored_solid_faces` | **disagrees with Revit** |
| 1832504 | 465.18 | 440.31 | 437.31 | `stored_solid_approximate` | column overlaps wall |
| 2066902 | 1542.77 | 638.23 | 635.23 | `stored_solid_approximate` | column overlaps wall |
| 1820322 | 1882.11 | not paired | 1865.97 | `stored_solid_approximate` | column overlaps wall |

**Room 1671995 disagrees with Revit's export, as built and at permit.** Its stored solid is one region with 6 holes whose outline is 5154 ft² (the box, 5228); Revit's export gives it 9 pieces with 1 hole totalling 1218 ft² (1146 at permit), their centroids 25.8 ft apart. Its list counts 69 objects and the walk reads 67 faces, as for the rooms that agree; nothing in the file tells it from them. It was a box 329% over Revit's area (356% at permit) and is now an outline 323% over (350%), with its boundary 1.80 ft from Revit's (2.30 ft as a box). It is the one room of the office where the stored solid and Revit's export disagree; whether Revit's room is split by something the file does not say, or the stored solid is not the room Revit exports, is not found. It carries `stored_solid_faces`, so an importer can see that this reading is not the walk's.

At building permit, 11 rooms are `stored_solid_faces`: 1436086, 1480292, 1736676, 1736690, 1736720, 1743273, 1743279, 1743309, 1743715 and 2302311 (all with Revit's area to 0.1%: their boxes were 2.6% to 58% over it) and 1671995. Room 3095831 keeps its box: 14 ends at y = 35.269 ft, in pairs 0.656 and 1.312 ft apart down a 100 ft wall, are not joined, since the columns along it share the wall's line.

Rooms per outline source (the export's own counts, run 41712392709; the scorer pairs 276 of the as-built file's 308 spaces and 278 of the permit's):

| | `stored_solid` | `stored_solid_faces` | `stored_solid_approximate` | `record_box` | family instances, `record_box` |
|---|---:|---:|---:|---:|---:|
| Office as built (234 rooms) | 221 | 10 | 3 | 0 | 44 |
| Office at permit (234 rooms) | 222 | 11 | 0 | 1 | 44 |

`space_profiles_vs_ifc`, `main` against this change (run 41712392709):

| | Area within 0.1% | `stored_solid` scored | `stored_solid_faces` scored, area within 0.1% | `stored_solid_approximate` scored, area within 0.1% | Room, record box |
|---|---:|---|---|---|---:|
| Office as built | 221 → 230 | 220, all | 10, 9 | 2, 0 | 12 → 0 |
| Office at permit | 222 → 232 | 222, 221 | 11, 10 | 0 | 12 → 1 |
| Core Interior | 116 | 116 | 0 | 0 | 0 |
| RE1 Architecture | 11 | 11 | 0 | 0 | 0 |

At permit the boundary distance is within 0.01 ft on 10 of the 11 `stored_solid_faces` rooms; the outline metric (which counts the vertices of Revit's ceiling steps, RE-313 §3) on 9: room 1436086 is 0.492 ft off on it and 0.000 ft on the boundary. The one `stored_solid` room at permit that is off (2T7gRjEFvBgRaNLZHAfExM, room 2611271, STAIRCASE, 66% in area) is RE-313's, unchanged; the walk closes a loop of 172.47 ft² on its box where Revit's area is 103.90, and the span does not change that, since the walk comes first.

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

For rooms 1422302, 1444351 and 1493531 the cache holds the record box's area (362.3 ft²), not the 291.5 ft² of Revit's export and of the outline. The cache is not there on every release (`rac_advanced`: none of 21 on 2019, 19 of 21 on 2024), so it is not used as evidence.

### The Autodesk samples (no Revit export)

`probe_re316_room_closure` on the 2019 and 2024 saves (run 41710248227):

| Project | Rooms | Closed by RE-101 | Resolved now | Box |
|---|---:|---:|---:|---:|
| `rac_basic` | 14 | 14 | 0 | 0 |
| `rac_advanced` | 91 | 85 | 0 | 6 |
| `rme_basic`, `rme_advanced` | 76 | 26 | 1 (`stored_solid_faces`, room 576007: 1887.2 ft² against its box's 1917.9) | 49 |
| Snowdon Towers (architectural, 2024) | 54 | 47 | 0 | 7 |

### Everything else

Measure unfiltered, `-f base=main -f nordic=true -f r2025=true -f samples=true -f holdout=true` ([41712392709](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41712392709): this change at edd7a72a against `main` at 8664b94a; 93 models, every scorer including `space_profiles_vs_ifc`). Every scorer's output is identical on every model but three, and the exports' hashes move on exactly the models with a room that now closes:

- **Office as built and at permit**: the rows above. `exported.bounding_box_spaces` goes from 57 to 44 and from 56 to 45 (the 44 family instances, and room 3095831 at permit).
- **`rme_basic` and `rme_advanced`, 12 files over the 6 releases 2015, 2019, 2024, 2025, 2026 and 2027**: one room each (576007, 1887.2 ft² against its box's 1917.9) takes an outline, `stored_solid_faces`, and box spaces go 50 → 49; no Revit export of them exists.
- **Held out, counts only (BIM Speed's Revit 2021 project, Warmond)**: the one model outside the office whose scorer output changes. Area within 0.1%: 174 → 179 of the 179 paired spaces, off 5 → 0; outline and boundary within 0.01 ft 174 → 179; rooms on their record box 8 → 2 (both within 0.1%: rectangles), `stored_solid_faces` 6 (all within 0.1% and 0.01 ft), `stored_solid` 171 unchanged. Of the 8 rooms that kept their box, 5 were off in area and 6 now take an outline: the sixth was a rectangle already equal to Revit's. Vertices within 0.01 ft go from 127 to 126, which the scorer prints and no earlier summary did: that room's outline has vertices the box had not. This model was not read while the rule was found.
- **Core Interior, RE1 Architecture, the R2025 models and the other held-out models**: unchanged, so no witness observation moves. `rac_advanced`, `rac_basic` and Snowdon Towers' architectural sample in every release: unchanged.
- **Cost**: Measure's `rvt-ifc` export times are 0.98 times main's at the median across the 93 models, 1.03 and 1.05 on the office.

## 4. What keeps its box

- **Office room 3095831 (permit)**, above.
- **The six `rac_advanced` rooms** (177330, 214970, 215140, 215177, 216415, 217813): round columns. With their arcs the six close (the probe's `with arcs` column). 43 of the 85 rooms RE-101 keeps in the same file have round columns it ignores, its outline 0.2% to 1.2% larger than the one with the columns taken off (room 177304: 3531.7 ft² against 3521.8). One of those two readings is wrong and no Revit export of the sample says which, so both are left as they are.
- **49 of the 50 `rme` rooms**: stepped floors and ceilings. The walls that reach the ceiling instead of the floor close for 18 of the 49 (run 41710248227; `rme_basic` and `rme_advanced` are the same), 13 of those equal the box and 5 differ from it by 1% to 11%; no Revit export of these rooms exists, and a bulkhead that hangs from the ceiling would be taken for a wall. They stay boxes.
- **Snowdon Towers' seven rooms** (#90: 828537, 828548, 830902, 830912, 2181757, 2181762, 2285535, in Autodesk's 2024 sample): their lists count more objects than the walk reads on three of them (25, 20 and 61 more: 828537, 828548 and 2181762), and no Revit export of the sample is in CI.
- **The 44 family instances written as spaces** are #1262's.

## 5. Not claimed

- That a `stored_solid_faces` outline equals Revit's outline on a release other than 2024: the evidence is one building, in two states, and one held-out project. 19 of the office's 21 scored `stored_solid_faces` rooms agree and 2 do not (1671995, twice), and nothing in the file tells them apart.
- That the span holds no face of another element: no solid measured has one inside 6,826 bytes of its last face where the list counts the walk's faces exactly, and the rule admits at most the list's difference elsewhere.
- That a cylinder's arc is Revit's on any room: `rac_advanced` has no export.
- Any volume of a resolved room (no prism marker is written).
- That the 0.02 ft slack is Revit's: it is the 0.0164 ft the three rooms measured need, and the two paired are 0.68% and 0.47% under Revit's area.

## 6. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=7d7c098c8ede1ae8bb55d559a4bfdf270a5e70fc -f base=none -f samples=true -f sample_years="2019 2024" \
  -f nordic=true -f models="nordic-office-* core-interior re1-architecture" \
  -f probe="probe_re316_solid_extent probe_re316_room_closure probe_re316_hostile_solid" -f probe_scorer="- - -" -f scorers=space_profiles_vs_ifc
gh workflow run measure.yml --ref main -f ref=edd7a72a943ee0d99da416ecf86269722977c80c -f base=main -f nordic=true -f r2025=true -f samples=true -f holdout=true
```
