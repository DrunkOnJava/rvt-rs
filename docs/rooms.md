# Rooms and spaces in an rvt-rs IFC export

This page is for the importer of an rvt-rs export: what an `IfcSpace` holds for each release, the one property that says whether to trust its area, what is measured against Revit's own export and where, and what is not. Every `IfcSpace` rvt-rs writes in a geometry export (the default) carries that property: rooms, Revit Spaces, family instances written as spaces and Areas alike. An export with no geometry (`typed_no_geometry`) has no outlines and no property. The evidence is in [RE-313](../reports/element-framing/RE-313-space-outlines-measured.md) and, for the rooms whose stored solid the first reader did not close, [RE-316](../reports/element-framing/RE-316-room-solids-resolved.md); the earlier findings are RE-29, RE-101, RE-102, RE-117, RE-153, RE-196, RE-255 and RE-296, and RE-191 and [RE-317](../reports/element-framing/RE-317-area-boundaries.md) for Areas.

## Which `IfcSpace`s rvt-rs writes

| Source | `IfcSpace` | Body |
|---|---|---|
| A Revit room | `Name` is the room's number, `LongName` its name, `PredefinedType` `SPACE`, in the storey of the Level its record names | the outline of the room's stored solid, or the record's bounding box (below) |
| A Revit area (Revit 2024 to 2027; Revit 2015 to 2023 only where the body is proven, RE-324, RE-330) | the same naming | the region its area boundary lines enclose around its point, else the region of Revit's own plan topology for the Area (RE-330), up to the Level above it (below); none where neither is proven: the space then has no `Representation` |
| A Revit Space (MEP), in a model that has one | `IfcSpace` without a predefined type; no measurement singles one out | its record's box (`record_box`) unless a carrier draws more (`family_body`) |
| A family instance whose "Export to IFC As" names `IfcSpace` (#763) | named `Family:Type:ElementId` as Revit's export names other family instances | its record's box, turned with the instance (`record_box`), until a carrier draws its family. Where its family draws no model form, Revit's export leaves the instance out, and rvt-rs will too once PR #1279 (RE-312, for #1262, parking bays) lands |

A room carries its ElementId (`RvtElementRecordGeometry.ElementId`), its number and name (`RoomNumber`, `RoomName`), `Pset_SpaceCommon.Reference` (name and number) and `IsExternal` where measured, and `Pset_SpaceCoveringRequirements.FloorCovering` where the room's Floor Finish is read. A room's `Qto_SpaceBaseQuantities.Height` is written where its record holds the solid's faces (RE-255) and its `NetVolume` where every vertical face of the solid runs from floor to ceiling, on Revit 2024 and 2025 (RE-296). `Qto_SpaceBaseQuantities` never holds an area: take the area from the body.

## Which property says whether to trust the area

Read `RvtElementRecordGeometry.SpaceOutlineSource` on the `IfcSpace`:

| Value | The body is | Its plan area is |
|---|---|---|
| `stored_solid` | the outline of the room's stored solid, extruded to the room's height, read by the exact walk of RE-101 | Revit's room area: within 0.1% of Revit's export on every `stored_solid` room scored below by the scorer's upward faces except one, the permit office's STAIRCASE room 2611271 at 66%, which by Revit's floor is within 0.1% like the others (221 of 221 as built, 222 of 222 at permit; RE-313 §2, §3, RE-316 §3) |
| `stored_solid_faces` | the outline of the room's stored solid, read from the faces its solid lists, up to the solid's own end, where the walk did not close them: their edges close as the walk reads them (RE-316) | Revit's room area: within 0.1% on 9 of 9 office rooms as built, by every metric. Bounded by the solid's list of objects and a gap, not by the walk, so a closed loop of another element's faces inside the room's box would be read as the room's: that is why it is labelled apart (RE-316 §2, §3) |
| `stored_solid_approximate` | the same, as the boundary of a region, a column that overlaps a wall taken off it or an end of one wall edge lying on another, or up to 0.02 ft short of the room's record box on a side | approximate. Of the office's 15 room scores (4 as built, 11 at permit) 10 are within 0.1% of Revit's floor area and 5 are 0.45% to 0.68% under it (four rooms, 1671995 in both files): Revit's export does not take such a column off the room, and nothing in the file says which. The room schedule leaves the area blank (RE-316 §2, §3) |
| `record_box` | the record's bounding box (turned with a family instance that is turned) | the box's area, not the space's. It equals the space's only when the space is a rectangle; the area is Revit's on none of the office's room scores that keep their box (1 room at permit, 3095831; RE-316 §4) |
| `saved_mesh` | the mesh Revit saved for the room (`rvt-ifc --saved-meshes`) | the mesh's; not scored |
| `area_boundary` | the region that the lines of the Area's scheme on its Level enclose around its point, its holes the loops directly inside, extruded from its Level up to the Level above it, both building stories | Revit's area space: within 0.1% of Revit's export, within 0.01 ft in outline, and with its lowest and highest vertex within 0.01 ft, on 60 of the 60 Area scores below, the office counted at both stages (RE-317 §5) |
| `none` | no body (an Area whose boundary is not proven) | not available |
| `family_body` | a Revit Space or a family instance written as an `IfcSpace`, drawn as more than its box; its `BodySource` says how | not scored |

A room's `BodySource` is `partition_element_record_bbox` whichever of the stored-solid or `record_box` values it has, because the placement and the height always come from the record's box: read `SpaceOutlineSource`, not `BodySource`. An export from before this property (rvt-rs 0.12.x and earlier) says the same through `ProfileSource` (`partition_room_solid_faces`, and since RE-316 `partition_room_solid_faces_resolved` and `partition_room_solid_faces_approximate`) and `ProfileResolved`. A room with `stored_solid_faces` or `stored_solid_approximate` carries no `NetVolume`.

The export also counts the spaces drawn as their record's box (rooms and family instances alike), so an importer can check a whole file before reading a space:

- `exported.bounding_box_spaces` in the diagnostics JSON (`rvt-ifc --diagnostics`), out of `exported.by_ifc_type.IFCSPACE`;
- `exported.resolved_outline_spaces` (`stored_solid_faces`) and `exported.approximate_outline_spaces` (`stored_solid_approximate`), the rooms whose outline was not read by the exact walk;
- a line in `rvt-ifc`'s summary for each: "N of M spaces are their record's box", "N rooms take an outline resolved from the faces up to their stored solid's end", "N rooms take an approximate outline".

## Which releases read a room's outline

| Release | Outline | Measured against Revit's export |
|---|---|---|
| 2008 to 2021 | read from the stored solid where the reader finds one | held-out models (below): two 2021 projects; Vitoria's Revit 2018 `ALD26 Before` (spent, below), whose frames sit in their records (RE-315, #1261) |
| 2022 | stored solid | 8 held-out rooms |
| 2023 | stored solid | 9 of 9 rooms of one project, local only (RE-102) |
| 2024 | stored solid; Areas as spaces with a body where their boundary lines prove one | Core Interior, the BIM4LCA office, held-out models (below) |
| 2025 | stored solid; Areas as on 2024 (RE-330) | RE1 Architecture, six purpose-made R2025 models, held-out models (below) |
| 2026, 2027 | stored solid, as on 2025 (RE-314, #1260); Areas as on 2024 (RE-330) | 16 held-out rooms, all rectangles, and a held-out house (below) |

Areas as spaces, with a body where their boundary lines prove one, are read on Revit 2024 to 2027; on 2025 to 2027 they are checked against the same project's 2024 save (Autodesk's `rac_advanced`: the same 25 Areas, numbers, names, storeys and bodies on each), and on Snowdon Towers Architectural against a Revit 2025 export (local only): all 101 of its Area spaces, and 95 of 96 bodies within 0.1% of Revit's area (RE-330). Where the lines give an Area no region, its region is the loop Revit's own plan topology keeps for it, where that is read and checked (RE-330): its area boundary lines moved by their area rule's offsets, arcs included, with the area Revit saves for the loop. On Revit 2015 to 2023 an Area is written only where its region and body are proven; the others are not written and are counted by reason in `skipped` (`area_without_proven_body`, RE-324). They are measured against the same project's 2024 save on Autodesk's samples, and against no Revit export: the first rule, writing every placed Area as 2024 does, wrote spaces Revit's export lacks on a held-out project (below).

`ROOM_SOLID_SUPPORTED_REVIT_VERSIONS` in `src/research/partition_room_boundaries.rs` is the list of releases whose room solids are read.

## What is measured, and where

`tools/re/space_profiles_vs_ifc.py` meshes both exports, moves them into Revit's internal feet and compares the plan union of each space's upward faces. It prints four figures, each on its own line:

- **area**: the plan area, within 0.1% of Revit's;
- **boundary**: the largest distance from a point of either outline to the other's outline, within 0.01 ft. This is the room's outline, and the figure used for "outline" in the table and in the text below;
- **outline**: the largest distance from a vertex of either mesh's upward faces to the other's outline, within 0.01 ft. Revit meshes a ceiling step inside a room as an upward face of its own, and its vertices count against this one, so it reads some correct outlines as wrong (RE-313 §3). The CI gate pins both. These three read the faces of Revit's mesh that point up, within 25 degrees of vertical, so a room under a stepped or sloped ceiling is read as the flat part of it: `tools/re/space_overlaps_vs_ifc.py` also measures the plan area of each space's floor (its downward faces) and of every face projected, in both exports, and counts the rooms within 0.1% of Revit's by each (RE-316 §3). The floor is the room's plan.
- **height**: the lowest and the highest vertex of either mesh, within 0.01 ft: the base and the top of the space, so its height. It is printed on its own line and for each kind of space (RE-317); it is 306 of 306 spaces in both exports on the office as built and 308 of 308 at building permit.

Measure runs the scorer on every reference model (`-f scorers=space_profiles_vs_ifc`); the IfcOpenShell job of CI runs it on the first two rows, pairing by GlobalId (`--match globalid`), and fails below their counts in area, outline and boundary.

| Model (release) | Spaces in both | Area within 0.1% | Boundary within 0.01 ft | Outline within 0.01 ft | Outline source: count, area, boundary |
|---|---:|---:|---:|---:|---|
| Core Interior (2024) | 116 | 116 | 116 | 116 | 116 rooms, stored solid: 116, 116 |
| RE1 Architecture (2025) | 11 | 11 | 11 | 11 | 11 rooms, stored solid: 11, 11 |
| BIM4LCA office as built (2024) | 306 of Revit's 308 | 260 | 260 | 250 | 220 rooms, stored solid: 220, 220; 9 rooms, stored solid faces: 9, 9; 3 rooms, stored solid approximate: 0, 0 (4 by Revit's floor, 0 within 0.1%); 44 family instances, record box: 1, 1; 30 areas, area boundary: 30, 30 |
| BIM4LCA office at building permit (2024) | 308 of Revit's 308 | 262 | 262 | 255 | 222 rooms, stored solid: 221, 221; 11 rooms, stored solid approximate: 10, 10; 1 room, record box: 0, 0; 44 family instances, record box: 1, 1; 30 areas, area boundary: 30, 30 |
| R2025 purpose-made models with rooms (2025), 6 models | 12 | 12 | 12 | 12 | 12 rooms, stored solid: 12, 12 |
| Held-out models (2022, 2024, 2025), 6 models | 24 | 24 | 24 | 24 | 24 rooms, stored solid: 24, 24 |
| Held-out models (2026, 2027), 4 models | 16 | 16 | 16 | 16 | 16 rooms, stored solid: 16, 16 (all rectangles) |
| One held-out house (2026) | 12 of 13 | 10 | 0 | 0 | 12 rooms, stored solid: 10, 0 |
| Held-out Gdynia (2021, Revit's IFC2X3 export) | 18 of 19 | 18 | 0 | 0 | 18 rooms, stored solid: 18, 0 |
| Held-out Warmond (2021, export passed through another tool) | 179 | 179 | 179 | 179 | 171 rooms, stored solid: 171, 171; 6 rooms, stored solid faces: 6, 6; 2 rooms, record box: 2, 2 (rectangles) |
| Vitoria `ALD26 Before` (2018, Revit's IFC2X3 export; spent) | 73 | 72 | 72 | 71 | 73 rooms, stored solid: 73, 72 |

On Warmond, RE-316 closes 6 of the 8 rooms that kept their box (5 were off in area; one was a rectangle already equal to Revit's, whose vertices are no longer the box's: vertices within 0.01 ft 127 → 126); the 2 that keep their box are rectangles. Area within 0.1% goes from 174 to 179 of the 179 paired spaces (Measure run 41728355227, `main` at 7fd339e2 against this change, counts only; by Revit's floor too: 177 of 177 `stored_solid` and `stored_solid_faces` rooms).

A model with no rooms in Revit's export (the other R2025 models, RE1 Electrical, Mechanical and Plumbing) scores 0 of 0. One held-out model writes none of Revit's spaces: BIM Whale's 2021 project (8), whose spaces are Areas of a 'Rentable' scheme. The first rule for Revit 2015 to 2023 Areas (RE-324), writing every placed Area as 2024 does, found Revit's 8 and wrote 2 spaces Revit's export lacks (Measure run 41757734285, counts only): a measured negative. An Area there is now written only where its body is proven, and none of the 10 written spaces had one, so the rule writes 0 of Revit's 8 and 0 extras, as `main` does (run 41764492879). That zero is vacuous: it does not show the rule separates Revit's 8 from the 2 extras. The project's Areas stay undelivered (#1261) and, under W5, it is a regression check, never developed on. Vitoria's 2018 project exported no element, and none of its 73 spaces, before PR #1286 (#1261): its spent `ALD26 Before` now writes all 73 (run 41707934595, full output, in the row above), and its held-out sibling `ALD26 Project` writes 73 `IfcSpace` against Revit's 73 (counts only, not scored here).

The other held-out figures are Measure run 41728355227 (`main` at 7fd339e2), counts only; the office's Area figures are Measure run 41740909229 (RE-317), which leaves every held-out count as it was. On the 2026 house and on Gdynia, every room's area is right where it is scored, but none of their outlines is within 0.01 ft of Revit's; the files are held out, so why is not examined.

What the misses are:

- **Rooms the exact walk did not close** (13 as built, 12 at permit; RE-316): as built 9 are `stored_solid_faces` and 4 `stored_solid_approximate`; at permit 11 are `stored_solid_approximate` and 1, 3095831, keeps its box (14 ends down a 100 ft wall are not joined). Before RE-316 all 13 and 12 kept their box (RE-316 §1, §3, §4).
- **Room 1671995 (both files)** is `stored_solid_approximate`, 0.45% under Revit's floor area (5154.12 against 5177.37 ft² as built). The scorer's upward faces read it as 323% over (350% at permit): Revit's ceiling over it is mostly not flat, and its flat patches total 1218.33 ft² in 9 pieces (RE-316 §3).
- **Family instances written as spaces (44 on the office)** are 42 "Turning Free Space" discs and two "VOLUME" families that Revit's export draws from the family's geometry (a mapped representation); rvt-rs draws their record box, a square where the family is a circle (24.22 ft² against 19.01 ft² on the 1500 mm turning spaces listed). No issue covers these yet; #1262 and PR #1279 are about the terrain office's parking bays, a different set, though its rule (a family instance written as a space is left out where its family draws no model form) may change this one too.
- **The office's 30 Areas** are drawn from their area boundary lines, and each equals Revit's in area and outline on both files (RE-317). An Area whose lines are not all along the plan axes, or whose point lies in no single region the lines bound, has no body.
- **The permit office's STAIRCASE room 2611271**, 66% off in area by the scorer's upward faces (172.77 against 103.90 ft²), is within 0.1% of Revit's floor area, like the other 221 `stored_solid` rooms: the scorer reads a room with a stepped or sloped ceiling as a fraction of itself (RE-316 §3, `tools/re/space_overlaps_vs_ifc.py`).

## What is not measured

- **Areas** (RE-317, RE-324), measured against Revit's export on the BIM4LCA office only:
  - Revit 2015 to 2023 against any Revit export: Autodesk's `rac_advanced` (2015 to 2023) and `rst_basic` (2016 to 2023) are read like the same project's 2024 save, and the held-out BIM Whale project is a regression check, quoted by counts only (RE-324). An Area there is written only where its body is proven. Since RE-330 all 25 of `rac_advanced`'s Areas have a region on every save, each with the area Revit saves for its loop (Revit's figure in the file, not Revit's export), and all 25 are written but on the 2021 save, where 3 name lines whose two copies disagree and are not (`named_lines_not_in_scheme_and_level`). Revit 2008 to 2014 are not read, and a 2015 Area that names a design option is left out;
  - the top storey: an Area on the highest Level has no body, where Revit draws it one unit high;
  - Levels that are not building stories: an Area has a body only where its Level and the Level above are both read as building stories and no other Level lies between them or at either elevation. All ten of the office's Levels are storeys, so every Area there meets that, and a Level that breaks it is unmeasured;
  - the Level's "Story Above", which Revit's exporter takes first for an Area's height and rvt-rs never reads: the office's heights equal the distance to the next Level up, so a Level whose Story Above is another Level is unmeasured;
  - an Area's base offset, read as zero;
  - the four Areas on one Level of Autodesk's `rac_advanced` that RE-317 put down to walls are bounded by lines whose area is measured 0.497 ft from them, to the walls' faces (RE-330). Revit's plan topology gives their region; where it is not read, lines that are not straight along the plan axes or do not meet once offset leave the Area without a body, as before;
  - `Qto_SpaceBaseQuantities.Height`, which Revit writes on an Area and rvt-rs does not yet.
  The diagnostics count the Areas without a body by reason (`exported.area_body_refusals`).
- Revit 2008 to 2020 and 2023 on any model Measure can run, beyond the held-out counts above: no reference model with a Revit room export exists for them in the corpus.
- Revit Spaces (MEP): no measurement singles one out.
- Room outlines of Revit 2026 and 2027 beyond the rectangles: on the held-out house none is within 0.01 ft of Revit's (#1260).
- A room's `Height` and `NetVolume` as scored by `tools/re/qtos_vs_ifc.py`, not by this scorer.
- Revit's phase filter. A room's phase is `RoomElem.m_phaseId` (not its created phase, which is unset; RE-326), and Revit's exporter leaves out a room whose phase is not the export phase, the project's last unless the export setup or a view names another. rvt-rs does the same: such a room is not written, the diagnostics count it in `skipped` as `room_not_in_export_phase` with the phase it sits in, and `rvt-ifc --phase <name|id>` chooses another export phase. A room whose phase is unset, unread or not one of the project's is written, and so is an Area or an MEP space. Measured: all 621 rooms of the 12 models with a Revit export are in the last phase and in Revit's export, so none of them is changed (unfiltered Measure, run 41784044966: of 111 exports 12 move, the `rac_basic_sample_project` saves of Revit 2016 to 2027, and 99 are byte for byte as before, the held-out models included). Unmeasured: a Revit export made with the default setup that leaves a room out for its phase, because no project in the corpus has one. The only room outside its last phase is in Autodesk's `rac_basic_sample_project` (14 rooms in Working Drawings, the second of three phases); a public Revit 2018 export of it holds all 14, which rvt-rs writes with `--phase "Working Drawings"` (14 of that export's 17 `IfcSpace` by GlobalId, no other entity changing) and leaves out by default (Measure runs 41784415396 and 41784419457). Revit 2015 and earlier write every room, as every element: both phase rules start at Revit 2016, because the element rule's only evidence on a 32-bit file is a Revit 2018 project (RE-315) and a room rule that reached further than the element rule would stand on less. Autodesk's `rac_basic_sample_project` saved by Revit 2015 therefore keeps its 14 rooms (#1332).
- Rooms resolved as `stored_solid_faces` or `stored_solid_approximate` (RE-316) on any release but Revit 2024: the evidence is one building in two states and one held-out project. No face of another element inside the room's box is excluded by anything but the solid's list of objects and a gap of 1,024 bytes (RE-316 §2). The six `rac_advanced` rooms with round columns (read as arcs, they would close in every release of the Autodesk sample, but whether Revit takes a column off a room's area is not measured), 49 of the 50 `rme` rooms (stepped floors and ceilings) and Snowdon Towers' seven keep their box.

## Check it yourself

```bash
rvt-ifc model.rvt -o model.ifc --diagnostics model.diagnostics.json
python3 tools/re/space_profiles_vs_ifc.py model.ifc revit-export.ifc --list 10
```

The second command needs Revit's own IFC export of the same model, IfcOpenShell, NumPy and Shapely (`tools/ci/requirements-ifc.txt`). It prints the counts above, one line each (`area`, `boundary`, `outline`, and the same split by what the space is and its `SpaceOutlineSource`), and for every space off in area or outline, the area on both sides, the distances and `SpaceOutlineSource`.
