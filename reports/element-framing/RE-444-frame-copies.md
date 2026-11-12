# RE-444: the frame the exporter keeps for an element framed in several partitions is the one with the greatest height, which can be the older one

**Date:** 2026-11-12
**Issues:** #1710 (W7: what is left of the columns' base quantities, point 2: two timber-office columns whose `Length` differs from Revit's while the extent of their mesh equals it), #1046 and #1016 (W7: the BIM4LCA office's quantities); follows RE-22 (one frame per ElementId: the greatest height), RE-26 (the newest frame is the current state) and RE-216 (the current copy of an element's data)
**Artefacts:** the BIM4LCA timber office at building permit (Revit 2024) with Revit's IFC4 export; the other BIM4LCA files; Autodesk's `rme_advanced_sample_project` and `rme_basic_sample_project`, saved by Revit 2024, 2025, 2026 and 2027, which no Revit export covers
**Scorers:** `probes/examples/probe_re444_frame_copies.rs` and `tools/re/frame_copies_vs_ifc.py` (new), `tools/re/length_vs_mesh_extent.py` (new), `tools/re/qtos_vs_ifc.py`, `tools/re/psets_vs_ifc.py`
**Runs:** to be filled (section 5)
**Status:** Pending its Measure. A probe and two scorers; no change to what the exporter writes.

## 1. The question

The BIM4LCA timber office at building permit holds 18 `Qto_` values and 52 `Pset_` values that differ from Revit's export, the most of any of the 126 exports (the other three office files hold 1 and 1 each). Among them are the `Length` of 3 mullions and 2 columns and the `Span` of the same 3 mullions, each longer than Revit's by 0.05, 0.123 or 0.083 m, and the mesh Revit saved for each has Revit's length. #1710 asks whether a mesh extent may replace a held `Length` when the two disagree. A first answer, replacing the `Length` from the mesh, would fix those 8 values and leave the cause. What is the cause?

## 2. The offline reading

**The five wrong lengths are the box of an older frame.** `tools/re/length_vs_mesh_extent.py` sets, for every `IfcColumn` and `IfcMember` of rvt-rs's export whose body is a face set drawn from the saved mesh, the written `Length` against the extent of the mesh. On the 101 exports whose IFC Measure 43115049478 kept, a written `Length` is never shorter than the mesh by 2 mm, and it is longer by more than 2 mm on 5 elements of the BIM4LCA files, all on the timber office at permit: the mullions 3024546, 3027086 and 3029162 (3.75, 4.27 and 0.75 m for meshes of 3.70, 4.22 and 0.70 m, which are Revit's `Length` and, for the `Span`, 3.70, 4.22 and 0.70) and the columns 1929459 and 3478460 (5.123 and 3.625 m for meshes of 5.000 and 3.542 m, Revit's `Length`). The 15,561 elongated mullions and 1,140 columns of the four office files agree with their meshes to 2 mm everywhere else.

**Their frames are in an older partition than their current record.** The MCP dump of each export (`provenance`) names the stream of the element's frame, the record whose box the exporter keeps, and the stream of the element's current native record. On the two as-built office files the two streams are the same on every row (12,974 and 10,865 of them). On the timber office at permit 999 of 10,825 rows have a frame in another stream than their current record, on the concrete office at permit 435 of 13,020. The five are of the 999: 3024546, 3027086 and 3029162 are framed in `Partitions/374` and current in `Partitions/424` (2 copies), 3478460 likewise (3 copies), 1929459 framed in `Partitions/395` and current in `Partitions/424`. The two walls whose `Width` differs (1436343 and 2114950, 0.176 against Revit's 0.158 m) and the walls that host the openings whose `Depth` differs (1347199, 1347201 and 1493966) are framed in `Partitions/374` and current in `Partitions/424` too. 3027671, the one mullion whose `Length` differs and is not longer than its mesh, is framed in the current partition: its 4.269995 m against Revit's 4.270047 m is the mesh's own single precision (the mesh gives 4.269998). And 3029161, whose `NetSurfaceArea` is one end face (0.0025 m²) short, is framed in the current partition but its partner 3029162 is the stale one, and the boxes of the neighbours decide which ends are hidden, so the missing end face may follow from it.

**The exporter keeps the greatest height.** `select_instance_records` (`partition_schema_mvp/records.rs`) keeps, of the frames of one ElementId, the one with the greatest vertical extent and, on a tie, the newest. RE-22 chose that over "the first by partition" for slabs (79 of 80 against 67 of 80, the first frame of 12 plates seeing only their topping), and RE-26 found the newest frame to be the current state of the walls and doors of Core Interior, where no element's frames disagree in height. Neither compared the newest frame with the greatest where they differ. An element that was made shorter after an earlier save has its longer frame in the older partition, and the greatest height keeps it.

**The Autodesk samples show the same.** The 2025 to 2027 saves of `rme_advanced` and `rme_basic` hold a `Length` on 131 columns each (46 of 3.5 m, 41 of 7.3 m, 41 of 10.6 m and 3 of 7.0 m) for meshes of 3.405828, 3.5, 3.3 and 3.2 m, where the 2024 saves of the same projects, with the same ElementIds and the same meshes, hold the meshes' extents. The `BoundingBoxHeight` the export records for the first of them, 573295, is 3.405828 in the 2024 save and 3.5 in the 2025 one. No Revit export covers these files, so this is a disagreement between two saves of one project, not a measured error, but a project saved again does not change its columns' heights.

## 3. The probe and the scorer

`probe_re444_frame_copies` lists, for every element of the product categories, walls, floors, doors, windows, columns, framing, mullions, panels, ceilings, roofs, ramps and stairs that has more than one frame of an exported instance, each frame's partition, offset and box, whether the boxes and the heights disagree, the frame the selector keeps, the newest, and the partition `native_document::current_partitions` names current. `frame_copies_vs_ifc.py` meshes the element in Revit's export, takes its vertical range (no rotation about the site's vertical changes it) and says which frames agree with it to 2 mm. The decisive population is the elements where the frame kept is not the newest.

## 4. The rows (registered on #1636 before the run)

Probe run on the BIM4LCA files, Core Interior, RE1, `rac_advanced` 2015 to 2020, BIMcollab's model and the 24 Revit 2025 models:

1. **The five.** On the timber office at permit the frame kept is not the newest for 3024546, 3027086, 3029162, 1929459 and 3478460, and the newest agrees with Revit's height on all five and the frame kept on none.
2. **The control.** On the two as-built office files no element's frames disagree in height where the frame kept is not the newest; and the elements of Core Interior whose frames disagree in height, if any, have the frame kept as the newest.
3. **The rule.** Over every model with a Revit export, in the decisive population (the frame kept is not the newest) the newest frame agrees with Revit's height on at least 90% of the elements and on more than the frame kept does, on every model that has such elements; the partition named current is the newest frame's partition on at least 95% of the elements whose frames disagree in height, and the ones where it is not are named.
4. **If the rows hold, the change** (the next pull request): the newest frame, or the current partition's, is kept before the greatest height is compared. If row 3 fails on a category, the rule is narrowed to the categories where it holds before any change.

## 5. What Measure found

To be filled.

## 6. Not done

- The change itself, and its Measure against `main`: unfiltered with `holdout=true`, every `ifc.sha256` that moves read element by element.
- A first plan of this report, to take a mullion's or column's `Length` from the saved mesh where the box overruns it, fixes 8 values and leaves the stale frame's type and quantities on the same elements; it is set aside, with its patch, in favour of the cause.
- 3027671's `Length` (the mesh's own 0.05 mm) stays.

## 7. Reproduce

```
gh workflow run measure.yml --ref <branch with the scorers> -f ref=<probe sha> -f base=none -f scorers=none -f nordic=true -f r2025=true -f bimcollab=true -f racadv=true -f models="nordic-* core-interior re1-* racadv-* bimcollab-* r2025-*" -f probe=probe_re444_frame_copies -f probe_scorer=frame_copies_vs_ifc
python3 tools/re/length_vs_mesh_extent.py <rvt-rs model.ifc> [...]
```
