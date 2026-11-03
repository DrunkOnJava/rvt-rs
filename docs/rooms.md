# Rooms and spaces in an rvt-rs IFC export

This page is for the importer of an rvt-rs export: what an `IfcSpace` holds for each release, the one property that says whether to trust its area, what is measured against Revit's own export and where, and what is not. Every `IfcSpace` rvt-rs writes in a geometry export (the default) carries that property: rooms, Revit Spaces, family instances written as spaces and Areas alike. An export with no geometry (`typed_no_geometry`) has no outlines and no property. The evidence is in [RE-313](../reports/element-framing/RE-313-space-outlines-measured.md); the earlier findings are RE-29, RE-101, RE-102, RE-117, RE-153, RE-196, RE-255 and RE-296.

## Which `IfcSpace`s rvt-rs writes

| Source | `IfcSpace` | Body |
|---|---|---|
| A Revit room | `Name` is the room's number, `LongName` its name, `PredefinedType` `SPACE`, in the storey of the Level its record names | the outline of the room's stored solid, or the record's bounding box (below) |
| A Revit area (Revit 2024) | the same naming | none: the space has no `Representation` |
| A Revit Space (MEP), in a model that has one | `IfcSpace` without a predefined type; no measurement singles one out | its record's box (`record_box`) unless a carrier draws more (`family_body`) |
| A family instance whose "Export to IFC As" names `IfcSpace` (#763) | named `Family:Type:ElementId` as Revit's export names other family instances | its record's box, turned with the instance (`record_box`), until a carrier draws its family. Where its family draws no model form, Revit's export leaves the instance out, and rvt-rs will too once PR #1279 (RE-312, for #1262, parking bays) lands |

A room carries its ElementId (`RvtElementRecordGeometry.ElementId`), its number and name (`RoomNumber`, `RoomName`), `Pset_SpaceCommon.Reference` (name and number) and `IsExternal` where measured, and `Pset_SpaceCoveringRequirements.FloorCovering` where the room's Floor Finish is read. A room's `Qto_SpaceBaseQuantities.Height` is written where its record holds the solid's faces (RE-255) and its `NetVolume` where every vertical face of the solid runs from floor to ceiling, on Revit 2024 and 2025 (RE-296). `Qto_SpaceBaseQuantities` never holds an area: take the area from the body.

## Which property says whether to trust the area

Read `RvtElementRecordGeometry.SpaceOutlineSource` on the `IfcSpace`:

| Value | The body is | Its plan area is |
|---|---|---|
| `stored_solid` | the outline of the room's stored solid, extruded to the room's height | Revit's room area: within 0.1% of Revit's export on 604 of the 605 `stored_solid` rooms scored below, the office counted at both stages (RE-313 §2, §3) |
| `record_box` | the record's bounding box (turned with a family instance that is turned) | the box's area, not the space's. It equals the space's only when the space is a rectangle; the area is Revit's on none of the office's 24 room scores that keep their box |
| `saved_mesh` | the mesh Revit saved for the room (`rvt-ifc --saved-meshes`) | the mesh's; not scored |
| `none` | no body (an Area) | not available |
| `family_body` | a Revit Space or a family instance written as an `IfcSpace`, drawn as more than its box; its `BodySource` says how | not scored |

A room's `BodySource` is `partition_element_record_bbox` whichever of the first two values it has, because the placement and the height always come from the record's box: read `SpaceOutlineSource`, not `BodySource`. An export from before this property (rvt-rs 0.12.x and earlier) says the same through `ProfileSource` (`partition_room_solid_faces`) and `ProfileResolved`.

The export also counts the spaces drawn as their record's box (rooms and family instances alike), so an importer can check a whole file before reading a space:

- `exported.bounding_box_spaces` in the diagnostics JSON (`rvt-ifc --diagnostics`), out of `exported.by_ifc_type.IFCSPACE`;
- a line in `rvt-ifc`'s summary: "N of M spaces are their record's box".

## Which releases read a room's outline

| Release | Outline | Measured against Revit's export |
|---|---|---|
| 2008 to 2021 | read from the stored solid where the reader finds one | not measured: no reference model with Revit's room export. A Revit 2018 project exports no element at all, and 2018 and 2021 projects decode with no rooms, until PR #1286 lands (#1261) |
| 2022 | stored solid | 8 held-out rooms |
| 2023 | stored solid | 9 of 9 rooms of one project, local only (RE-102) |
| 2024 | stored solid; Areas as spaces without a body | Core Interior, the BIM4LCA office, held-out models (below) |
| 2025 | stored solid | RE1 Architecture, six purpose-made R2025 models, held-out models (below) |
| 2026, 2027 | `record_box` until PR #1288 (RE-314, #1260) reads their stored solids | 16 held-out rooms, all rectangles, so the box is exact; PR #1288 changes this row |

`ROOM_SOLID_SUPPORTED_REVIT_VERSIONS` in `src/research/partition_room_boundaries.rs` is the list of releases whose room solids are read.

## What is measured, and where

`tools/re/space_profiles_vs_ifc.py` meshes both exports, moves them into Revit's internal feet and compares the plan union of each space's upward faces. It prints three figures, each on its own line:

- **area**: the plan area, within 0.1% of Revit's;
- **boundary**: the largest distance from a point of either outline to the other's outline, within 0.01 ft. This is the room's outline, and the figure used for "outline" in the table and in the text below;
- **outline**: the largest distance from a vertex of either mesh's upward faces to the other's outline, within 0.01 ft. Revit meshes a ceiling step inside a room as an upward face of its own, and its vertices count against this one, so it reads some correct outlines as wrong (RE-313 §3). The CI gate pins both.

Measure runs the scorer on every reference model (`-f scorers=space_profiles_vs_ifc`); the IfcOpenShell job of CI runs it on the first two rows, pairing by GlobalId (`--match globalid`), and fails below their counts in area, outline and boundary.

| Model (release) | Spaces in both | Area within 0.1% | Boundary within 0.01 ft | Outline within 0.01 ft | Outline source: count, area, boundary |
|---|---:|---:|---:|---:|---|
| Core Interior (2024) | 116 | 116 | 116 | 116 | 116 rooms, stored solid: 116, 116 |
| RE1 Architecture (2025) | 11 | 11 | 11 | 11 | 11 rooms, stored solid: 11, 11 |
| BIM4LCA office as built (2024) | 276 of Revit's 308 | 221 | 221 | 211 | 220 rooms, stored solid: 220, 220; 12 rooms, record box: 0, 0; 44 family instances, record box: 1, 1 |
| BIM4LCA office at building permit (2024) | 278 of Revit's 308 | 222 | 222 | 216 | 222 rooms, stored solid: 221, 221; 12 rooms, record box: 0, 0; 44 family instances, record box: 1, 1 |
| R2025 purpose-made models with rooms (2025), 6 models | 12 | 12 | 12 | 12 | 12 rooms, stored solid: 12, 12 |
| Held-out models (2022, 2024, 2025), 6 models | 24 | 24 | 24 | 24 | 24 rooms, stored solid: 24, 24 |
| Held-out models (2026, 2027), 4 models | 16 | 16 | 16 | 16 | 16 rooms, record box: 16, 16 (all rectangles) |
| One held-out house | 12 of 13 | 7 | 0 | 0 | 12 rooms, record box: 7, 0 |

A model with no rooms in Revit's export (the other R2025 models, RE1 Electrical, Mechanical and Plumbing) scores 0 of 0.

What the misses are:

- **Rooms whose stored solid does not close on the record's box (12 on each office file)** keep the box, and its area is wrong on all 12.
- **Family instances written as spaces (44 on the office)** are 42 "Turning Free Space" discs and two "VOLUME" families that Revit's export draws from the family's geometry (a mapped representation); rvt-rs draws their record box, a square where the family is a circle (24.22 ft² against 19.01 ft² on the 1500 mm turning spaces listed). No issue covers these yet; #1262 and PR #1279 are about the terrain office's parking bays, a different set, though its rule (a family instance written as a space is left out where its family draws no model form) may change this one too.
- **The office's other 30 spaces are its Areas**: Revit's export gives them a body, rvt-rs writes them without one (RE-191).
- **One room at building permit** is read from its stored solid and is 66% off in area (172.77 against 103.90 ft²); its cause is not found (RE-313 §3).

## What is not measured

- Revit 2008 to 2021 and 2023 on any model Measure can run: no reference model with a Revit room export exists for them in the corpus (and Revit 2018 and 2021 projects decode with no rooms until PR #1286, #1261).
- Revit Spaces (MEP): no measurement singles one out.
- Rooms of Revit 2026 and 2027 beyond the rectangles above (#1260, PR #1288).
- A room's `Height` and `NetVolume` as scored by `tools/re/qtos_vs_ifc.py`, not by this scorer.
- Revit's phase filter: Revit leaves out a room by its phase parameter, which is not measured to be its created phase (docs/status.md).
- Why 12 of the BIM4LCA office's 234 rooms keep their box on each file: the RE-101 probe says their solids do not close on the record's box; what is different about those solids is not found.

## Check it yourself

```bash
rvt-ifc model.rvt -o model.ifc --diagnostics model.diagnostics.json
python3 tools/re/space_profiles_vs_ifc.py model.ifc revit-export.ifc --list 10
```

The second command needs Revit's own IFC export of the same model, IfcOpenShell, NumPy and Shapely (`tools/ci/requirements-ifc.txt`). It prints the counts above, one line each (`area`, `boundary`, `outline`, and the same split by what the space is and its `SpaceOutlineSource`), and for every space off in area or outline, the area on both sides, the distances and `SpaceOutlineSource`.
