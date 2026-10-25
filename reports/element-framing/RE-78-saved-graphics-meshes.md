# RE-78 — Family-instance bodies from the meshes Revit saved for display

**Date:** 2026-09-27
**Issues:** #255, #227
**Credit:** rosejn (#255: the native object-graph and saved-graphics reader, `native_saved_scene` and the modules under it, extracted with their authorship).

**Result:** positive for doors, shading devices, columns, members, plates, furniture and most fixtures; negative for windows. Opt-in.

- A Revit file keeps the graphics it last drew for each element. rosejn's reader decodes them per ElementId in document feet, at a chosen detail level (Fine here).
- `saved_meshes::attach` replaces every body that is only its element record's bounding box (`BodySource` `partition_element_record_bbox`, no resolved profile) with the element's saved mesh, in the element's own placement. Elements drawn from decoded data keep their bodies, and so do aggregate wholes (stairs, curtain walls), whose parts are already drawn. A Tag carried by several entities is left alone.
- The mesh is written as one `IfcCartesianPointList3D` and one `IfcTriangulatedFaceSet` per element. As `IfcFacetedBrep` Snowdon's IFC was 456 MB; as face sets it is 111 MB.

## Measured

`tools/re/saved_meshes_vs_ifc.py` compares each replaced element's world box with the box of the same Tag in Revit's own IFC export (IfcOpenShell 0.8.5, `IfcOpeningElement` skipped on both sides). Snowdon's export is in shared coordinates: internal feet are rotated 26.0156° and translated (1370151.83, 258247.98, 780.5) ft, and the published transform's rounding leaves one file-wide residual of (0.0347, 0.0038, 0) ft, which is removed. The column "box" scores the record boxes these bodies replace, on the same elements, with the same offset.

| file | category | n | within 0.01 ft, saved mesh | within 0.01 ft, box |
|---|---|---:|---:|---:|
| Core Interior (2024) | IfcDoor | 132 | 132 | 0 |
| | IfcShadingDevice | 20 | 20 | |
| | IfcWindow | 6 | 6 | |
| Snowdon Towers (2024) | IfcMember | 1,595 | 1,565 | 641 |
| | IfcPlate | 484 | 461 | 228 |
| | IfcBuildingElementProxy | 401 | 345 | 73 |
| | IfcLightFixture | 388 | 289 | 0 |
| | IfcFurniture | 212 | 198 | 47 |
| | IfcDoor | 132 | 131 | 0 |
| | IfcSanitaryTerminal | 111 | 68 | 22 |
| | IfcColumn | 110 | 110 | 56 |
| | IfcWindow | 68 | 2 | 0 |
| | IfcCovering | 68 | 68 | 51 |
| | all replaced | 3,736 | 3,332 | 1,138 |

Core Interior's 116 rooms also take their saved mesh: the room volume, with the same extents as the record box but the room's own outline (up to 31 triangles, not 12). Revit's reference export carries no spaces to score them against.

Snowdon Towers: 3,789 of 4,744 bounding-box bodies replaced, 2.55 M triangles, 37 s and 5.6 GB peak memory for the export (`--profile ci`). Without the option the IFC is 12 MB.

## Residuals, measured

- **Windows** (66 of 68 on Snowdon): the saved mesh is 0.44 ft short on one side and 0.88 ft on the other in plan, the same on every instance of a type. Revit's export draws parts the Fine-detail saved mesh does not. Not the detail level: at Coarse and Medium the same 2 windows agree, while doors (127 of 132) and light fixtures (248 and 252 of 388) agree less than at Fine (131, 289), which is why Fine is used.
- **Light fixtures** (90): the saved mesh is 0.58 ft taller than Revit's export.
- **Members** (30): exactly Revit's shape, 13.33 ft lower. Probably graphics saved relative to a group; not resolved.
- **Railings, sanitary terminals, proxies**: 23, 17 and 39 over 1 ft.
- 23 plates have no Tag in Revit's export.

## Not claimed

- Default export: the option is off because of time and size.
- Revit 2025: when this report was written the reader declined the file ("native record framing is unvalidated for Revit 2025", RE1 Architecture). It reads RE1's four Revit 2025 models now (Measure run 37231768480; §Shapes below). Older releases are not measured.
- The opening a door or window cuts is still its box (#227).

## Shapes against Revit's export (2026-10-25)

A world box can agree while the shape inside it does not. `tools/re/mesh_bodies_vs_ifc.py` meshes each body drawn from a saved mesh and Revit's body for the same Tag with IfcOpenShell, each in its file's own site frame in internal feet, and compares the volume and how far each body's vertices lie from the other's surface, both ways. A class with more than 300 such bodies is scored on an even stride of 300 by Tag, and at most 400 vertices of a body are tested. Measure run [40192112285](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40192112285), `main` at a1b1e24c with `--saved-meshes`, `-f nordic=true`. "Exact" is every tested vertex of each body within 0.001 ft of the other's surface; the volume of nearly all of those agrees to 0.01%.

| file | bodies scored | exact | by class (exact of scored) |
|---|---:|---:|---|
| Core Interior (2024) | 138 | 138 | IfcDoor 132/132, IfcWindow 6/6 |
| BIM4LCA office as built (2024) | 1,709 | 1,294 | IfcWindow 269/269, IfcPlate 260/260, IfcColumn 199/199, IfcMember 294/294 of 9,678, IfcElectricAppliance 50/50, IfcFireSuppressionTerminal 40/40, IfcBuildingElementProxy 12/12, IfcCovering 12/12, IfcRamp 4/4, IfcFurniture 68/83, IfcDoor 48/269, IfcSanitaryTerminal 34/100, IfcWasteTerminal 0/51, IfcLightFixture 0/43, IfcRailing 0/18 |
| BIM4LCA office at building permit (2024) | 1,797 | 1,376 | the same pattern |
| RE1 Architecture (2025) | 56 | 45 | IfcDoor 5/5, IfcMember 10/10, IfcPlate 2/2, IfcFurnishingElement 18/23, IfcBuildingElementProxy 8/9, IfcFlowTerminal 2/7 |
| RE1 Electrical (2025) | 49 | 31 | IfcDistributionControlElement 5/5, IfcBuildingElementProxy 22/30, IfcFlowTerminal 4/14 |
| RE1 Mechanical (2025) | 68 | 65 | IfcFlowSegment 25/25, IfcFlowTerminal 7/7, IfcFlowFitting 32/35 |
| RE1 Plumbing (2025) | 41 | 7 | IfcFlowTerminal 6/6 (volume within 0.05%), IfcFlowFitting 1/35 (median volume 1.22 times Revit's) |

Where a saved mesh is exact it is Revit's body to the export's own precision: the Core Interior doors agree to 0.000003 ft both ways. The office's windows are exact on all 269, where Snowdon's differed (§Residuals); that difference is the families', not the reader's.

Where a mesh is not exact, it is mostly part of the body: its own vertices lie on Revit's surface while Revit's reach well past it, and its volume is a fraction of Revit's (the office's `Sense_Public_allas` sinks 0.3%, `WC-istuin` toilets 9.5%, RE1's dining chairs 6.4%). Nothing in the file found so far tells such a mesh from a complete one before comparing it with Revit's export, so the option stays opt-in and experimental.

