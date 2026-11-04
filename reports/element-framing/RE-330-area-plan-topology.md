# RE-330: an Area's region from Revit's own plan topology

**Date:** 2026-11-04
**Issues:** #1261 (BIM Whale's Areas), #1194 (Areas beyond Revit 2024), #763 (Areas as spaces); follows RE-317 (an Area's region from its area boundary lines) and RE-324 (the Areas of Revit 2015 to 2023)
**Artefacts:** Autodesk's `rac_advanced` sample project, every save from 2015 to 2027 that `research/autodesk-sample-projects.tsv` pins and 2026 and 2027 (fetched, never stored); the BIM4LCA office, concrete structure, as built and at building permit (Revit 2024), with Revit's own IFC export of each (`research/bim4lca-measure.tsv`, CC BY-SA 4.0, never stored)
**Probe:** `probes/examples/probe_re330_area_plan_topology.rs`
**Status:** positive. Every Area of every file read here has its region: 25 of 25 on each `rac_advanced` save, 30 of 30 on each office file. Each region has the area Revit saves for its loop, and on the office the area of Revit's own export of the Area.

## 1. Before

RE-317 draws an Area's region from the boxes of its scheme's area boundary lines on its Level, where every line is straight along a plan axis and the lines meet. On `rac_advanced` that drew 3 of the 25 Areas. RE-317 §6 put the other 22 down to lines that are not along an axis (18) and to Areas "bounded partly by walls" (4), whose point lay in no loop of the lines. RE-324 drew the same 3 on the 2015 to 2023 saves and left the rest unwritten (`area_without_proven_body`).

Neither cause is the one. On that sample 71 of the 127 area boundary lines are drawn a distance from where the area is measured: the area scheme measures to a wall's face, and the line's box is the measured line, not the drawn one. Lines that meet where they are drawn do not meet where they are measured. Revit closes the loop by meeting the measured lines where they cross, which the boxes cannot show.

## 2. Where the region is

For each area scheme on each Level Revit keeps a `LevelRoomPlan` element. Its `m_planTopology` is a `PlanTopology`, which the schema describes in full and `native_document` decodes:

- `m_levelId`, and `m_rooms`: the Areas of that scheme on that Level.
- `m_segments`: every area boundary line cut where another meets it. Each is keyed by `{m_item: the line's ElementId, m_majorIndex: 0, m_subIndex: k}`. Its `m_info` gives `m_param0` and `m_param1`, the piece's range along the line from 0 to 1, and `m_points`, the piece's points where the line is drawn (an arc in twelve or more points).
- `m_circuits`: every closed loop (`PlanCircuit`). Each has `m_id`, `m_area` (square feet, the loop alone, its holes not taken out), `m_envelope`, and `m_sides`: segment keys in order around the loop, `m_subIndex` `-k - 1` where the loop runs segment `k` backwards. The loop that bounds a connected set of lines from outside has a negative area.
- `m_components`: each connected set of lines, with `m_outerCircuit` (its outside loop) and `m_enclosingCircuit` (the loop it sits in, `-1` for none). A set whose enclosing loop is an Area's is a hole of that Area.

An Area is a `RoomElem` with `m_areaSchemeId` set. Its `m_cachedCircuitId` is the `m_id` of its loop, and `m_point` is the point RE-317 reads from its data object.

A line is a `CurveElem` with `m_isAreaMeasurementScheme` true. Its curve is a `GLine` (`m_origin`, `m_dirVec`, `m_endParams`) or a `GArc` (`m_center`, `m_radius`, `m_xVec`, `m_yVec`, `m_endParams` in radians; the arc's record also holds a `GLine` of no length at its centre). A line that follows a wall carries `AreaMeasureCurveData`. Its `m_wallElemId` is the wall, and `m_schemeSegments` lists `{m_startParam, m_endParam, m_offset, m_instId}`: over each part of the line, from 0 to 1, how far its area is measured from it, in feet. On `rac_advanced` the offsets are 0.497 ft and 0.041 ft. On the office no line has one.

## 3. The rule

The region of an Area is its circuit rebuilt from its lines:

1. For each side, take the line's curve from `m_param0` to `m_param1`. Reverse it where the side's sub index is negative. Check that the curve passes through the segment's saved ends, to 1e-6 ft.
2. Move each part of it to the curve's right (direction turned clockwise) by its `m_offset`, cutting it where the offset changes. A line without `AreaMeasureCurveData` keeps offset 0. An arc's radius changes by the offset.
3. Where one side ends and the next begins, meet their curves where they cross, taking the crossing nearest the lines' shared end. Where the curves do not cross near it, join the two ends with a straight step.
4. The outer loop is the circuit's. The holes are the outer loops of the components that the circuit encloses.

The checks: the outer loop's area, exact for arcs, equals the circuit's `m_area` to 1e-6 relative. Each hole's area equals its circuit's. The Area's point lies inside the outer loop and in no hole. An Area that fails a check, or whose plan, circuit, segments or lines are not read, keeps RE-317's refusal, as before (fail closed).

The exporter takes RE-317's region where the lines' arrangement finds one, and this region where it finds none. Where both are found they agree: on the office every region has the same area and perimeter both ways, and on `rac_advanced` the 3 Areas RE-317 draws have their circuits' areas. So every body written before is written as before, byte for byte.

Arcs are written in chords of at most 2° (`ARC_STEP_RADIANS`). Where two straight edges continue in a straight line (a line cut where another meets it), the point between them is left out, as Revit's profile leaves it out (RE-317 §3).

## 4. Results

**`rac_advanced`, every save:** on 2015, 2016, 2017, 2019, 2021, 2023, 2024, 2026 and 2027, all 25 Areas have a region. The outer loop has the circuit's area to within 1e-15 relative where every side is straight. The one Area with an arc (`10`, 2024 numbering) agrees to 1e-9, its arc integrated exactly. The point is inside on 25 of 25. One plan on each save holds Areas `19` to `25`, and the four of them that RE-317 found in no loop are the ones whose lines are offset.

**The BIM4LCA office:** as built and at building permit, 30 of 30 Areas have a region, eleven of them with holes. The outer loop equals the circuit's area on all 30. The region less its holes equals the area of Revit's own export of the same Area, paired by GlobalId, to 2.1e-10 relative on 30 of 30 on each file. No line of the office has an offset, and every one of these regions has the area and perimeter of RE-317's.

**The export:** `rvt-ifc` on `main` (fadd80b5) and on this change, with `SOURCE_DATE_EPOCH` fixed:

- each `rac_advanced` save from 2015 to 2024 but 2021 writes 25 Area spaces with a body, where `main` writes 3. `skipped` no longer has `area_without_proven_body`, and `exported.area_body_refusals` is empty;
- the 2021 save writes 22, where `main` writes none. Its other 3 Areas have a region here, but they name lines whose two copies in the file disagree (the save holds two states of the model, RE-324), so RE-317's check still refuses them (`named_lines_not_in_scheme_and_level`);
- every other export compared is byte-identical, the office's included: 70 local files of every release from 2015 to 2027, among them Autodesk's samples, the magnetar corpus and the BIM4LCA models.

Reading the plans and lines takes 80 to 95 ms on `rac_advanced` and about 0.3 s on the office.

## 5. Not measured

- No Revit 2015 to 2023 or 2026 to 2027 export with Areas exists to compare with. `rac_advanced` is checked against the areas Revit saves in the file, which are Revit's own figures for the same loops, not against Revit's IFC.
- Areas of Revit 2025 to 2027 are still not written: the partition chain record that gives an Area its number, name, Level and height is read on 2015 to 2024 only. Their plans and regions read as on 2024.
- Revit 2008 to 2014.
- A line in a non-primary design option, and a plan topology per design option or per phase: none of the files has more than one plan for a scheme on a Level. An Area held by two plans keeps RE-317's region.
- An Area that Revit reports "not enclosed": none here. Its circuit would be missing or would fail a check, and it keeps RE-317's refusal.
- The step where two offset curves do not cross: none on these files.
- BIM Whale's 2021 project (#1261) is held out and was not opened.

## Reproduce

```bash
cargo run --profile ci -p rvt-probes --example probe_re330_area_plan_topology -- rac_advanced_sample_project.rvt
rvt-ifc rac_advanced_sample_project.rvt -o out.ifc --diagnostics out.json
python3 tools/re/space_profiles_vs_ifc.py office.ifc office_Revit.ifc --match globalid
```
