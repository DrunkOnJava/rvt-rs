# RE-333: an Area's region from Revit's own plan topology

**Date:** 2026-11-04
**Issues:** #1261 (BIM Whale's Areas), #1194 (Areas beyond Revit 2024), #763 (Areas as spaces); follows RE-317 (an Area's region from its area boundary lines) and RE-324 (the Areas of Revit 2015 to 2023)
**Artefacts:** Autodesk's `rac_advanced` sample project, every save from 2015 to 2027 that `research/autodesk-sample-projects.tsv` pins and 2026 and 2027 (fetched, never stored); the BIM4LCA office, concrete structure, as built and at building permit (Revit 2024), with Revit's own IFC export of each (`research/bim4lca-measure.tsv`, CC BY-SA 4.0, never stored)
**Probe:** `probes/examples/probe_re333_area_plan_topology.rs`
**Status:** positive; section 8 has what the merged reader writes. Every Area of every file read here has a region from the plan topology: 25 of 25 on each `rac_advanced` save, 30 of 30 on each office file, each with the area Revit saves for its loop, and on the office the area of Revit's own export of the Area. The exporter writes them on every `rac_advanced` save but 2021's, where two copies of 16 lines make the group unreadable and the topology is not asked.

## 1. Before

RE-317 draws an Area's region from the boxes of its scheme's area boundary lines on its Level, where every line is straight along a plan axis and the lines meet. On `rac_advanced` that drew 3 of the 25 Areas. RE-317 §6 put the other 22 down to lines that are not along an axis (18) and to Areas "bounded partly by walls" (4), whose point lay in no loop of the lines. RE-324 drew the same 3 on the 2015 to 2023 saves and left the rest unwritten (`area_without_proven_body`). RE-330 (#1351) carries lines on to the lines that name them and joins parallel lines that stop at the same place, and draws **16 of the 25** on every save but 2021's: main writes 16, not 3, on the saves where it applies. The 9 it leaves out are the ones this report's topology covers.

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

The reader is bounded against a crafted file: a circuit over 10,000 sides, or that would make over 10,000 edges once its lines are cut where their offset changes, or whose lines have over 20,000 offset parts together, is not built (`area_circuit_side_not_read`), and so is a circuit one of whose lines has no `u32` ElementId (the Area's list of lines would not name a line its outline uses).

The checks: the outer loop's area, exact for arcs, equals the circuit's `m_area` to 1e-6 relative. Each hole's area equals its circuit's. The Area's point lies inside the outer loop and in no hole. An Area that fails a check, or whose plan, circuit, segments or lines are not read, keeps RE-317's refusal, as before (fail closed).

The exporter takes this region first, where it is read and passes the checks above, and otherwise the region of the lines' arrangement with RE-330's joins (RE-317, RE-330). Neither is asked of a group of lines that cannot be read as a whole: a line in a non-primary design option, at a stray height, with copies that disagree, or missing. Where both are found they must be the same region: the topology has no arc, the same net area to 1e-6 relative and the same corners within 1e-4 ft (a vertex where two straight edges continue on in a line is not a corner), and each hole of one is a hole of the other. An Area whose two regions differ has no body and says `area_regions_disagree`, so that a file nothing was measured on cannot disagree silently. Measured at `e9124b3a`, before the check existed: the two readers' regions were the same on 16 of 16 Areas of `rac_advanced` where both are found, and on 30 of 30 on the office; with the check, not yet measured, and an Area it refuses would be counted under `area_regions_disagree` in `exported.area_body_refusals`. An Area refused by both says the plan topology's reason (one of the six `area_*` reasons of the diagnostics: `area_not_in_one_plan_topology`, `area_circuit_not_found`, `area_circuit_side_not_read`, `area_circuit_area_differs`, `area_arc_not_finite`, `area_point_outside_circuit`) rather than the arrangement's.

Arcs are written as arcs, in the form Revit's export gives them: an `IfcIndexedPolyCurve` of lines and three-point arcs, a span over a half turn as two arcs (a circle as two half arcs), and an arc whose middle lies within 1.6e-5 ft of its chord, which the writer's micrometre rounding could put on it, as a line. The outline is checked as it is written: its area, arcs as arcs, must be the circuit's saved area to 1e-6 relative, as must the rebuilt edges'; an arc whose centre, radius or sweep is not finite, or that turns more than once, refuses the Area (`area_arc_not_finite`). Only the test of the Area's point uses chords (of at most 2°). Where two straight edges continue in a straight line (a line cut where another meets it), the point between them is left out unless another segment of the plan ends there: RE-317 §3 keeps that point where a third line meets, and so does this (a segment that no circuit runs except as a spur does not count, as RE-317 prunes a line with a free end). Revit's IFC4 export of the office keeps those points; the IFC2X3 export of `rac_advanced` keeps them at some Areas and not at others, so the rule is the office's (section 8).

## 4. Results

**`rac_advanced`, every save:** on 2015, 2016, 2017, 2019, 2021, 2023, 2024, 2026 and 2027, all 25 Areas have a region. The outer loop has the circuit's area to within 1e-15 relative where every side is straight. The one Area with an arc (`10`, 2024 numbering) agrees to 1e-9, its arc integrated exactly. The point is inside on 25 of 25. One plan on each save holds Areas `19` to `25`, and the four of them that RE-317 found in no loop are the ones whose lines are offset.

**The BIM4LCA office:** as built and at building permit, 30 of 30 Areas have a region, eleven of them with holes. The outer loop equals the circuit's area on all 30. The region less its holes equals the area of Revit's own export of the same Area, paired by GlobalId, to 2.1e-10 relative on 30 of 30 on each file. No line of the office has an offset, and every one of these regions has the area and perimeter of RE-317's. Its vertices are RE-317's: section 8 has what differs from main's file.

**The export:** measured on the merged reader in section 8: 17 exports move and 41 do not.

Reading the plans and lines takes 80 to 95 ms on `rac_advanced` and about 0.3 s on the office.

## 5. Not measured

- No Revit IFC4 export of a 2015 to 2023 or 2026 to 2027 file with Areas exists to compare with. The only Revit export of `rac_advanced`'s Areas is the IFC2X3 one of RE-330 (2015 to 2020 state), which section 8 scores the topology against. `rac_advanced` is otherwise checked against the areas Revit saves in the file, which are Revit's own figures for the same loops, not against Revit's IFC.
- Revit 2025 to 2027 Areas are written (section 6), measured against the same project's 2024 save only. Figures from a Snowdon Towers 2025 file await a recorded source (section 7).
- Revit 2008 to 2014.
- A line in a non-primary design option, and a plan topology per design option or per phase: none of the files has more than one plan for a scheme on a Level. An Area held by two plans keeps RE-317's region.
- An Area that Revit reports "not enclosed": none here. Its circuit would be missing or would fail a check, and it keeps RE-317's refusal.
- The step where two offset curves do not cross: none on these files.
- BIM Whale's 2021 project (#1261) is held out and was not opened.

## Reproduce

```bash
cargo run --profile ci -p rvt-probes --example probe_re333_area_plan_topology -- rac_advanced_sample_project.rvt
rvt-ifc rac_advanced_sample_project.rvt -o out.ifc --diagnostics out.json
python3 tools/re/space_profiles_vs_ifc.py office.ifc office_Revit.ifc --match globalid
```

## 6. Revit 2025 to 2027

The Area's partition chain record, which gives its number, name, Level and height, was read on 2015 to 2024 only. On 2025 and 2026 it is the 2024 record. Admitting both releases gives Autodesk's `rac_advanced` the 25 Areas of its 2024 save, each with the same GlobalId, number, name, storey, body area and height.

On 2027, and on some 2025 files, the same records are behind a block of entries. A 2024 and later chain record opens with its ElementId (`u64`), its size (`u32`) and `ElementHeader`'s tag (`u16`, `+0x0c`). The `u16` after the tag (`+0x0e`) is a count:

- On the 2024 files read (the BIM4LCA office, Core Interior) and Autodesk's 2025 and 2026 `rac_advanced` the count is 0 on every record. RE1 Architecture (2025) has records with up to 11 entries, and 2027 `rac_advanced` up to 15; the recorded Snowdon Towers 2024 samples carry them too (section 8).
- On 2027 the record has that many 22-byte entries from `+0x12`: an `i64` BuiltInParameter (negative), an `i64` -1, a `u16` and a `u32`.
- After the entries comes the 2026 body, opening with the `BuiltInCategory`.

On 2027 `rac_advanced` all 17,247 chain records have well-formed entries, from 0 to 15 of them (6,972 with none), and with the entries skipped their categories give the census of the 2026 copy: 169 categories, 25 Areas, 127 area boundary lines. The 2026 copy's 17,231 records all have a count of 0.

The Area reader reads a 2024 and later record after its entries. A 2027 `rac_advanced` export then gives the 25 Areas of the 2024 save, with the same GlobalIds, numbers, names, storeys, body areas and heights.

Not measured: no Revit 2025 to 2027 export with Areas is available, so these releases are checked only against the 2024 save of the same project. Readers other than the Area reader that read a chain record's category at `+0x12` are not changed here.

## 7. Two rules, and a 2025 source that is not recorded

The 2025 figures from a Snowdon Towers Architectural file and its Revit 2025 export await a recorded source (URL, sha256 and licence) and are not in this report. Two rules came from that file; neither changes anything on `rac_advanced` or the office:

- **A spur is left out.** A spur is a segment that the loop runs one way and, by the next side, straight back: a line that ends inside the Area. Once offset its two sides would leave a notch that Revit's area does not have. The recorded Snowdon Towers 2024 samples exercise it (section 8 has what is written there).
- **Where an Area's partition point is not read**, the plan topology's region stands, checked against the point the topology's own Area saves.

A line's offset parts with the same offset are one edge, which avoids an edge of no length where a part ends at a segment's end.

## 8. The merged reader

The merge with main (#1351, RE-330) puts the plan topology first where it is read and passes its checks, and the lines' arrangement with RE-330's joins second; a group of lines that cannot be read as a whole gives neither (section 3). Every figure below is from the head `e9124b3a` of #1371 unless it names another, and was measured before that head was rebuilt onto main (`feat/1261-area-plan-topology`: puzzbobb's two commits and the maintainer's five cherry-picked, signed, onto 182ed219, which adds the value sources of #1368, the decisions log of #1326 and the changes of #1405 and #1412). **It has not been measured on the rebuilt head**; the unfiltered Measure run on it replaces these figures. They come from the Measure runs of `e9124b3a` against main (the scorers come from a scratch branch at the same commit, because main's `space_profiles_vs_ifc` does not know the new marker), both green:

- run 41894715743: the samples 2015 to 2027 with the IFC kept and the probe;
- run 41894722135: the office with `space_profiles_vs_ifc` and the IFC kept.

The CI of the fork's pull request on `e9124b3a` passes but for two things: the five builds that run `tool_contract_is_committed`, which want `docs/mcp-tools.*` regenerated for the new `space_outline_source` value (they are synced by script, not by hand), and Deploy viewer, red on main since #1370 (#1374). Earlier heads: `2689cfe0` (runs 41882754673, 41882760570, 41886352595) and `a28a6ba5` (runs 41890721958, 41890725593); figures from them are labelled so.

**Which exports move** (run 41894715743, 58 pairs): **17 move and 41 do not.** The 17 are `rac_advanced` 2015 to 2020 and 2022 to 2027 (the 2021 save is not among them), `rst_basic` 2025 to 2027, and Snowdon Towers Architectural and Structural (the recorded Autodesk 2024 samples). The 2021 `rac_advanced`, `rst_basic` 2016 to 2024 and every other sample are byte for byte main's. The scorer diff is `schema.txt` of the 17 only: on a `rac_advanced` save of 2015 to 2020, 2022 or 2023, `sets` +9 and swept solids with a Position of their own +9 (the 9 Areas that gain a body); on 2024, swept solids +9; on 2025 to 2027, +25 of each; on `rst_basic` 2025 to 2027, `sets` +1 (its one Area, written without a body); on Snowdon Towers Architectural, `sets` +101 and swept solids +96, on Structural `sets` +7 and swept solids +2. The author's own run before the merge (run 41878649263, `a9d057d9`) moved 18: it also moved the 2021 save, which wrote 22 Areas there (3 refused) where main writes none. That is what the group-level refusal the merge adds prevents: the 2021 save holds two copies of 16 lines, so its group is not read, and neither is the plan topology.

**Per save** (Areas written, with a body, by `SpaceOutlineSource`):

| Save | Areas written | with a body | markers | not written / without a body |
|---|---:|---:|---|---|
| `rac_advanced` 2015 to 2020, 2022, 2023 | 25 | 25 | `area_plan_topology` 25 | none (main: 16 with a body, 6 `area_boundary` and 10 `area_boundary_joined`, and 9 not written, `point_in_no_single_region`) |
| `rac_advanced` 2024 | 25 | 25 | `area_plan_topology` 25 | none (main: 25 written, 16 with a body, 9 without) |
| `rac_advanced` 2025, 2026, 2027 | 25 | 25 | `area_plan_topology` 25 | none (main: none written) |
| `rac_advanced` 2021 | 0 | 0 | none | 25: `boundary_lines_not_readable` 22, `named_lines_not_in_scheme_and_level` 3 (as main) |
| `rst_basic` 2025, 2026, 2027 | 1 | 0 | `none` | `next_storey_not_proven` 1 (as the 2024 save) |
| Snowdon Towers Architectural (2024) | 101 | 96 | `area_plan_topology` 96, `none` 5 | `area_circuit_area_differs` 1, `point_not_read` 4 (main writes none) |
| Snowdon Towers Structural (2024) | 7 | 2 | `area_plan_topology` 2, `none` 5 | `next_storey_not_proven` 5 (main writes none) |

The two Snowdon Towers files are the recorded Autodesk 2024 samples (`research/autodesk-sample-projects.tsv`, sha256 `33271010...` and `6dc833f9...`): their records carry entry blocks, which main does not read, so main writes no Area there. These are counts of what rvt-rs writes; no Revit export of the two files is available, so nothing here compares them with one. The Snowdon Towers 2025 file of section 7 is not these.

**The 2020 save against the three.js IFC2X3 export of RE-330 section 1** (`tools/re/space_profiles_vs_ifc.py model.ifc rac_advanced_sample_project.ifc --match globalid`, the export fetched by hand from its pin and never stored): `IfcSpace` in rvt-rs 116, in Revit 116 (91 rooms and 25 Areas); in both 116, **only in rvt-rs 0, only in Revit 0**. For the Areas that is **written 25, Revit's 25, extra 0, missing 0**. All 25 are within 0.1% in area and within 0.01 ft in outline, boundary and in the lowest and highest vertex of the body; the largest outline and boundary distance of any of the 25 is below the 5 decimals the check prints (0.00001 ft), the arc's included. **This is out of sample for the topology:** nothing in the plan-topology reader was read off that export; its offsets and loops come from the file (`m_schemeSegments`, `PlanCircuit`), and its checks are against the area Revit saves in the file and the Area's point. (RE-330's joins were read off the export, so for them it was in-sample.) The export is not one of Revit's IFC4 exports and is counted apart, as RE-330 says, and it is never used to remove a value or to settle a disagreement between exporters.

The worst relative difference in area is 2.0e-7, on three four-sided Areas of under 83 ft2 (about 1.5e-5 ft2); it was not examined further. The 91 rooms are not part of this change and score as before. **The per-Area figures of this export are not kept in this report** (Griffin's policy for a reference with no licence stated, #1377: counts only, because the export's terms were never checked, as for BIMcollab's example): the 25-row table that an earlier version of this report held, and the per-Area vertex counts and areas below, are gone. The archived commits of #1371 carry that earlier version; it is already public in that history, and the policy is applied from here on.

**The arc** (`probe_re333_area_plan_topology`, the 2015 to 2020 state): the topology gives one Area with an arc edge, a loop of 33 vertices and an exact area that is the circuit's to 1e-6. rvt-rs writes the profile as an `IfcIndexedPolyCurve` of `IfcLineIndex` and `IfcArcIndex` segments; the export writes it as an `IfcCompositeCurve` of an `IfcPolyline` and an `IfcTrimmedCurve` over an `IfcCircle`. Both are true curves, so it is judged as #1365 has it, in IfcOpenShell's fine tessellation beside the default one (both files in Revit's internal feet, the top face of the body): at both the Area is within 0.1% in area and below 0.00001 ft in boundary distance of the export's. The symmetric difference of the two faces is the same at both tessellations, so it is not a chord's; what makes it is not examined. At `2689cfe0` (run 41882754673) the arc was cut into 2 degree chords.

**Vertices against the same export** (the scorer's vertex-to-vertex distance, 0.01 ft; the 2020 save): the merged head's Areas are within it on **17 of 25**; main's 16 on **9 of 16**. Of the 8 the head misses, 5 keep a vertex where a third line meets a straight outline that the export drops, 2 have more vertices in the export than in the head, and 1, the arc, is meshed in a different number of vertices in the two files. At `2689cfe0`, before the vertex rule, it was 21 of 25. So the export keeps a vertex at some points where a third line meets and not at others, and this file does not settle the rule. The rule keeps such a vertex because the office's Revit IFC4 export keeps it (below); the IFC2X3 export is counted apart and is never used to settle a disagreement between exporters.

**The 2022 and 2023 saves are of the later state of the model** (RE-324 section 4: 16 north-edge lines moved 0.0246 ft; measured on `e9124b3a`, and the same at `2689cfe0`). The topology tracks the save's own state: between the 2020 and the 2022 save, 7 Areas differ, by 0.23 to 2.77 ft2 (0.23 is 9.49 ft of outline times 0.0246 ft, so these outlines run along the moved lines), the 2022 save differs from the 2015 to 2020 export by the same amounts in those 7, and the other 18 Areas are equal (below 0.0001 ft2). The 2024 save differs from the 2020 save by the same amounts as the 2022 save. So the 2022 to 2024 saves are not scored against the export.

**RE-330's 16 joined Areas against their topology loops** (main's IFC against the head's, by GlobalId, the same save, the Areas' top surfaces; run 41894715743): on every save where main draws them, the 2015 to 2020, 2022, 2023 and 2024 saves (9 saves), **16 of 16 are equal**, with a symmetric difference and a boundary distance both below the 5 decimals the check prints (0.00001 ft2, 0.00001 ft). That includes the 10 that RE-330 draws by carrying a line on or by a connector, which the topology draws from the lines' offsets. It confirms RE-330's joins from the file's own data. The marker changes with it: the Areas main says `area_boundary` or `area_boundary_joined` now say `area_plan_topology`, because the topology comes first; the other two values are written where the topology is not read or fails a check.

**The office, exactly** (runs 41894722135, `e9124b3a`, and 41890725593, `a28a6ba5`, which gave the same): main's and the head's IFC of both files differ, and only here:

- the scorer diff is the label of the per-marker rows (`area boundary` to `area plan topology`, 30 Areas, their heights); the counts for area, outline, boundary, height and **vertices (115 as built, 113 at permit)** are main's;
- of the 308 spaces of each file (278 rooms and 30 Areas), none differs in region, vertex set or area; the 278 rooms are identical;
- the IFC text differs in the 30 Areas' `SpaceOutlineSource` (`area_boundary` to `area_plan_topology`), in the two timestamps (`FILE_NAME` and `IfcOwnerHistory`), and in the points of the profiles of 9 Areas as built (8 at permit): the outline starts at another vertex of the same ring in 3 (2 at permit), and the holes come in another order, the same rings, in 6 on each file. The 21 (22) other Areas' rings are identical.

At `2689cfe0`, before the vertex rule, the office's Area 1 had 4 vertices where main's has 6 and `vertices within 0.01 ft` went 115 to 114 as built and 113 to 112 at permit (runs 41882760570, 41886352595).

**The warning for dropped chain records** now appears only on a file that holds an Area or an area boundary line: Snowdon Towers Architectural (14,881) and Structural (2,932), 2 of the 58 samples and neither office file; Plumbing, Electrical and HVAC, which hold none, no longer have it (run 41894715743; at `2689cfe0` it was on five).

Not scored against the export: the 2015 to 2019 saves (the same 2015 to 2020 state as the 2020 save; the 2015 save scored the same at `2689cfe0`), the 2025 to 2027 saves, and everything in the tables but the 2020 save's Areas.

## 9. What is written without a Revit export, and the gate

**Decision (Griffin, 2026-11-05, "Ship, labelled, with a gate").** The Areas of Revit 2025 to 2027 and the 108 Area spaces of the recorded Autodesk 2024 Snowdon Towers samples (101 Architectural, 7 Structural, counted at `e9124b3a`, 98 of them with a body) are written by default, each saying `SpaceOutlineSource` `area_plan_topology`. They are **not measured against any Revit export**: none exists for these files. The evidence for them is:

- the loop drawn for each Area equals the area Revit saves in the file for that loop (`PlanCircuit m_area`, to 1e-6 relative, arcs integrated), and holds the Area's point;
- on Revit 2025 to 2027, the same project's 2024 save gives the same Areas (`rac_advanced`: the same 25 GlobalIds, numbers, names, storeys, body areas and heights on the 2025, 2026 and 2027 saves).

**The gate.** They stay written only if the held-out run shows no Area written that Revit's export lacks (`only in rvt-rs` 0) on every Revit 2025 to 2027 model with Areas. The final Measure of the head that lands is where that figure is read; it is not measured on `e9124b3a`. If it is not 0, the rule is not rewritten without Griffin's say.

The docs say the same: `docs/rooms.md`, `docs/status.md`, `docs/evidence.md`, `docs/supported-profile.md`, `docs/support-matrix.json` and the changelog fragments.
