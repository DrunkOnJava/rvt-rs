# RE-357 — Where Revit places a door's or window's origin

**Date:** 2026-11-05
**Issues:** #1465 (closed by this report's outcome); #1494 (the doors with no opening, section 5); follows RE-347 (the axes, #1447), RE-343 (the flips), RE-87 (instance transforms), RE-215 (symbol cut loops) and RE-167 (type GlobalIds)
**Probe:** `probes/examples/probe_re343_door_flips.rs` (unchanged)
**Scorer:** `tools/re/door_placements_vs_ifc.py` (the `origin` lines, new)
**Status:** negative for a rule; the origin stays at the record box's centre. Revit's placement origin is not the stored origin on any door or window set in a wall (0 of 1,061 within 0.001 ft, counted as run; 604 counted once, without the office at permit). Its offset from the stored origin is one value for every element of an `IfcDoorType` or `IfcWindowType` entity (160 of 160 type entities that hold a walled door or window as run, 92 of 92 counted once), which is the signature of revit-ifc's per-type `StyleTransform`, taken from the one instance that creates the type and not recorded in the file. Two of its three axes follow the element's own opening (x and z on 919 of 919 walled doors and windows of the two office files); the third, across the wall, does not (y on 375 of 385 doors and 198 of 534 windows there), so no rule with 0 wrong exists on all the walled elements. The one readable condition under which all three axes agree, both flips set (391 of 391 as run), is reported in section 4 and not written. Where Revit's export gives a door no opening the origin is the stored origin (153 of 153 doors as run). Measure run 42127619521, head `6b4058d17c07947543db4b6d2bc4c2e671fcedca`; every figure below is from it unless a line says otherwise.

## 0. The question

#1465: rvt-rs writes a door's or window's `IfcLocalPlacement` origin at its record box's centre (`location_feet`, z at the box's base), and Revit's is more than 0.5 ft from it on almost all (RE-347 section 4). Is the stored origin (`InstanceTransform.origin`, RE-87) Revit's origin outright? If not, is the offset constant per family or type, a family property, or derived from a stored value, and is there a rule with 0 wrong on the elements set in a plain wall?

## 1. What was measured

`tools/re/door_placements_vs_ifc.py` pairs, by Tag, each door and window of `probe_re343_door_flips` with the `IfcDoor` or `IfcWindow` of Revit's export and of rvt-rs's, and moves every origin and axis into Revit's internal frame through that file's own `IfcSite` placement (RE-347 section 1). For each element whose stored transform the probe read, `d` is Revit's placement origin less the stored origin, taken in the stored axes and in Revit's own placement axes. The scorer then prints, by kind and binding:

- how many elements have `d` of 0 (the stored origin is Revit's);
- how many groups hold one `d` (the type entity: the `IfcDoorType` or `IfcWindowType` entity the export gives the element, the symbol, the symbol and the flip state);
- the signs of `d` by flip state;
- per axis, how many `d` equal the corner of the opening each export gives the element. The opening is the `IfcOpeningElement` the element fills, meshed in world coordinates and expressed in Revit's placement axes about the stored origin; its corner is the lowest corner where the two flips are equal and the negated highest x and y with the lowest z where they differ.

The run: Measure [42127619521](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42127619521), head `6b4058d17c07947543db4b6d2bc4c2e671fcedca` (the scorer; `src/` is `main`'s at `df967677`), dispatched from `main`: `-f ref=6b4058d1 -f base=none -f scorers=none -f keep_ifc=true -f nordic=true`, the probe and scorer above, `-f models="core-interior re1-* nordic-office-*"`. Every job that ran succeeded, `same-project gates` included (no `src/` change). The models with a door or window in both exports are Core Interior (Revit 2024), RE1 Architecture (Revit 2025) and the BIM4LCA office as built and at permit (Revit 2024, one building; research/bim4lca-measure.tsv); the other five (the two terrain files and RE1's three services models) hold none. The figures are the `door flips: origin` lines of each model's `probe_score.txt`, and the per-state and per-element ones are counted from its `row` lines. Held-out models are not in this change's run. A local scoring of the same scorer on the kept IFCs of run 42066757105 (head `eb6de281`, before RE-347's rule, which moves axes and not origins) gave the same counts for Core Interior, RE1 and the office as built.

## 2. The stored origin is not Revit's

| model | doors bound to a plain wall | windows bound to a plain wall | stored origin within 0.001 ft of Revit's |
|---|---:|---:|---:|
| Core Interior | 132 | 6 | 0 |
| RE1 Architecture | 4 | 0 | 0 |
| office as built | 193 | 269 | 0 |
| office at permit | 192 | 265 | 0 |

1,061 as run, 604 counted once (without the permit file). `d` is more than 0.5 ft on every one of them. The doors rvt-rs binds to no plain wall are different (section 5).

## 3. The offset belongs to the type entity

In every model, `d` is one value for all the elements of an `IfcDoorType` or `IfcWindowType` entity, in the stored axes and in Revit's, and it is not one value per symbol:

| model | type entities, doors | with one `d` | symbols, doors | with one `d` (stored axes) | type entities, windows | with one `d` | symbols, windows | with one `d` (stored axes) |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| Core Interior | 10 | 10 | 3 | 0 | 1 | 1 | 1 | 1 |
| RE1 Architecture | 4 | 4 | 3 | 2 | 0 | 0 | 0 | 0 |
| office as built | 67 | 67 | 33 | 18 | 10 | 10 | 6 | 4 |
| office at permit | 62 | 62 | 32 | 18 | 6 | 6 | 4 | 3 |

That is 160 type entities of 160 as run, and 46 of 82 symbols (92 of 92 and 25 of 46 counted once). Of the type entities that hold more than one element (Core Interior 9 doors and 1 window, the office as built 33 and 9, at permit 35 and 5), every one has one `d`. Per symbol and flip state the doors are 47 of 60 in the office as built and 45 of 58 at permit, and the windows 11 of 11 and 6 of 6.

That is what revit-ifc does (CLEANROOM.md source 5; read for behaviour, not copied). In `FamilyInstanceExporter` (`Source/Revit.IFC.Export/Exporter/FamilyInstanceExporter.cs`), where a door or window's type is created (the type key is not in the cache, or the instance uses its own geometry), the exporter asks the Revit API for the door or window transform of that instance, its symbol and its two flips (`ExporterIFCUtils.GetTransformForDoorOrWindow`, a RevitAPI name whose body is not read) and keeps its inverse as the type's `StyleTransform` (`FamilyTypeInfo.StyleTransform`, `Source/Revit.IFC.Export/Utility/FamilyTypeInfo.cs`). Every instance placed with that type then has its own transform multiplied by that one `StyleTransform`, and the product's origin is the `IfcLocalPlacement` origin. So Revit's origin is the stored origin plus a type-level offset taken from the instance that created the type. The file records neither which instance that was nor Revit's type key (RE-343 section 3 reached the same limit for the 49 inverted doors that rvt-rs binds to no wall).

## 4. Which part of the offset a stored value gives

Against the opening Revit's own export cuts for the element, `d` equals the opening's corner as defined in section 1 on all three axes in Core Interior (132 of 132 doors, 6 of 6 windows) and RE1 Architecture (4 of 4 doors), and per axis in the office, within 0.001 ft:

| | x | y | z | all three |
|---|---:|---:|---:|---:|
| office as built, doors, 193 | 193 | 187 | 193 | 187 |
| office as built, windows, 269 | 269 | 99 | 269 | 99 |
| office at permit, doors, 192 | 192 | 188 | 192 | 188 |
| office at permit, windows, 265 | 265 | 99 | 265 | 99 |

- **x and z** are the element's own opening: x is the stored frame's lowest x (the Revit-frame lowest x where the flips are equal and the negated highest x where they differ), z is the opening's bottom, which is a door's base and a window's sill. Core Interior and RE1 cannot tell the two x conventions apart (their openings are symmetric about the stored origin); the office can, since its openings are asymmetric about it by 0.3 to 0.7 ft in x.
- **y**, across the wall, is not. On the doors it is the opening's corner on 375 of 385; the other 10 (6 and 4) are each the only element of their type entity, in openings 1.6 to 3.0 ft deep (1.558 to 2.971 ft), and `d` is off by up to 2.585 ft. On the windows it is the corner on the 99 of each file whose two flips are both set; on the 26 of each file with neither set `d` is the opening's upper-y face (26 of 26 in both), and on the unequal-flip windows (144 as built, 140 at permit) `d` is not a face of the opening at all: 1.552 ft beyond the lower-y face on 140 windows in each file (four type entities as built, three at permit), and 0.781 ft on the 4 remaining ones as built (one type entity). The windows' openings are 1.3 to 2.1 ft deep.
- **Both flips set: seen, not written.** The elements with both `m_flippedX` and `m_flippedY` set agree with the opening's corner on every axis wherever they occur: 391 of 391 walled elements as run (Core Interior 66 doors and 6 windows, RE1 Architecture 1 door, the office as built 60 and 99, at permit 60 and 99; 232 of 232 of the 604 counted once), by the run's `row` lines. The other states do not: doors with neither set are right on 90 of 91 and with unequal flips on 234 of 243 (the 1 and the 9 of the 10 doors above whose y is not the corner), and windows with neither set or unequal flips on none of 336. It is not written as a rule, for three reasons. The flip state has no mechanism behind it: Revit's offset is the creating instance's `StyleTransform` shared by the whole type entity (section 3), so the agreement says that on these models the elements of a both-flips-set type entity stand in walls like the one its creator stood in, not that the flip state fixes the origin. The case that would break it is a later instance of a type entity standing in a different wall from the instance that created the type: Revit gives it the creator's offset, not the one its own opening gives, and the corner rule would be wrong on it. The doors here cannot test that: every door type entity with more than one element has every member's own corner equal to `d` (the 10 doors that disagree are each alone in their type entity), so no door type entity here holds two members whose openings differ (not measured on the windows, whose y is not the corner in most states). And the other 372 of the 604 elements would keep the box's centre, so one type would carry two origin conventions. The held-out models were not run for it.
- **rvt-rs's own opening** is not Revit's on every door either. Used in place of Revit's, it gives x on 190 of 193 doors as built and 189 of 192 at permit (up to 4.199 ft off), y on 187 and 188, z on 192 and 191 (all three on 186 and 187), and the same figures as Revit's on the windows. So even the two axes that follow the opening would not be written with 0 wrong from rvt-rs's reading of it.

## 5. The doors rvt-rs binds to no plain wall

Of the 154 doors that rvt-rs binds to no plain wall (the office as built 76, at permit 77, RE1 Architecture 1), the stored origin is Revit's origin (within 0.001 ft) on 153. Those 153 are the doors Revit's export gives no opening (75, 77 and 1); the one other, in the office as built, has an opening in both exports, and `d` is its corner on all three axes. So where neither export has an opening for a door, the origin is the stored origin (153 of 153 as run, 76 of 76 counted once), and where one exists the offset is as in section 4. This is a measurement for #1494, the issue about those doors (the 160 of RE-343, 49 of them inverted), where it may become a rule with 0 wrong of its own; nothing is written for them here. `d` is also one value per type entity for them (29 of 29 type entities as built, 31 of 31 at permit), and 0 for all but the one.

## 6. Decision and what it leaves

The origin stays at the record box's centre (`location_feet`), which is the measured-reason outcome #1465's done-when allows, and the issue closes with it. A rule needs 0 wrong on the walled elements, written only where it reads; x and z are readable, y is a type-level constant from an unrecorded instance and is off by up to a wall's thickness on 346 of the 919 walled elements of the two office files (10 doors, 336 windows), and writing x and z with y at the box's would still be a wrong origin. Nothing in the exporter changes, so no `location_feet` consumer (`scene_graph`, `sheet`, `gltf`, the MCP's views, the elements CSV, the viewer's Schedule, the saved-mesh frames, the opening cut's `centre_feet`, the type GlobalId's flip against the host wall) changes its output, and the IFC is unchanged (no base-against-head run was made; the change touches no `src/` file).

What would change it: a way to tell, from the file, which instance creates each type and under which key, or the `StyleTransform` itself; a Revit export or file that holds both. Neither is in the open models.

## 7. Not claimed, not measured

- Held-out models, Revit 2025 to 2027 beyond RE1 Architecture's 4 walled doors, and the models of RE-347 section 6.
- Windows bound to no wall (none: every window is bound).
- Which instance creates a type, and whether it is the first by ElementId or by export order; whether the type key's level and wall side explain which members share a type entity.
- Snowdon Towers (local only), curtain walls, curved walls, turned instances.
- What `GetTransformForDoorOrWindow` computes (a RevitAPI call; its body is not read).
- The time and memory of the scorer's added meshing: it runs on the probe's scorer step only, and was not timed.

## 8. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=6b4058d17c07947543db4b6d2bc4c2e671fcedca -f base=none -f scorers=none -f keep_ifc=true -f nordic=true \
  -f probe=probe_re343_door_flips -f probe_scorer=door_placements_vs_ifc -f models="core-interior re1-* nordic-office-*"
```

The scorer's `door flips: origin` lines are counts, and their per-axis lines are what section 4 reads. With a run's kept IFCs and probe output, and Revit's export:

```bash
python3 tools/re/door_placements_vs_ifc.py probe.txt revit.ifc --ours rvt-rs.ifc
```

The `row` lines carry, per element, `d` in both frames, the two openings' boxes in Revit's frame, the per-axis gaps, the type entity (`ftype`) and the symbol.
