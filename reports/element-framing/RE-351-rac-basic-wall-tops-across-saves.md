# RE-351: the 16 `rac_basic` walls its saves draw to different heights

**Date:** 2026-11-05
**Issues:** #1471 (16 walls of Autodesk's `rac_basic` differ between its 2025 and 2026 saves by up to 3.19 m: reader defect or project edit); follows RE-267 (a wall's top cut at the underside of the floor that covers it), RE-346 (the same-project census that found them) and RE-211 (a roof by extrusion)
**Probe:** `probes/examples/probe_re351_wall_top_covers.rs`
**Artefacts:** Autodesk's own `rac_basic` sample project, pinned in `research/autodesk-sample-projects.tsv`, fetched by `tools/fetch-autodesk-samples.sh` and never stored: 2025 (`41f1893a…d4293`), 2026 (`bc547fe9…d99dc`), and the other releases the same file lists (2015 to 2027).
**Status:** the walls are a reader defect, not an edit of the project. Their stored data (record box, plan, base and top constraints, joins) is the same on the 2025 and 2026 saves; what differs is a roof, 243274, that the 2025 reader draws as a solid and the 2026 reader draws as its record box, and the wall-top pass takes a record box for a level roof. **This report is not final**: sections 3 and 4 are filled in from the probe's run on every save and from the run that scores the fix; each figure below names its run, and what has no run is labelled.

## 0. The question

16 walls of `rac_basic` are drawn from the same source on its 2025 and 2026 saves with different extents, up to 3.19 m, every one smaller on 2026, six sharing one 3.1865 m offset (#1471, found by RE-346's census on #1454): 428588, 428745, 430064, 506386, 234869, 429964, 430318, 430361, 430412, 430859, 497540, 506797, 768442, 937935, 938974, 939084. Is that the reader or the project? It is already so on `main`.

## 1. The difference is in the walls' height, and only there

The first measurement is the 2025 and 2026 exports of Measure run 42063046730 (head 3149973f, #1454's first head), which kept the IFC files, read here with a script that parses each wall's `IfcExtrudedAreaSolid` and placement (`IfcProduct` Tag = ElementId). It is #1454's code, not `main`'s; section 3 repeats it on `main`'s with the probe.

On all 16 walls the written plan (the profile's world bounds), the base and `RvtElementRecordGeometry.BoundingBoxHeight` are equal on the two saves. What differs is the top: on the 2025 save each wall is drawn up to its record box, and on the 2026 save each is cut at 5.7265 m, with the properties `BodyHeightSource` (`wall_top_cut_at_covering_floor`, and `wall_top_stepped_at_covering_floor` on 428588 and 428745, which keep the part of their plan the cover does not reach) and `TopCoverElementId` 243274. On the 2025 save no wall of `rac_basic`, `rac_advanced` or `rst_basic` carries `BodyHeightSource`; on the 2026 save the only walls of those three that do are these 16. `rme_advanced` and `rme_basic` cut 6 walls each at a slab, on both saves alike. The 16 are therefore every wall of the ten exports (the five projects, 2025 and 2026) that is cut at a roof, and none of the ten exports has a wall cut at its base (`BodyBaseSource`).

| wall | source | base | box top (= the 2025 top) | 2026 top | lower by |
|---|---|---:|---:|---:|---:|
| 428588 | join_trimmed | 2.700 | 6.000 | 5.7265, and 6.000 beyond the roof's plan | 0.2735 |
| 428745 | join_trimmed | 2.200 | 6.000 | 5.7265, and 6.000 beyond the roof's plan | 0.2735 |
| 430064 | join_trimmed | 3.000 | 7.4728 | 5.7265 | 1.7463 |
| 506386 | join_trimmed | 3.000 | 8.1103 | 5.7265 | 2.3838 |
| 234869 | layer_joins | 3.000 | 8.9131 | 5.7265 | 3.1865 |
| 429964 | layer_joins | 3.000 | 8.9131 | 5.7265 | 3.1865 |
| 430318 | layer_joins | 3.000 | 8.9131 | 5.7265 | 3.1865 |
| 430361 | layer_joins | 3.000 | 7.6447 | 5.7265 | 1.9181 |
| 430412 | layer_joins | 3.000 | 7.9525 | 5.7265 | 2.2260 |
| 430859 | layer_joins | 3.000 | 8.9131 | 5.7265 | 3.1865 |
| 497540 | layer_joins | 3.000 | 8.8449 | 5.7265 | 3.1184 |
| 506797 | layer_joins | 3.000 | 8.9131 | 5.7265 | 3.1865 |
| 768442 | layer_joins | 3.000 | 7.9525 | 5.7265 | 2.2260 |
| 937935 | layer_joins | 3.000 | 7.5292 | 5.7265 | 1.8027 |
| 938974 | layer_joins | 3.000 | 8.9131 | 5.7265 | 3.1865 |
| 939084 | layer_joins | 3.000 | 8.2088 | 5.7265 | 2.4823 |

(Metres, from the exports of run 42063046730; `join_trimmed` is `partition_element_record_join_trimmed` and `layer_joins` `partition_wall_centreline_layer_joins`, 4 and 12 walls.) The six walls that are 3.1865 m lower are the six whose box top is 8.9131 m: the box's top is the wall's highest point, and 5.7265 m is the cover's underside.

## 2. The cover is a roof the two saves draw differently

On the 2026 save each of the 16 walls' top join or cut pairs names roof 243274 (`Basic Roof: SG Metal Panels roof`, `IfcRoof`), which is its `TopCoverElementId`; whether the 2025 save's records name it too is read in section 3. Its record box is 19.5 m by 6.36 m and 3.472 m deep, from 5.7265 m on both saves (`BoundingBoxHeight` 3.47216). The 2025 save draws it as a faceted solid (`BodySource` `partition_roof_extrusion`, RE-211: a roof by extrusion, its section read from its sketch lines); the 2026 save draws it as its record box (`partition_element_record_bbox`, an `IfcExtrudedAreaSolid` over the box). The roof has no material layer set in either save's export; `attach_slab_layers` gives a roof its type's layers only where they add up to its record box's height, so here either its type's layers do not add up to 3.472 m or they are not read.

`attach_extrusion_roofs` runs only on the releases in `SKETCH_SPLINE_SUPPORTED_REVIT_VERSIONS`, which is 2025 alone, so no other release reads a roof by extrusion. The wall-top pass (`ifc::export_content::wall_tops`, RE-267) cuts a wall's top at the underside of the slab, roof or footing its top join names where that element is drawn as a plain extrusion, whose underside it takes to be level across the wall: a roof drawn from its slopes or its extrusion is a solid and is left out. A roof that is its record box is a plain extrusion, so on 2026 it is a cutter, and its underside is the bottom of its box, 3.47 m below the box's top.

That is a reader defect. The cut is a statement that Revit's wall stops at a level; for a roof whose shape is not read, nothing says the box's underside is one. On the 2025 save the same roof, read, is not a cutter, and the walls are drawn to their record boxes, as every wall is whose cover the pass does not take.

## 3. Every save of `rac_basic`

Not yet read. Measure run 42106436497 (head b2b5716e, `main` with the probe) runs the probe on every `rac_basic` save, 2015 to 2027. This section holds, per wall and save, the record box, base and top Levels, centreline and joins from the file's own bytes, and the written top.

## 4. The change

A roof counts as a cutter of a wall's top only where it is one of the roofs whose record box is exactly its type's layers (`m_slab_layers`, the field `attach_slab_layers` gives a floor, roof or ceiling whose layers add up to its record box's height, RE-57). A plate of that kind is level by construction; a roof with a deeper box, or whose layers are not read, is left out and the wall keeps its record box. Slabs and footings are as before. The base pass (#1384) is as before: it takes a roof's top as a base level on the same grounds, which has the same flaw for a wall that stands on a roof drawn as its box; none of the ten exports of section 1 has a wall cut at its base, and no model has been checked for one.

Not yet measured on this head.

## 5. Not measured

- A Revit export of a `rac_basic` save: none exists to compare with. Autodesk's samples have no export from Revit, and three.js's IFC2X3 export of `rac_advanced` is counts only. The walls under roof 243274 therefore have no oracle of their own; the 2025 save's drawing of them, a wall to its record box, is the earlier save's, and is not Revit's cut by an arch.
- Whether a flat roof that cuts a wall on the BIM4LCA office as built or at permit is still cut: not yet measured on this head.

## 6. Reproduce

```bash
tools/fetch-autodesk-samples.sh samples 2015 2027
cargo run --profile ci -p rvt-probes --example probe_re351_wall_top_covers -- samples/2026-racbasicsampleproject.rvt > 2026.jsonl
```
