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

- **Windows** (66 of 68 on Snowdon): the saved mesh is 0.44 ft short on one side and 0.88 ft on the other in plan, the same on every instance of a type. Revit's export draws parts the Fine-detail saved mesh does not.
- **Light fixtures** (90): the saved mesh is 0.58 ft taller than Revit's export.
- **Members** (30): exactly Revit's shape, 13.33 ft lower. Probably graphics saved relative to a group; not resolved.
- **Railings, sanitary terminals, proxies**: 23, 17 and 39 over 1 ft.
- 23 plates have no Tag in Revit's export.

## Not claimed

- Default export: the option is off because of time and size.
- Revit 2025: the reader declines the file ("native record framing is unvalidated for Revit 2025", RE1 Architecture), and the export goes ahead with its boxes and says so. Older releases are not measured.
- The opening a door or window cuts is still its box (#227).
