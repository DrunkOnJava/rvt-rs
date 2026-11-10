# RE-399: which walls, columns and mullions Revit leaves quantities off, against how Revit draws them and what the saved graphics flag

**Date:** 2026-11-10
**Issues:** #1046 (base quantities Revit computes from its joined geometry, W7), #1016 (quantities); follows RE-268 (the ends of a curtain mullion Revit's solid keeps), RE-291 (a column Revit cuts at an end), RE-387 (the volume of a body drawn from a mesh of flat faces) and RE-388 (a wall's saved mesh)
**Artefacts:** the BIM4LCA office as built and at building permit (Revit 2024, Revit 24.0.5's IFC4 exports, CC BY-SA 4.0, as Measure fetches them) and puzzbobb's 24 Revit 2025 models (Revit 25.0.2's IFC4 exports); revit-ifc at the tag `IFC_v24.1.0` (CLEANROOM.md source 5; the office's exporter, 24.0.5, has no tag)
**Runs:** Measure [42862248369](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42862248369) (`tools/re/wall_set_kind_vs_ifc.py`, commit `cace8149ccf4`, on the code of `main` at `d7354075`, the office and the 24 Revit 2025 models), [42871737720](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42871737720) (`tools/re/column_set_kind_vs_ifc.py`, commit `5242f1f8bfe2`, on `main` at `45cdcd4a`, the office) and [42863856326](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42863856326) (`probes/examples/probe_1046_member_flags.rs`, commit `c234776a5379`, the office; its scorer step, `tools/re/member_flags_vs_ifc.py`, crashed in IfcOpenShell with exit 139, so the counts below are the probe's own output)
**Status:** Negative for walls: whether Revit writes a wall's side areas and gross volume or only its `NetVolume` is not told by the body Revit draws the wall with, on the office or the Revit 2025 models. Negative for mullions: the saved graphics of a curtain mullion hold the six faces of a box and flag none of them hidden or filtered, whichever ends Revit's `NetSurfaceArea` counts. For columns Revit's own body and quantity set agree on every column, which is RE-291's finding read from Revit's side. Nothing rvt-rs writes changes; the scorers' commits are scratch commits, kept under `refs/archive/2026-11-10/cited/`.

## 1. The question

RE-387, RE-388 and RE-268 each stopped at a value Revit's export leaves out where nothing found told which objects it leaves it out on: a column's `GrossVolume` (left out on the 18 columns Revit draws as a cut mesh), the side areas and gross volume of the walls Revit gives a `NetVolume` alone (28 of the office's 607 walls with a quantity set as built, 24 of 589 at permit), and the `NetSurfaceArea` of the 3,397 mullions RE-268's reading of the kept ends does not reach. This asks whether anything in Revit's own file (the body it draws an object with) or in the saved graphics (the flags of a primitive) separates them.

## 2. What Revit's exporter does

Read at `IFC_v24.1.0`, naming files and classes only, no code copied:

- `PropertyUtil.CreateWallBaseQuantities` (RE-270 section 1) writes a wall's `GrossSideArea`, `NetSideArea`, `GrossVolume` and `NetVolume` together, from the solids it is given, and none of them where it is given a mesh; `WallExporter` calls it from its branch for a wall exported as a wall with an axis (an `IfcWallType` export) and, from its other branch, only when it exports by components. A set with a `NetVolume` and none of those areas is therefore not this function's output for a wall with a vertical face. Whether Revit can export a wall's geometry as an extrusion is decided by `ExporterIFCUtils.CanExportWallGeometryAsExtrusion`, which the exporter calls and does not contain.
- The calculators of the quantity definitions (`Exporter/PropertySet/Calculators`) each read a Revit parameter of the element or its symbol first. Without one, `GrossVolumeCalculator` multiplies the area and the length of the extrusion data the element was exported with and gives nothing where there is none, `NetVolumeCalculator` sums the volume of the element's solids and meshes, and `NetSurfaceAreaCalculator` adds the area of every face of each solid.

## 3. Measured

### Walls: the body does not say which wall gets the full set

`wall_set_kind_vs_ifc.py` takes each `IfcWall` of Revit's export that has a `Qto_WallBaseQuantities`, names the kind of its set (the full one, with `GrossSideArea`, `GrossVolume`, `Height`, `NetSideArea` and `NetVolume`; or the volume alone, a `NetVolume` with none of the side areas or the gross volume; no wall has any other) and counts it by the item classes of Revit's `Body` representation and by whether Revit voids it. The reading it tests is the one section 2 suggests: the full set marks a wall Revit exported as an extrusion with an axis, so Revit's body would say so.

| Revit writes | Revit's body | As built | At permit |
|---|---|---:|---:|
| the full set | `IfcExtrudedAreaSolid` | 275 | 294 |
| the full set | `IfcPolygonalFaceSet` (one wall also an `IfcTriangulatedFaceSet`) | 304 | 271 |
| `NetVolume` alone | `IfcExtrudedAreaSolid` | 1 | 4 |
| `NetVolume` alone | `IfcPolygonalFaceSet` | 27 | 20 |

A wall with an extruded body has the full set on 275 of 276 as built and 294 of 298 at permit, but 304 of the 331 walls with a face-set body have it too (271 of 291 at permit), so the body names none of the 28 (24) volume-only walls. Voiding does not either: 252 of the full-set walls are voided as built (250 at permit) and 5 of the 28 (9 of 24). The 138 walls of the 24 Revit 2025 models all have the full set (93 extrusions and 45 face sets). RE-388's gate, a wall with more than four vertical faces gets no side area, stays what it was: the one thing the saved mesh holds that separates them on this project (RE-388 counts 26 volume-only walls among the 581 walls both Revit's export and its probe hold, each with five or more vertical faces), and nothing in Revit's body adds to it.

### Columns: Revit's body and its quantity set agree, as RE-291 found

`column_set_kind_vs_ifc.py` counts each `IfcColumn` of Revit's export by which of `GrossVolume`, `Length`, `CrossSectionArea` and `OuterSurfaceArea` its `Qto_ColumnBaseQuantities` holds, the item class of its body (Revit's column bodies are mapped items, so the class is that of the item in the map) and the item class of rvt-rs's body:

| Revit writes | Revit's body | rvt-rs's body | As built | At permit |
|---|---|---|---:|---:|
| all four | `IfcExtrudedAreaSolid` | `IfcExtrudedAreaSolid` | 80 | 56 |
| all four | `IfcExtrudedAreaSolid` | `IfcTriangulatedFaceSet` | 199 | 271 |
| `CrossSectionArea` alone | `IfcPolygonalFaceSet` | `IfcTriangulatedFaceSet` | 0 | 18 |

No column has any other combination. The 18 columns without the four quantities are the 18 Revit draws as a mesh, and RE-291 found that Revit draws a column as a cut mesh where a slab its pair step names lies across one end of it (all 18 at permit, none of the other 327, none of the 279 as built). rvt-rs's body, a triangulated face set since #1191, is the same for those 18 and for the 271 columns Revit draws as extrusions, so its body class does not carry the difference; the prediction stays RE-291's and RE-353's reading of the slabs. rvt-rs already writes `GrossVolume` on 265 of the 271 mesh-drawn columns Revit gives one (RE-387 sections 2 and 4), and this changes none of that. Whether the saved mesh's shape separates the 18 from the 271, as a box test does for a curtain panel (RE-396), was not measured.

### Mullions: the saved graphics flag no face

`probe_1046_member_flags` decodes the saved graphics of every `IfcMember` rvt-rs writes with the branches the visibility and filter flags hide included, and prints each primitive's flags, area and normal. Its hypothesis was that the end faces RE-268 shows Revit's `NetSurfaceArea` leaves out are the primitives reached through a hidden or filtered branch.

- The office as built has 10,003 members, 10,001 of them with exactly six primitives and 2 with none; at permit 10,031 members, 10,029 and 2. Counted from the probe's `prim` lines, the six normals of 9,965 of those members (9,993 at permit) form the three opposite pairs of a box's faces (a dot product below -0.999); the other 36 hold six primitives that are not a box's. 237 members of each file are `partial_saved_graphics`, decoded in part.
- Of the 60,006 primitives as built (60,174 at permit), none is hidden and none is filtered, and no member has a hidden or filtered primitive.

The saved graphics hold every face of a box-shaped mullion, both ends included, whichever ends Revit's solid keeps, and carry nothing that says which. RE-268's reading from where the boxes of a curtain wall's mullions meet stays the only one, and the 3,397 mullions as built that it does not reach stay without a `NetSurfaceArea`. The flags of the 237 members decoded in part are those of their decoded primitives only.

## 4. Not shown

- **Other projects.** The volume-only walls appear on one project in two states, the office (28 as built, 24 at permit), and the columns Revit draws as a cut mesh on its permit file only; the Revit 2025 models hold no volume-only wall and no column (RE-291).
- **Why Revit's export leaves the areas out of those walls,** and which wall Revit can export as an extrusion: `CanExportWallGeometryAsExtrusion` is inside Revit, and no field of the file or of Revit's export found here predicts it.
- **A column's mesh shape** against the 18 and the 271 (section 3).
- **The 237 members decoded in part:** a flag among their undecoded primitives would not be seen.

Closed by this report: do not re-probe the body class or the voiding of a wall, or the visibility and filter flags of a mullion's saved graphics, as a predictor of what Revit leaves out, without a new signal.

## 5. Reproduce

The scorers come from `--ref`, a branch (`measure/wall-mesh-quantities`, tip `5242f1f8bfe2`, which holds all three; the three commits are also kept under `refs/archive/2026-11-10/cited/<first 12 characters>`, and `git push origin refs/archive/2026-11-10/cited/5242f1f8bfe2:refs/heads/measure/wall-mesh-quantities` restores the branch); `-f ref` is the code that makes the exports.

```bash
gh workflow run measure.yml --ref measure/wall-mesh-quantities -f ref=d7354075c812e6c1727db517e60faaf949f1c5bd -f base=none \
    -f nordic=true -f r2025=true -f models="nordic-office-as-built nordic-office-permit r2025-*" -f scorers=wall_set_kind_vs_ifc
gh workflow run measure.yml --ref measure/wall-mesh-quantities -f ref=45cdcd4a077242af922ed8343df09e2bf6e2fbf3 -f base=none \
    -f nordic=true -f models="nordic-office-as-built nordic-office-permit" -f scorers=column_set_kind_vs_ifc
# The probe comes from -f ref (its scorer crashed, so none is named):
gh workflow run measure.yml --ref main -f ref=c234776a537909c176f7c54c1da758c18723def9 -f base=none \
    -f nordic=true -f models="nordic-office-as-built nordic-office-permit" -f scorers=none \
    -f probe=probe_1046_member_flags -f probe_scorer=-
```
