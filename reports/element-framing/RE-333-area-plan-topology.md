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

The checks: the outer loop's area, exact for arcs, equals the circuit's `m_area` to 1e-6 relative. Each hole's area equals its circuit's. The Area's point lies inside the outer loop and in no hole. An Area that fails a check, or whose plan, circuit, segments or lines are not read, keeps RE-317's refusal, as before (fail closed).

The exporter takes this region first, where it is read and passes the checks above, and otherwise the region of the lines' arrangement with RE-330's joins (RE-317, RE-330). Neither is asked of a group of lines that cannot be read as a whole: a line in a non-primary design option, at a stray height, with copies that disagree, or missing. Where both are found they have the same region (16 of 16 on `rac_advanced`, 30 of 30 on the office); section 8 has what was measured, including the office Area whose outline has two fewer vertices. An Area refused by both says the plan topology's reason (one of the five `area_*` reasons above) rather than the arrangement's.

Arcs are written in chords of at most 2° (`ARC_STEP_RADIANS`). Where two straight edges continue in a straight line (a line cut where another meets it), the point between them is left out. RE-317 §3 keeps that point where a third line meets, which this does not: one office Area has two fewer vertices than Revit's export (section 8).

## 4. Results

**`rac_advanced`, every save:** on 2015, 2016, 2017, 2019, 2021, 2023, 2024, 2026 and 2027, all 25 Areas have a region. The outer loop has the circuit's area to within 1e-15 relative where every side is straight. The one Area with an arc (`10`, 2024 numbering) agrees to 1e-9, its arc integrated exactly. The point is inside on 25 of 25. One plan on each save holds Areas `19` to `25`, and the four of them that RE-317 found in no loop are the ones whose lines are offset.

**The BIM4LCA office:** as built and at building permit, 30 of 30 Areas have a region, eleven of them with holes. The outer loop equals the circuit's area on all 30. The region less its holes equals the area of Revit's own export of the same Area, paired by GlobalId, to 2.1e-10 relative on 30 of 30 on each file. No line of the office has an offset, and every one of these regions has the area and perimeter of RE-317's. Its vertices are not all RE-317's: one Area has two fewer, section 8. Its vertices are not all RE-317's: one Area has two fewer, section 8.

**The export:** measured on the merged reader in section 8: 17 exports move and 41 do not.

Reading the plans and lines takes 80 to 95 ms on `rac_advanced` and about 0.3 s on the office.

## 5. Not measured

- No Revit IFC4 export of a 2015 to 2023 or 2026 to 2027 file with Areas exists to compare with. The only Revit export of `rac_advanced`'s Areas is the IFC2X3 one of RE-330 (2015 to 2020 state), which section 8 scores the topology against. `rac_advanced` is otherwise checked against the areas Revit saves in the file, which are Revit's own figures for the same loops, not against Revit's IFC.
- Revit 2025 to 2027 Areas are written (section 6), measured against the same project's 2024 save and, for Snowdon Towers Architectural, section 7's export, whose provenance is not recorded.
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

- On the 2024 files read (the BIM4LCA office, Core Interior) and Autodesk's 2025 and 2026 `rac_advanced` the count is 0 on every record. RE1 Architecture (2025) has records with up to 11 entries, Snowdon Towers Architectural (2025) up to 1,815, and 2027 `rac_advanced` up to 15.
- On 2027 the record has that many 22-byte entries from `+0x12`: an `i64` BuiltInParameter (negative), an `i64` -1, a `u16` and a `u32`.
- After the entries comes the 2026 body, opening with the `BuiltInCategory`.

On 2027 `rac_advanced` all 17,247 chain records have well-formed entries, from 0 to 15 of them (6,972 with none), and with the entries skipped their categories give the census of the 2026 copy: 169 categories, 25 Areas, 127 area boundary lines. The 2026 copy's 17,231 records all have a count of 0.

The Area reader reads a 2024 and later record after its entries. A 2027 `rac_advanced` export then gives the 25 Areas of the 2024 save, with the same GlobalIds, numbers, names, storeys, body areas and heights.

Not measured: no Revit 2025 to 2027 export with Areas is available, so these releases are checked only against the 2024 save of the same project. Readers other than the Area reader that read a chain record's category at `+0x12` are not changed here.

## 7. Snowdon Towers Architectural, against Revit's own 2025 export

**Provenance not recorded.** Neither file's source, sha256 or licence is recorded (`research/autodesk-sample-projects.tsv` holds only the 2024 set), so under `CLEANROOM.md` these figures are not admissible until it is; the review asked for it (item 4) and the author has not answered. They stand here unchanged, and come out if the origin is never recorded.

Snowdon Towers Architectural, Revit 2025, with a Revit 2025 IFC export of it (exporter 25.4.50.6, local only: neither file is redistributable here, as for RE-101). The export has 101 `IfcSpace` whose `Category` is `Areas`.

- **Spaces:** rvt-rs writes 101 Area spaces, all of them Revit's by GlobalId, with the same number and name, and none that Revit's export lacks. `main` writes none: 2025 Areas were not read.
- **Bodies:** rvt-rs writes 96 bodies. 95 have the area of Revit's within 0.1% (worst 2.1e-4, Revit's arcs traced in chords to compare). The other is `P4` (Common), which Revit's export writes with no `Representation`, though the other Areas of its Level have one. Why is not found.
- **No body:** 5 Areas have no body in rvt-rs where Revit's export draws one. On each, the loop rebuilt here differs from the area Revit saves for it, by 4e-6 to 1.6e-3 relative, and Revit's export has the saved area.
  - Each has a sub-millimetre piece of a line with an offset of its own, and how Revit treats that is not found.
  - The area check refuses them, as it is meant to, and they keep their partition refusals: `point_not_read` for 4 and `boundary_lines_not_readable` for 1.

Two rules came from this file and change nothing on `rac_advanced` or the office:

- **A spur is left out.** A spur is a segment that the loop runs one way and, by the next side, straight back: a line that ends inside the Area. Once offset its two sides would leave a notch that Revit's area does not have.
- **Where an Area's partition point is not read**, the plan topology's region stands, checked against the point the topology's own Area saves. 48 of Snowdon's Areas were refused only for `point_not_read`.

A line's offset parts with the same offset are one edge, which avoids an edge of no length where a part ends at a segment's end.

## 8. The merged reader

The merge with main (#1351, RE-330) puts the plan topology first where it is read and passes its checks, and the lines' arrangement with RE-330's joins second; a group of lines that cannot be read as a whole gives neither (section 3). Measure, `--ref main`, all green, on `2689cfe0`; the CI of the fork's pull request passed on that head (28 checks, `full`).

**Runs.** The samples 2015 to 2027 against main with the IFC kept, run 41882754673; the office against main with `space_profiles_vs_ifc`, run 41882760570, and with the IFC kept, run 41886352595; the author's own samples run on their head before the merge (`a9d057d9`) against main, run 41878649263.

**Which exports move** (run 41882754673, 58 pairs): **17 move and 41 do not.** The 17 are `rac_advanced` 2015 to 2020 and 2022 to 2027 (the 2021 save is not among them), `rst_basic` 2025 to 2027, and Snowdon Towers Architectural and Structural (the recorded Autodesk 2024 samples). The 2021 `rac_advanced`, `rst_basic` 2016 to 2024 and every other sample are byte for byte main's. The scorer diff is `schema.txt` of the 17 only. On a `rac_advanced` save of 2015 to 2020, 2022 or 2023 it is `sets` +9 and swept solids with a Position of their own +9, one each for the 9 Areas that gain a body; on the 2024 save, which wrote the 25 Areas already, swept solids +9 and no `sets`; on 2025 to 2027, `sets` +25 and swept solids +25. The author's own run, before the merge, moved 18: it also moved the 2021 save, which wrote 22 Areas there (3 refused) where main writes none. That is the group-level refusal the merge adds: the 2021 save holds two copies of 16 lines, so its group is not read, and neither is the plan topology.

**Per save** (Areas written, with a body, `SpaceOutlineSource`):

| Save | Areas written | with a body | markers | not written / without a body |
|---|---:|---:|---|---|
| `rac_advanced` 2015 to 2020, 2022, 2023 | 25 | 25 | `area_boundary` 25 | none (main: 16 with a body, 6 `area_boundary` and 10 `area_boundary_joined`, and 9 not written, `point_in_no_single_region`) |
| `rac_advanced` 2024 | 25 | 25 | `area_boundary` 25 | none (main: 25 written, 16 with a body, 9 without) |
| `rac_advanced` 2025, 2026, 2027 | 25 | 25 | `area_boundary` 25 | none (main: none written) |
| `rac_advanced` 2021 | 0 | 0 | none | 25: `boundary_lines_not_readable` 22, `named_lines_not_in_scheme_and_level` 3 (as main) |
| `rst_basic` 2025, 2026, 2027 | 1 | 0 | `none` | `next_storey_not_proven` 1 (as the 2024 save) |
| Snowdon Towers Architectural (2024) | 101 | 96 | `area_boundary` 96, `none` 5 | `area_circuit_area_differs` 1, `point_not_read` 4 (main writes none) |
| Snowdon Towers Structural (2024) | 7 | 2 | `area_boundary` 2, `none` 5 | `next_storey_not_proven` 5 (main writes none) |

The two Snowdon Towers files are the recorded Autodesk 2024 samples (`research/autodesk-sample-projects.tsv`, sha256 `33271010...` and `6dc833f9...`): their records carry entry blocks, which main does not read, so main writes no Area there. The counts are what rvt-rs writes; no Revit export of these two files is available, so nothing here compares them with one. The Revit 2025 file and export of section 7 are not these.

**The 2020 save against the three.js IFC2X3 export of RE-330 section 1** (`tools/re/space_profiles_vs_ifc.py model.ifc rac_advanced_sample_project.ifc --match globalid`, the export fetched by hand from its pin and never stored): `IfcSpace` in rvt-rs 116, in Revit 116 (91 rooms and 25 Areas); in both 116, **only in rvt-rs 0, only in Revit 0**. For the Areas that is **written 25, Revit's 25, extra 0, missing 0**. All 25 are within 0.1% in area and within 0.01 ft in outline, boundary and in the lowest and highest vertex of the body. **This is out of sample for the topology:** nothing in the plan-topology reader was read off that export; its offsets and loops come from the file (`m_schemeSegments`, `PlanCircuit`), and its only checks are against the area Revit saves in the file and the Area's point. (RE-330's joins were read off the export, so for them it was in-sample.) The export is not one of Revit's IFC4 exports and is counted apart, as RE-330 says, and it is never used to remove a value or to settle a disagreement between exporters.

| Area | rvt-rs ft2 | Revit's ft2 | relative difference | outline ft | height ft |
|---:|---:|---:|---:|---:|---:|
| 1 | 1978.334 | 1978.334 | 1.0e-10 | 0.0000 | 0.0000 |
| 2 | 1040.444 | 1040.444 | 5.7e-08 | 0.0000 | 0.0000 |
| 3 | 6912.840 | 6912.840 | 2.2e-08 | 0.0000 | 0.0000 |
| 4 | 1203.391 | 1203.391 | 3.8e-15 | 0.0000 | 0.0000 |
| 5 | 78.413 | 78.413 | 1.8e-15 | 0.0000 | 0.0000 |
| 6 | 1444.637 | 1444.637 | 9.0e-11 | 0.0000 | 0.0000 |
| 7 | 897.423 | 897.423 | 1.1e-08 | 0.0000 | 0.0000 |
| 8 | 3658.544 | 3658.544 | 1.5e-08 | 0.0000 | 0.0000 |
| 9 | 446.881 | 446.881 | 6.4e-08 | 0.0000 | 0.0000 |
| 10 | 6309.709 | 6309.710 | 1.2e-07 | 0.0017 | 0.0000 |
| 11 | 77.111 | 77.111 | 2.0e-07 | 0.0000 | 0.0000 |
| 12 | 3709.916 | 3709.916 | 1.5e-08 | 0.0000 | 0.0000 |
| 13 | 348.138 | 348.138 | 7.1e-08 | 0.0000 | 0.0000 |
| 14 | 474.855 | 474.855 | 1.9e-15 | 0.0000 | 0.0000 |
| 15 | 183.525 | 183.525 | 6.2e-16 | 0.0000 | 0.0000 |
| 16 | 2386.907 | 2386.907 | 5.4e-11 | 0.0000 | 0.0000 |
| 17 | 3658.442 | 3658.442 | 1.1e-08 | 0.0000 | 0.0000 |
| 18 | 896.604 | 896.604 | 1.4e-08 | 0.0000 | 0.0000 |
| 19 | 6450.970 | 6450.970 | 2.4e-08 | 0.0000 | 0.0000 |
| 20 | 3658.442 | 3658.442 | 1.1e-08 | 0.0000 | 0.0000 |
| 21 | 896.604 | 896.604 | 1.4e-08 | 0.0000 | 0.0000 |
| 22 | 6889.133 | 6889.133 | 6.3e-09 | 0.0000 | 0.0000 |
| 23 | 184.063 | 184.063 | 9.3e-16 | 0.0000 | 0.0000 |
| 24 | 67.576 | 67.576 | 1.9e-07 | 0.0000 | 0.0000 |
| 25 | 82.474 | 82.474 | 2.0e-07 | 0.0000 | 0.0000 |

The 2015 save, which is of the same 2015 to 2020 state, scores the same: 25 Areas, worst relative difference 2.0e-7, worst outline 0.0017 ft.

The worst relative difference in area is 2.0e-7. The worst outline and boundary distance, 0.0017 ft, is Area 10, whose arc is traced in 2 degree chords (the chord's sag at that radius is 0.0018 ft); every other Area is within 0.0001 ft. The 91 rooms are not part of this change and score as before.

**Vertices against the same export** (the scorer's vertex-to-vertex distance, 0.01 ft; the 2020 save): the merged head's Areas are within it on **21 of 25**; main's 16 on **9 of 16**. The head misses Areas 7, 8 and 12, where the export has one or two vertices more than the topology's outline (8 against 6, 7 against 6, and 17 against 16), and Area 10, the arc (65 chords against 64). Main's misses are the other way: Areas 14, 17, 18, 19, 20 and 21 carry vertices that the carried-on lines add and the export lacks (5 against 4, 6 against 4, 7 against 6, 28 against 20, 5 against 4, 7 against 6), and Area 12 has 16 against the export's 17, as the head does. So the export keeps a vertex at some points where a third line meets and not at others, and this file does not settle the rule; the office's Revit IFC4 export keeps it (below).

**The 2022 and 2023 saves are of the later state of the model** (RE-324 section 4: 16 north-edge lines moved 0.0246 ft). The topology tracks the save's own state: between the 2020 and the 2022 save, Areas 1, 2, 3, 10, 12, 19 and 22 differ by 1.78, 0.99, 0.23, 0.23, 2.77, 0.23 and 2.77 ft2 (0.23 is 9.49 ft of outline times 0.0246 ft, so these outlines run along the moved lines), the 2022 save differs from the 2015 to 2020 export by the same amounts, and the other 18 Areas are equal (below 0.0001 ft2). The 2024 save differs from the 2020 save by the same amounts as the 2022 save. So the 2022 to 2024 saves are not scored against the export.

**RE-330's 16 joined Areas against their topology loops** (main's IFC against the merged head's, by GlobalId, the same save, the Areas' top surfaces): on every save where main draws them, the 2015 to 2020, 2022, 2023 and 2024 saves, **16 of 16 are equal**, with a symmetric difference and a boundary distance both below the 5 decimals the check prints (0.00001 ft2, 0.00001 ft). That includes the 10 that RE-330 draws by carrying a line on or by a connector, which the topology draws from the lines' offsets. It confirms RE-330's joins from the file's own data. The marker changes with it: the Areas main says `area_boundary_joined` now say `area_boundary`, because the topology comes first and the exporter has no marker that says an Area came from it; `area_boundary_joined` is written where the topology is not read and the arrangement with its joins draws the Area.

**The office** (runs 41882760570 and 41886352595): **the IFC of both files is not byte for byte main's, which the fork's report said it was.** Run 41882760570 (`space_profiles_vs_ifc`): `ifc.sha256` differs on both files, and the scorer diff is one line on each, `vertices within 0.01 ft` 115 to 114 as built and 113 to 112 at permit; the counts for area, outline, boundary and height are unchanged. With the IFC kept (run 41886352595), exactly one space differs on each file, and it is the same one: Area 1, whose region is the same (symmetric difference 0.000000 ft2, 13,953.181 ft2 both ways) but which main draws with 6 vertices and the topology with 4. The topology leaves out a point where two straight edges continue in a straight line, "as Revit's profile leaves it out (RE-317 section 3)"; RE-317 section 3 says the opposite for a point where a third line meets, which is a vertex of the outline, and main's 6 vertices are the ones that matched Revit's export. So on one Area of 30 on each file the topology's body has the right region and two fewer vertices than Revit's export has. The other 29 Areas and every room are the same as main's. This is a code finding, not a text one, and it is not fixed here: the rule would keep a collinear point where another segment of the plan meets it.

**A new warning.** The count of declared chain records whose entry block does not read is a warning in the diagnostics. It appears on 5 of the 58 samples, all Snowdon Towers 2024 files: Architectural 14,881, Plumbing 13,335, Electrical 6,979, HVAC 5,767 and Structural 2,932. It counts records the Area reader cannot classify, and does not say that any is an Area: 101 and 7 Areas are found on Architectural and Structural all the same, and no Area is written on Plumbing, Electrical or HVAC. It does not appear on `rac_advanced`, `rst_basic`, or the office. Whether to keep it when the file holds no Area record is open (the review of the fork's pull request, item 6).

Not scored against the export: the 2016 to 2019 saves (the same 2015 to 2020 state as the 2020 save), the 2025 to 2027 saves, and everything in the table but the 2020 save's Areas.
