# RE-334: Rooms take off the round columns their stored solid holds

**Date:** 2026-11-04
**Issues:** #1354 (rac_advanced's rooms against the public IFC2X3 export); follows RE-316 (section 4 left this open), RE-330 (the export)
**Artefacts:** Autodesk's `rac_advanced_sample_project`, saves of 2015 to 2020 (`research/autodesk-sample-projects.tsv`); the IFC2X3 export of the same project from the three.js repository at commit `7c1f47731608de51a2240ce31655634665cb51ce`, 42,831,859 bytes, sha256 `9440a72eb2d27e3ba9edeafdcdfb71945e6baec56d5b09211bcde743de7fe0a5` (`research/rac-advanced-export.tsv`); for the other exporters, the BIM4LCA office (Revit 2024), Core Interior, RE1 and the 23 R2025 models with Revit's exports
**Tool:** `tools/re/space_profiles_vs_ifc.py` (unchanged); Measure's new `racadv` input
**Status:** positive on two projects, with two external checks. On `rac_advanced`'s six saves of 2015 to 2020, all 91 rooms are within 0.1% in area and 0.01 ft in outline of the public IFC2X3 export (before, 42 of the 85 rooms drawn from their stored solid were, and 6 rooms kept their record box), and every one of the 48 spaces with round columns has as many voids as the export's. A public Revit 2018 IFC4 export of `rac_basic` draws the one column of room 101 as a void too, and the reader moves that room from 1.710% to 0.628% over its area (section 4). Those two are the only Revit exports of a room with a round column there are: none of the 53 other models measured (the office, Core Interior, RE1, the R2025 models and the held-out files) has one. In-sample for the decision to take the columns off, since it was read from the first export (section 0); not for the arcs, which RE-316 read from the file before the export was found, and not for the second export, which was read after the rule.

## 0. What the check rests on

- The check is **the IFC2X3 export of RE-330 section 1**: Revit 2021's exporter, string ending "IU alternativa 21.0.0.383", publisher not established, showing the 2015 to 2020 state of the project. It is counted apart as IFC2X3, as RE-330 counts it. It is used here as a geometry oracle for rooms and **removes nothing**: no value that main writes is dropped by it. What the export decided is a reading of the file (a cylinder face of the room's stored solid is an edge of its outline, so a column is a void), not a value to leave out.
- **One Revit IFC4 export of a room with a round column exists** that the corpus does not hold: the public Revit 2018 export of `rac_basic` (RE-326 section 5), read outside Measure (section 4). Where Revit's exporters disagree about a value the standing rule leaves it out; here there is no second exporter to disagree.
- **In-sample.** That the columns are taken off was decided by reading this export. The arcs the reader uses (RE-316 section 1: the axis at +158, the radius at +182, the uv box an angle and a distance) were read from the files in RE-316, before the export was found, and the reader draws them with the tolerance RE-316 chose (0.0005 ft). A Revit export of another project with round columns in its rooms, or of another release, is what would test the decision.
- The Measure pair (`-f racadv=true`, models `racadv-2015` to `racadv-2020`) fetches the sample and the export for the run, pinned by sha256 and byte count, and never stores them or puts anything derived from the export in the Actions cache. Their per-room scorer output is uploaded in full, as the Autodesk samples' is (the main session, 2026-11-04). The export carries no licence of its own and Autodesk's terms for the sample it exports were not checked.

## 1. Why 43 rooms were 0.17% to 1.24% over

The kept IFC of the 2020 save at 35a9d322 (#1351's run 41832628804) and the pinned export, read as plain STEP outside IfcOpenShell: each space's extrusion profile placed through the solid's position and the space's object placement, then areas and voids compared by GlobalId. It reproduces the scorer's count: 58 of 107 spaces within 0.1% in area, 42 of them rooms and 16 Areas.

- 48 of the export's 116 spaces are an `IfcArbitraryProfileDefWithVoids`. Each void is a 34-vertex circle of radius 0.4917 ft (RE-316 read 0.4921 ft off the cylinder faces). All 48 are among the 107 spaces rvt-rs writes.
- rvt-rs drew those rooms without voids. For the 43 rooms the walk read, with the circles of radius 0.4921 ft (0.7608 ft² each) taken off, the number taken from Revit's profile, rvt-rs's area is within 0.007% of Revit's on every one (largest: room 125, -0.007%; the others 0.001% to 0.003%). Before it, 0.169% (room 123) to 1.235% (room 125) over.
- The count of voids per room is the count of columns: room 102 has 13, rooms 216 and 318 11 and 13, room 321 8, room 121 8.
- The six rooms that kept their record box (108, 204, 209, 223, 229, 306) have columns against their walls: Revit's profiles for them have 59 to 283 vertices in all (the arcs as chords) and 0 to 7 voids, and a box is 0.34% (room 204) to 13.2% (room 223) over them.
- Not the cause: the phase (all 91 rooms pair by GlobalId), the export's form (it writes 48 spaces as `IfcRectangleProfileDef`, which is the same geometry as a four-vertex polygon, and the 43 rooms are already `partition_room_solid_faces`), or slack.

## 2. The rule

`wall_edges` in `partition_room_boundaries.rs` takes, for a vertical cylinder face that reaches the floor (its lowest end within 0.001 ft of the box's floor, its axis vertical, its u and v horizontal), the chords of its arc within 0.0005 ft as plan edges, run clockwise around the axis, as RE-316 did for its probes. It did so only for a span that asked (`SpanRead::arcs`); now the walk of RE-101 reads them too (`span` unset), and `resolved_outline_of_solids` asks. A cylinder whose arc cannot be read (a sweep outside 0 to 2 pi) rules the room out, and a room with more than 4,096 plan segments (`PLAN_SEGMENTS_MAX`; a column of 0.49 ft is 70 chords, and the most in one room measured is 13 columns) is left to its box, so the quadratic work of joining and nesting the segments is bounded however many cylinders a solid claims.

Nothing else changes: planar faces, the box test, the label (`SpaceOutlineSource` stays `stored_solid` for a room the walk reads, and the profile source `partition_room_solid_faces`).

## 3. Results

All Measure, dispatched from the branch's workflow (`--ref fix/1354-rac-advanced-rooms`), `space_profiles_vs_ifc`, filtered.

- **`rac_advanced` 2015 to 2019 on `main` at fdff7018**, run 41899250576: 107 spaces in both, area within 0.1% 58, within 1% 104; outline, boundary 58; vertices 47; lowest and highest vertex 107. Rooms: stored solid 85, area 42; record box 6, area 0. Areas: 6 `area_boundary` and 10 `area_boundary_joined`, all within 0.1% and 0.01 ft. The five saves give identical counts.
- **The reader at 95817487**, run 41899789207 (2015 to 2019): area within 0.1% **107**, within 1% 107; outline, boundary and height 107; **vertices 47**, unchanged. Rooms: **stored solid 91, area, outline, boundary and height 91 of 91**, none on its record box. The five saves give identical counts.
- **All six saves, the reader at f4f3da51 (its code is 95817487's) against `main`**, run 41900428551: the 2020 save, which the first runs lacked (its file is named `racadvancedsampleproject.rvt`), gives the same counts on both sides as the other five: `main` 58 of 107, the reader 107 of 107, and the six saves' scorer outputs equal to one another on each side.
- **Outside the scorer**, the kept IFC of the 2019 and the 2020 saves against the export, read as STEP: 107 spaces paired, the same number of voids in every one (48 spaces with voids on each side), the largest area difference 0.0058% (room 125), the next 0.0048%. The reader draws a circle as 70 chords and Revit's export as 34, which is why **vertices stay at 47 of 107**: the vertex-to-vertex metric needs the same tessellation, and rvt-rs does not copy Revit's. The outline and boundary metrics, which measure to the other file's outline, are 107 of 107.

## 4. Revit's other exporters, and the other releases

- **The office, Core Interior, RE1, the R2025 models**, run 41899794330 (33 models, the reader against `main`): **every export is byte for byte as before** (the `ifc.sha256` of all 33 equal on both sides) and every `space_profiles_vs_ifc` output is equal. So none of those 33 has a room with a vertical cylinder that reaches the floor, and none can disagree with the voids: the BIM4LCA office and Core Interior have Revit IFC4 exports, RE1 an IFC2X3 one.
- **The held-out models and BIMcollab's example**, run 41900672250, counts only (18 held-out files, BIMcollab's Revit 2015 example and the spent Vitoria pair: 20 models, the reader against `main`): **all 20 exports are byte for byte as before** and every scorer output (the counts of the 19 held out, the full output of the spent pair) is equal. None has a room with a round column either.
- **Autodesk's samples, 2015 to 2027**, run 41899827774 (58 pairs, the reader against `main`): 43 exports are byte for byte as before and 15 move.
  - **`rac_advanced` on all 13 releases, 2015 to 2027**: `exported.bounding_box_spaces` goes from 6 to 0 on every save, so the six rooms with columns against their walls take an outline on every release, the 2021 to 2027 saves too. The export shows the 2015 to 2020 state, so the 2021 to 2027 saves are scored against nothing; their rooms are the same project.
  - **Snowdon Towers architectural, 2024**: `bounding_box_spaces` 7 to 5 and `bounding_box_bodies` 3,104 to 3,102: 2 of the 7 rooms RE-316 left on their box take an outline. **No export scores them**: the local Revit IFC4 export of that sample holds no `IfcSpace` at all (0 of its 787,488 entities).
  - **`rac_basic` 2015**: the export moves and its diagnostics are equal on every field. It is the one save that writes the project's 14 rooms by default (RE-326: Revit 2016 and later leave them out for their phase). Run 41904133402 kept the 2015 save's IFC for `main` and for the reader, and the public Revit 2018 IFC4 export of the project (RE-326 section 5: ReferenceView, exporter 17.4.0, sha256 `1114ce91…5ea35`, a forum attachment with no licence, used locally and not committed) was read beside them as plain STEP, outside Measure and outside IfcOpenShell. Its room 101 is an extrusion of a profile with **one void**, a 34-vertex circle of 8.403 ft²; the other 13 rooms are extrusions without one or triangulated face sets (floor area from the downward triangles). Against its area (781.13 ft²): **room 101** `main` 794.49 ft² (+1.710%), the reader 786.04 ft² (+0.628%) with one void of 8.451 ft², **so the reader closes 8.45 of the 13.36 ft² between them**; the other 13 rooms are equal on `main` and the reader (11 within 0.001%, rooms 105 and 201 +0.195% and +0.147% on both). The 4.95 ft² still over in room 101 is in its outer ring (794.49 ft² against Revit's 789.54), which this change does not touch, and is not examined. **No room gets worse.**
- **So the voids are confirmed by two exporters:** the IFC2X3 export of `rac_advanced` (48 spaces, 107 of 107 pairs with the same number of voids) and the IFC4 export of `rac_basic` (room 101: one void, 8.403 ft² against the reader's 8.451). No Revit export of a room with a round column disagrees. If one does, the standing rule says to leave the voids out, and this report is where that would be recorded.

## 5. Not measured

- Revit's own quantities for the `rac_advanced` rooms: its export carries none. The `rac_basic` export carries Revit's `GSA Space Areas` for each space, and room 101's is 781.08 ft², its geometry with the column taken off (781.13 ft²; 789.54 without it): Revit's own room area takes the column off in that one room, 0.007% from the geometry. The reader's schedule area for room 101 is 786.04 ft², 0.635% over it (the outer ring, section 4). The schedule area and a prism room's `NetVolume` follow the outline and change on rooms with columns.
- The `rac_basic` comparison is by plain STEP outside Measure and IfcOpenShell, on the 2015 save and Revit 2018's export; the export is not in Measure, so nothing gates it. Room 101's outer ring (4.95 ft²) is not explained.
- The 2021 to 2027 saves of `rac_advanced` and Snowdon's two rooms against any export (section 4).
- That a column against a wall is a notch of Revit's outline on any project but this one: the six rooms are the evidence.
- Rooms with a cylinder whose axis is not vertical, or that does not reach the floor: ignored as before.
- Revit's tessellation of a circle: not copied (section 3).

## 6. Reproduce

```bash
# the export, by pin; never stored
curl -fsSL -o rac_advanced_sample_project.ifc \
  https://raw.githubusercontent.com/mrdoob/three.js/7c1f47731608de51a2240ce31655634665cb51ce/examples/models/ifc/rac_advanced_sample_project.ifc
sha256sum rac_advanced_sample_project.ifc   # 9440a72eb2d27e3ba9edeafdcdfb71945e6baec56d5b09211bcde743de7fe0a5
# the six saves against it, the reader against main, IFC kept
gh workflow run measure.yml --ref fix/1354-rac-advanced-rooms -f ref=f4f3da511eef1543739dad88c6d7fec2eea7070d -f base=main \
  -f racadv=true -f models="racadv-*" -f scorers=space_profiles_vs_ifc -f keep_ifc=true
python3 tools/re/space_profiles_vs_ifc.py model.ifc rac_advanced_sample_project.ifc --match globalid --list 10
# the other exporters
gh workflow run measure.yml --ref fix/1354-rac-advanced-rooms -f ref=9581748713da809d7c152eed7ea5e5fb92bde431 -f base=main -f nordic=true -f r2025=true \
  -f models="nordic-office-* core-interior re1-* r2025-*" -f scorers=space_profiles_vs_ifc
```
