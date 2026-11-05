# RE-357 — Where Revit places a door's or window's origin

**Date:** 2026-11-05
**Issues:** #1465; follows RE-347 (the axes, #1447), RE-343 (the flips), RE-87 (instance transforms), RE-215 (symbol cut loops) and RE-167 (type GlobalIds)
**Probe:** `probes/examples/probe_re343_door_flips.rs` (unchanged)
**Scorer:** `tools/re/door_placements_vs_ifc.py` (the `origin` lines, new)
**Status:** negative for a rule; the origin stays at the record box's centre. Revit's placement origin is not the stored origin on any door or window set in a wall. Its offset from the stored origin is one value for every element of an `IfcDoorType` or `IfcWindowType` entity (92 of 92 type entities that hold a walled door or window in the three models scored), which is the signature of revit-ifc's per-type `StyleTransform`, taken from the one instance that creates the type and not recorded in the file. Two of its three axes follow the element's own opening (x on 462 of 462 and z on 462 of 462 walled doors and windows of the office as built); the third, across the wall, does not (y on 187 of 193 doors and 99 of 269 windows), so no rule with 0 wrong exists on all the walled elements. The one readable condition under which all three axes agree, both flips set, holds on 232 of 600 walled elements and is reported in section 4, not written. Where Revit's export gives no opening the origin is the stored origin (75 of 75 doors of the office as built and 1 of 1 of RE1 Architecture). **Not yet measured on a Measure head:** every figure below was scored locally on the kept IFCs of Measure run 42066757105 (see section 1); the Measure run of this change is cited in section 7 once it has finished.

## 0. The question

#1465: rvt-rs writes a door's or window's `IfcLocalPlacement` origin at its record box's centre (`location_feet`, z at the box's base), and Revit's is more than 0.5 ft from it on almost all (RE-347 section 4). Is the stored origin (`InstanceTransform.origin`, RE-87) Revit's origin outright? If not, is the offset constant per family or type, a family property, or derived from a stored value, and is there a rule with 0 wrong on the elements set in a plain wall?

## 1. What was measured

`tools/re/door_placements_vs_ifc.py` pairs, by Tag, each door and window of `probe_re343_door_flips` with the `IfcDoor` or `IfcWindow` of Revit's export and of rvt-rs's, and moves every origin and axis into Revit's internal frame through that file's own `IfcSite` placement (RE-347 section 1). For each element whose stored transform the probe read, `d` is Revit's placement origin less the stored origin, taken in the stored axes and in Revit's own placement axes. The scorer then prints, by kind and binding:

- how many elements have `d` of 0 (the stored origin is Revit's);
- how many groups hold one `d` (the type entity: the `IfcDoorType` or `IfcWindowType` entity the export gives the element, the symbol, the symbol and the flip state);
- the signs of `d` by flip state;
- per axis, how many `d` equal the corner of the opening each export gives the element. The opening is the `IfcOpeningElement` the element fills, meshed in world coordinates and expressed in Revit's placement axes about the stored origin; its corner is the lowest corner where the two flips are equal and the negated highest x and y with the lowest z where they differ.

The files, all scored locally with IfcOpenShell 0.9.0.post1 (the version `tools/ci/requirements-ifc.txt` pins): the probe output and rvt-rs's own export of each model are from Measure run 42066757105 (head `eb6de281`, the code before RE-347's rule; the rule moves axes and not origins), and Revit's exports are `2024_Core_Interior_slim.ifc` (sha256 `bfdf36ff…`, Revit 2024), `RE1-Architecture.ifc` (`a9b5d366…`, Revit 2025) and the BIM4LCA office as built, `ARK_NordicLCA_Office_Concrete_As-Built_Revit.ifc` (`b2f530eb…`, Revit 2024; research/bim4lca-measure.tsv). The office at permit is the same building and is left to the Measure run. Held-out models are not in this change's runs.

## 2. The stored origin is not Revit's

| model | doors bound to a plain wall | windows bound to a plain wall | stored origin within 0.001 ft of Revit's |
|---|---:|---:|---:|
| Core Interior | 132 | 6 | 0 |
| RE1 Architecture | 4 | 0 | 0 |
| office as built | 193 | 269 | 0 |

`d` is 0.5 ft or more on every one of them. The doors rvt-rs binds to no plain wall are different (section 5).

## 3. The offset belongs to the type entity

In every model, `d` is one value for all the elements of an `IfcDoorType` or `IfcWindowType` entity, in the stored axes and in Revit's, and it is not one value per symbol:

| model | type entities, doors | with one `d` | symbols, doors | with one `d` | type entities, windows | with one `d` | symbols, windows | with one `d` |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| Core Interior | 10 | 10 | 3 | 0 (stored axes), 1 (Revit axes) | 1 | 1 | 1 | 1 |
| RE1 Architecture | 4 | 4 | 3 | 2 (stored axes), 3 (Revit axes) | 0 | 0 | 0 | 0 |
| office as built | 67 | 67 | 33 | 18 | 10 | 10 | 6 | 4 |

Of the type entities that hold more than one element (9 in Core Interior, 33 doors and 9 windows in the office), every one has one `d`. Per symbol and flip state the doors are 47 of 60 in the office and the windows 11 of 11. 
That is what revit-ifc does (CLEANROOM.md source 5; read for behaviour, not copied). In `FamilyInstanceExporter` (`Source/Revit.IFC.Export/Exporter/FamilyInstanceExporter.cs`), where a door or window's type is created (the type key is not in the cache, or the instance uses its own geometry), the exporter asks the Revit API for the door or window transform of that instance, its symbol and its two flips (`ExporterIFCUtils.GetTransformForDoorOrWindow`, a RevitAPI name whose body is not read) and keeps its inverse as the type's `StyleTransform` (`FamilyTypeInfo.StyleTransform`, `Source/Revit.IFC.Export/Utility/FamilyTypeInfo.cs`). Every instance placed with that type then has its own transform multiplied by that one `StyleTransform`, and the product's origin is the `IfcLocalPlacement` origin. So Revit's origin is the stored origin plus a type-level offset taken from the instance that created the type. The file records neither which instance that was nor Revit's type key (RE-343 section 3 reached the same limit for the 49 inverted doors that rvt-rs binds to no wall).

## 4. Which part of the offset a stored value gives

Against the opening Revit's own export cuts for the element, `d` equals the opening's corner as defined in section 1 on all three axes in Core Interior (132 of 132 doors, 6 of 6 windows) and RE1 Architecture (4 of 4 doors), and per axis in the office as built:

| office as built, within 0.001 ft | x | y | z | all three |
|---|---:|---:|---:|---:|
| doors, 193 | 193 | 187 | 193 | 187 |
| windows, 269 | 269 | 99 | 269 | 99 |

- **x and z** are the element's own opening: x is the stored frame's lowest x (the Revit-frame lowest x where the flips are equal and the negated highest x where they differ), z is the opening's bottom, which is a door's base and a window's sill. Core Interior and RE1 cannot tell the two x conventions apart (their openings are symmetric about the stored origin); the office can, since its openings are asymmetric about it by 0.3 to 0.7 ft in x.
- **y**, across the wall, is not. On the doors it is the opening's corner on 187 of 193; the other 6 are each the only element of their type entity, in openings 1.6 to 3.0 ft deep, and `d` is off by up to 2.585 ft. On the windows it is the corner on the 99 whose two flips are both set; on the 26 with neither set `d` is the wall's other face, and on the 144 with unequal flips `d` is not a face of the opening at all (1.552 ft beyond the lower-y face on four type entities, 140 windows, and 0.781 ft on the fifth, 4 windows). The windows' openings are 1.3 to 2.1 ft deep.
- **Both flips set.** The elements with both `m_flippedX` and `m_flippedY` set agree with the opening's corner on every axis wherever they occur: 60 of 60 doors and 99 of 99 windows in the office, 66 of 66 doors and 6 of 6 windows in Core Interior, 1 of 1 in RE1 Architecture (232 of the 600 walled elements; the flip state is a stored value a reader can check). The other states do not: doors with neither set are right on 30 of 31 and with unequal flips on 97 of 102 (the y of the 6 above), and windows on none of 170. This is reported and not written as a rule. The mechanism of section 3 gives no reason the condition should hold where a type's later instance stands in a different wall from its creator; the 6 doors that disagree are in states 00, 01 and 10; the other 368 elements would keep the box's centre, so one type would have two origin conventions; and the held-out models, which can test it, were not in this run.
- **rvt-rs's own opening** is not Revit's on every door either. Used in place of Revit's, it gives x on 190 of 193 doors (up to 4.199 ft off), y on 187 and z on 192 (all three on 186), and the same figures as Revit's on the windows. So even the two axes that follow the opening would not be written with 0 wrong from rvt-rs's reading of it.

## 5. The doors rvt-rs binds to no plain wall

Of the 76 doors of the office as built that rvt-rs binds to no plain wall, and the 1 of RE1 Architecture, the stored origin is Revit's origin (within 0.001 ft) on 75 of 76 and on 1 of 1. The 75 are the doors Revit's export gives no opening, and the 1 of RE1 has none either (76 of 76 with no opening are at the stored origin); the one other of the office has an opening in both exports, and `d` is its corner on all three axes. So where neither export has an opening for a door, the origin is the stored origin, and where one exists the offset is as in section 4. This is a measurement for the issue about those doors (the 160 of RE-343, 49 of them inverted); nothing is written for them here. `d` is also one value per type entity for them (29 of 29 type entities in the office), and 0 for all but the one.

## 6. Decision and what it leaves

The origin stays at the record box's centre (`location_feet`). A rule needs 0 wrong on the walled elements, written only where it reads; x and z are readable, y is a type-level constant from an unrecorded instance and is off by up to a wall's thickness on 176 of 462 walled elements of the office as built (6 doors, 170 windows), and writing x and z with y at the box's would still be a wrong origin. Nothing in the exporter changes, so no `location_feet` consumer (`scene_graph`, `sheet`, `gltf`, the MCP's views, the elements CSV, the viewer's Schedule, the saved-mesh frames, the opening cut's `centre_feet`, the type GlobalId's flip against the host wall) changes its output, and the IFC is `main`'s byte for byte.

What would change it: a way to tell, from the file, which instance creates each type and under which key, or the `StyleTransform` itself; a Revit export or file that holds both. Neither is in the open models.

## 7. Not claimed, not measured

- Measure on this head: the run is cited here once it has finished (figures above are local, on the kept IFCs of run 42066757105 at `eb6de281`).
- The office at permit (the same building as built), held-out models, Revit 2025 to 2027 beyond RE1 Architecture's 4 doors and the models of RE-347 section 6.
- Windows bound to no wall (none: every window is bound).
- Which instance creates a type, and whether it is the first by ElementId or by export order; whether the type key's level and wall side explain which members share a type entity.
- Snowdon Towers (local only), curtain walls, curved walls, turned instances.
- What `GetTransformForDoorOrWindow` computes (a RevitAPI call; its body is not read).

## 8. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=<sha> -f base=none -f scorers=none -f keep_ifc=true -f nordic=true \
  -f probe=probe_re343_door_flips -f probe_scorer=door_placements_vs_ifc -f models="core-interior re1-* nordic-office-*"
```

`<sha>` is a commit holding the scorer of this change. The scorer's `door flips: origin` lines are counts, and their per-axis lines are what section 4 reads. With a run's kept IFCs and probe output, and Revit's export:

```bash
python3 tools/re/door_placements_vs_ifc.py probe.txt revit.ifc --ours rvt-rs.ifc
```

The `row` lines carry, per element, `d` in both frames, the two openings' boxes in Revit's frame, the per-axis gaps, the type entity (`ftype`) and the symbol.
