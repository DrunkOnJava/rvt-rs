# RE-296 — A space's NetVolume

**Date:** 2026-10-30
**Issues:** #1016 (quantities), W7 of [`docs/release-1.0-criteria.md`](../../docs/release-1.0-criteria.md); follows RE-255 (a space's Height) and RE-101 (room outlines from their stored solids)
**Artefacts:** the BIM4LCA office as built and at building permit (Revit 2024, Revit 24.0.5's IFC4 exports) and puzzbobb's R2025 models with rooms (12 to 17, Revit 2025), as Measure fetches them; Core Interior and RE1 Architecture, whose exports have base quantities off (RE-242)
**Runs:** Measure [41137912626](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41137912626) (`probes/examples/probe_re296_space_prisms.rs`), [41139774401](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41139774401) (the quantity census) and [41140315951](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41140315951) (this change against `main`, unfiltered)
**Result:** positive, narrowed: where a room's stored solid is a prism of its outline, Revit's `Qto_SpaceBaseQuantities.NetVolume` is the outline's area, its holes taken off, times the height of the room's box, and is written there; none different. `GrossVolume`, the volume of a room whose ceiling steps, and the area quantities of Revit 25's export stay unwritten.

## 1. What Revit writes

revit-ifc's `NetVolumeCalculator` and `GrossVolumeCalculator` (CLEANROOM.md source 5, read at v24.2.0; the office's exporter is 24.0.5.432) give a space its `NetVolume` as the volume of the solids and meshes of its geometry, the room's closed shell, and its `GrossVolume` as the area of the largest boundary loop times the length of the extrusion that draws the room, which exists only where `SpatialElementExporter` could export the shell as an extrusion. On the office as built Revit's export draws 308 spaces so:

| body | spaces | quantities |
|---|---:|---|
| tessellation (`IfcPolygonalFaceSet`) | 142 | `Height`, `NetVolume` |
| extrusion | 92 | `Height`, `GrossVolume`, `NetVolume` |
| mapped extrusion (the `Turning Free Space` accessibility volumes and 2 `Volume` spaces) | 44 | `GrossVolume`, `NetVolume` |
| extrusion of the boundary, no closed shell | 30 | `Height` |

## 2. Which rooms are prisms

RE-101 reads a room's stored solid as planar faces. `probe_re296_space_prisms` prints each room's faces as horizontal planes, vertical faces with the heights they span, and any other, with the room's outline and box. Of the 278 office spaces that have a `NetVolume`:

- 221 have an outline that closes on the room's record box. On 167 of them every vertical face runs the whole way from the box's floor to its ceiling, no face is neither vertical nor horizontal and no horizontal face lies between the two: the outline's area, holes taken off, times the box's height is Revit's value on all 167, none different. At building permit it is 178 of 178.
- The other 54 (44 at permit) have a vertical face that stops short of the ceiling, a step (0.82 ft short at least), and the same product is Revit's on 1 of them, a room whose ceiling has a strip 0.0016 ft wide, 1.5e-4 m3 of its 14.3 m3. The stored faces carry a box in their own planes and not their outlines: between the heights where a face starts or ends, the vertical faces' edges close into loops for 3 of the 54, and give Revit's volume for 1.
- 13 (12 at permit) have an outline that does not close on the box (RE-101 §3), and 44 are the mapped volumes, whose solids are not in the room's partition.

No room that passes the test is one Revit gives no `NetVolume`: all 167 as built, and 178 at permit, have it.

## 3. What rvt-rs does

`prism_of_solids` (`partition_room_boundaries`) decides it from the faces of the solid whose outline `attach_room_outlines` keeps, and `attach_room_outlines` marks the room (`m_room_prism`). `add_space_net_volume` writes `NetVolume` as the area of the outline body (RE-101) times the box's height on a marked room.

## 4. Measured

This change against `main` (Measure 41140315951, unfiltered, `-f nordic=true -f r2025=true`, every scorer):

| model | `NetVolume` equal | different | Revit does not write |
|---|---:|---:|---:|
| office as built | 0 → 167 of 278 | 0 | 0 |
| office at permit | 0 → 178 of 278 | 0 | 0 |
| R2025 12 to 17 | 0 → 2 of 2 each | 0 | 0 |

RESULTS_PLACEHOLDER

## 5. GrossVolume

Of the 167 prisms, Revit draws 88 as an extrusion (a `GrossVolume`; the outer loop's area times the height is Revit's on 84 of the 88, and Revit's is the net volume or between the two on three rooms with holes and on a room of none) and 79 as a tessellation (none; at permit 109 and 69). A rule writing `GrossVolume` on the prisms would write 79 values Revit's export omits. The number of horizontal faces stored (0, 1 or 2), the holes, the wall faces, the area, the height and every field of the room's native record were compared between the two groups: none separates them (the fields that are disjoint are the ElementIds and the numbers derived from them). It stays unwritten.

## 6. What stays open

- `NetVolume` of the 111 spaces as built (100 at permit) that are not prisms: 54 with a stepped ceiling, 44 mapped volumes (their solids are not in the room's partition) and 13 whose outline does not close on the box.
- `GrossVolume`, 136 as built.
- The area quantities `GrossFloorArea`, `NetFloorArea` and `GrossCeilingArea`: Revit 25's export writes them on both rooms of every R2025 model with rooms, each the outline's area; the office's 24.0.5 exports write none on any of its 308 spaces, so a rule writing them would count 924 values the office's export leaves out. Nothing in a file says which exporter will read it.

## 7. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=SHA_PLACEHOLDER -f base=main -f nordic=true -f r2025=true
gh workflow run measure.yml --ref main -f ref=2219ae1631d6eb65b0d3b3a7b63c7ee6e2c1d6a4 -f base=none -f nordic=true -f models="nordic-office-*" -f scorers=none -f probe=probe_re296_space_prisms -f probe_scorer=-
```
