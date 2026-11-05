# RE-332: where an area boundary line lies against the wall it is picked from

**Date:** 2026-11-04, cut to research notes on 2026-11-05
**Issues:** #1261 (the Areas of 2015 to 2023 files that Revit bounds partly by walls); follows RE-317, RE-324 and RE-330; overtaken by RE-333 (#1428)
**Artefacts:** Autodesk's `rac_advanced_sample_project`, saves of 2015 to 2024 (`research/autodesk-sample-projects.tsv`, fetched by Measure's `-f samples=true`); the IFC2X3 export of the same project from the three.js repository (RE-330 section 1: `examples/models/ifc/rac_advanced_sample_project.ifc` at commit `7c1f47731608de51a2240ce31655634665cb51ce`, sha256 `9440a72eb2d27e3ba9edeafdcdfb71945e6baec56d5b09211bcde743de7fe0a5`, fetched by hand, never stored); the BIM4LCA office (Revit 2024); the held-out BIM Whale project, by counts only
**Probe, tool:** `probes/examples/probe_re332_wall_hosted_lines.rs`; `tools/re/wall_lines_vs_offsets.py`
**Status:** research notes, **no export change**. Measured and kept: how the 127 area boundary lines of `rac_advanced` lie against the walls their reference lists name (section 2); that each position is the offset the file stores for the line, on all 127 lines (section 3); and the two lines Revit draws on a wall face for one stretch and on its centre line for another (section 4). **Not landed:** the reader this report first drew two Areas with (a rule read off the three.js IFC2X3 export, `area_boundary_wall_line`), kept at `refs/archive/2026-11-05/feat/1261-areas-bounded-by-walls` (98ba33b3a4e0b6d9723ee8f5b1f821aaaf9ddb17); section 7 reports what it measured. RE-333 (#1428) draws all 25 Areas of every save but 2021's from the offsets the file stores, with no rule read off an export. **One claim of the first version was wrong:** that nothing in the file says where a line steps from face to centre (section 5).

## 0. What the check rests on

- The only external check is the IFC2X3 export of RE-330 section 0: a build of Revit 2021's exporter whose string ends "IU alternativa 21.0.0.383" (publisher not established), the project's 2015 to 2020 state. It is not one of Revit's IFC4 exports, it is counted apart, and it is **never used to remove a value or to settle a disagreement between exporters**. No workflow fetches it (its terms are not known, RE-330 section 1): the scoring here was run by hand from the pin above. **Only counts of it are kept here** (Griffin's policy for a reference with no licence stated, #1377): the first version also held the export's positions along two lines and the overlaps between its Areas, and those are gone.
- The walls in this report are read from the bytes: the probe prints each host wall's record box (section 2), and the body rvt-rs draws for each host wall has the footprint of the export's wall to 0.001 ft.
- The reader of section 7 was not landed. Its agreement with the export was in-sample: its rule was chosen so that two Areas would equal the export's outlines.
- The held-out BIM Whale project is a regression check, by counts. Nothing here was developed on it.

## 1. The nine Areas that RE-330 left out, and what bounded each in RE-330's reader

Every edge of every Area's outline in the export lies on an area boundary line or is a join of RE-330 (a carry or a connector). There is no edge that a wall bounds and no line bounds. The walls enter through the lines: a line picked from a wall names that wall in its reference list (section 2).

| Area | Level | Lines on its outline (ElementIds) | What stopped RE-330's reader | Walls the lines name |
|---|---|---|---|---|
| 1 | 1 | 177354, 177355, 177365, 177377, 177425 | its south edge runs along line 177377, drawn on a wall face for one stretch and on the wall's centre line for another (section 4) | 139856, 148422 (curtain walls), 139857, 169770 |
| 2 | 1 | 177355, 177377, 177425, 177513, 177586, 177634 | along 177377, on the stretch 44 to 74 ft beyond the host wall's east end | 139857, 169770 |
| 3 | 1 | 26 lines, among them 177363, 177377, 178310 | 177363 and 177377 as for Areas 1 and 4, and the slanted line 178310 (RE-330 section 5) | many |
| 4 | 1 | 177363, 177366, 177367, 177772, 177963 | its north edge runs along line 177363, as Area 1's south edge along 177377; and 177772 stops 0.101 ft short of 177963 (as Area 6) | 144837, 148769, 149877 |
| 5 | 1 | 177377, 177513, 177586, 177634 | along 177377, on the stretch 74 to 85 ft beyond the host wall's east end | 169770 |
| 6 | 1 | 177353, 177367, 177772, 177963 | 177772 stops 0.101 ft short of 177963 and names nothing there (RE-330 section 3), so its region is Area 4's | 139855, 149877 |
| 7, 8 | 1 | 177368, 177369, 178053, 178096, 178178, 178258, 178310, 178440, 178525 | the slanted line 178310 (RE-330 section 5) | 178310 names none; the others name 150980, 153616 and more |
| 10 | 2 | 178788 | an arc (RE-330 section 5) | 178788 names none |

So the walls decide five Areas, through two lines, 177363 and 177377. Areas 6, 7, 8 and 10 have no compound line on their outline. Some of their lines are picked from walls (the table), and lie on a centre line or a face as every ordinary line does; what stopped RE-330's reader there was a join, a slanted line or an arc. RE-333 draws all nine (section 8).

## 2. Where Revit puts a line picked from a wall

Of the 127 area boundary lines of the 2020 save (`probe_re330_area_line_curves`, run 41832621319): **61 name a basic wall in their reference list, 50 a curtain wall, 16 none, and none names two**. The other ids in the lists are the Level, the area scheme (3325 and 18459), 9490 (every line) and 136350 (the 38 lines of the third Level). Every wall id the lists name is a wall of the export (`IfcWall` or `IfcCurtainWall`, by `Tag`), the basic walls with a matching footprint.

`probe_re332_wall_hosted_lines` compares, for each such line, its record box with its host wall's record box (the walls' record boxes are read in the same pass), and prints the line's two edges across the wall as offsets from the wall's centre line. On the 2020 save (run 41862228192; the counts are the same on the 2015 to 2019, 2022 to 2024 saves, and on 2021's, whose walls have 44 copies that disagree):

| Host wall type (thickness) | Lines | Where the line lies |
|---|---:|---|
| Exterior - Insulation on Masonry (0.9941 ft) | 18 | on a face, 0.497 ft off the centre line, **the outer face** (the side away from the building) |
| Exterior - Insulation on Masonry (0.9941 ft) | 2 | **compound**: the box spans the centre line and one face (section 4) |
| Generic - 225mm Concrete (0.7382 ft) | 4 | on a face, 0.369 ft off, the outer face |
| Interior - 138mm Partition (0.4544 ft) | 31 | on the centre line |
| Generic - 200mm (0.6562 ft) | 6 | on the centre line |
| curtain walls | 50 | 44 on a line 0.041 ft off the wall's axis (`other` in the probe, 3.2808 ft thick in the record box), 6 on its axis |

So the export draws an Area **to the outer face of an exterior wall and to the centre line of an interior wall**. The probe's counts on the 2020 save: 43 `centre`, 22 `face`, 2 `compound`, 44 `other` (curtain walls), 16 `nohost`. The wall's **record box** is wider than its body along the wall, where it runs to the centre line of the wall it joins: for wall 169770 the box starts 0.497 ft (half the thickness) before the body does; the thickness across (0.994094) and the centre line (73.267030) are the same in both. The box of the wall is (-32.312334, 72.769983) to (-4.521160, 73.764077).

## 3. The position is the offset the file stores

A line picked from a wall carries `AreaMeasureCurveData`, whose `m_schemeSegments` lists, over each part of the line, the distance its area is measured from it (RE-333 section 2; `probe_re333_area_plan_topology` prints it as `line` rows). This report's first version did not look there. Joining the two probes' output line by line (`tools/re/wall_lines_vs_offsets.py`, section 10) gives, on the 2022 save (probe runs 41862228192 and 42008543772, and run 42027944261 of this report's head, both probes on the 2020, 2021 and 2022 saves; the 2020, 2021 and 2023 saves give the same table):

| Position by record box | Host wall | Lines | Stored offsets, feet |
|---|---|---:|---|
| `centre` | Interior - 138mm Partition, Generic - 200mm | 37 | 0 |
| `centre` | curtain walls | 6 | 0 on 3, 0.041010 on 3 |
| `face` | Exterior - Insulation on Masonry (0.9941) | 18 | 0.497047, half the thickness, on all |
| `face` | Generic - 225mm Concrete (0.7382) | 4 | 0.369, half the thickness: -0.369 on 2, +0.369 on 2 |
| `compound` | Exterior - Insulation on Masonry | 2 | 0 over part of the line and 0.497047 (one sign on each line) over the rest |
| `other` | curtain walls | 44 | 0.041010: +0.041010 on 43, -0.041010 on 1 |
| `nohost` | none | 16 | none |

So the record box and the stored offset say the same thing for **all 127 lines**: a line on a face has an offset of half its wall's thickness, a line on a centre line has none, a curtain wall's line is 0.041010 ft off, a line that names no wall has none, and the two compound lines have both. 71 lines have an offset, the 71 of RE-333. The record box held the line's position by its extent, and for a compound line it is the union of the offset parts; the offsets are the stored form. A positive offset moves the edge to the line's right (RE-333 section 2).

**The joins of RE-330 are wall dimensions.** The edges of the 16 Areas that RE-330 draws, laid over the lines' record boxes, leave 51 uncovered pieces (counted per Area, so a shared edge counts twice). Their lengths are the dimensions of the walls the lines name:

| Length (ft) | Pieces | Equals |
|---:|---:|---|
| 0.041 | 23 | a curtain wall's offset from its axis (section 2) |
| 0.328 | 2 | half of Generic - 200mm (0.6562) |
| 0.369 | 2 | half of Generic - 225mm Concrete (0.7382) |
| 0.497 | 13 | half of Exterior - Insulation on Masonry (0.9941) |
| 0.625 | 7 | 0.9941 less 0.3691, the exterior wall and half the concrete one |
| 0.666 | 4 | 0.9941 less 0.3281, the exterior wall and half the 200 mm one |

The 0.041, 0.369 and 0.497 pieces are stored offsets. RE-330's carry and connector caps (at most 1 ft) bound distances that are wall dimensions; RE-333 meets the offset curves where they cross instead, and its loops for the 10 Areas RE-330 draws by a carry or a connector are the same region (16 of 16 equal, RE-333 section 8). That is an identification of values, not a proof of RE-330's mechanism.

## 4. The two lines drawn on a face and on a centre line

Lines 177377 and 177363 are picked from the exterior walls 169770 and 144837. Their record boxes are 0.497 ft across, half of the wall's thickness, from the wall's centre line to one face (`compound` in the probe). Their stored curves, read by `probe_re330_area_line_curves`, are the centre lines, and their stored offsets (the 2022 save, `probe_re333_area_plan_topology`) are the step:

| Line | Host wall (record box) | Stored curve | Record box | Stored offsets (part of the line, from its start) |
|---|---|---|---|---|
| 177377 | 169770 (x -32.3123 to -4.5212, y 72.7700 to 73.7641) | (80.2785, 73.2670) to (-32.1434, 73.2670) | x -32.1434 to 80.2785, y 72.7700 to 73.2670 | 0 from 0 to 0.833320, then -0.497047 to 1 |
| 177363 | 144837 (x -32.3123 to -4.7484, y 19.4092 to 20.4033) | (-4.8747, 19.9063) to (-32.3123, 19.9063) | x -32.3123 to -4.8747, y 19.9063 to 20.4033 | 0 from 0 to 0.310893, then 0.497047 to 1 |

The step is at x -13.405 on both lines (0.833320 of 177377's length, 0.310893 of 177363's), the stored x of line 177364, picked from the curtain wall 145547 (whose box is 0.041 ft west of it). West of the step the offset moves the edge half a thickness to the wall's face, on the courtyard side, outside the building; east of it, where each side of the wall has an Area, the offset is 0 and the edge is the centre line. In the export the Areas that border these lines do the same (counts only here: three pairs of its Areas overlap by strips under 0.05 ft wide, and no other pair by more than 0.0001 ft2).

The first version of this report read the step from the export and said the file holds only the centre curve and the union box ("a line's element data is a header, the reference lists and one curve record, and its size is that of any other line with the same number of references"). That was the partition record, which does not hold `AreaMeasureCurveData`; the native record read by member name (RE-333) does. The step is stored; nothing here says whether Revit also works it out again when it regenerates the line.

## 5. What the first version said could not be proven, and what the file says

- **The stretch of a compound line within its host wall's span, and within one thickness of its ends.** The first version took this to be unreadable: both positions occur there, and the step is at a place only the export told. The file stores the step (section 4). Areas 1, 3 and 4 need it, and RE-333 draws them.
- **That the enclosure explains the position.** The first version did not use the argument that a line is on the centre line wherever an Area lies on both sides, because it could not find Area 3's region among the lines the reader read. The offsets make it unnecessary.
- **Whether any other model has such a line.** The BIM4LCA office has no line with an offset at all (RE-333 section 2), so it holds none. BIM Whale's project was not opened. A line with an offset that changes along it is what to look for.

## 6. The rule that was tried

The first version read a line as picked from a wall when all of these held: its reference list names exactly one wall whose record box was read, and none whose box was not; its record box is 0.05 to 1 ft across; the wall's record box runs the same way; the box's width equals half the wall's thickness to 0.001 ft; and one long edge of the box lies on the wall's centre line and the other on a side of the wall's box. It then took the line to be the wall's centre line and drew an Area along it only on the stretches more than one wall thickness beyond the wall's record box, where no face can move the line. Every Area along it anywhere else was refused (`wall_line_position_not_proven`). It is not on main, and RE-333 needs none of it.

What the probe measured for its numbers, which stay true of the file:

- **Tolerance 0.001 ft.** Over the 10 `rac_advanced` saves the largest residual of any line against its wall is 5e-7 ft (a face, at the probe's six printed decimals), 0 for the centre lines and the two compound lines, and the nearest line that is neither on a centre line nor on a face is 0.041 ft off (the 440 `other` rows). Any tolerance from 1e-6 to 0.04 ft classifies every line alike here.
- **The box of a compound line.** Its width is half the wall's thickness, 0.497 ft on both lines, so any bound up to 0.497 and any from it gives the same lines; 0.041 ft, a curtain wall's offset, is the nearest other distance from a wall feature.
- **The stored curve is the centre line.** Checked on the two lines only (`probe_re330_area_line_curves`, run 41832621319: 177377 at y 73.267030 from x 80.278543 to -32.143371, 177363 at y 19.906295 from x -4.874670 to -32.312334).

## 7. What the reader that was not landed measured

Measure, `--ref main`, all green, of the archived branch. The runs of `8ed50748` (the probe alone of `3a3ceeeb`) came first; the reader was then changed (a stub carried on is checked, the refusal order, the hosts and the walls' scan), and the runs of `6ec876f2` re-checked the counts and moved the refusal reasons as predicted.

**The probe** (run 41862228192, the 10 `rac_advanced` saves): on every save but 2021's it reads 127 lines and 180 walls, with the counts of section 2 and the two compound lines with their stretches beyond one wall thickness (`177377`: line x -32.143371 to 80.278543, wall x -32.312334 to -4.521160, **-3.527065 to 80.278543**; `177363`: line x -32.312334 to -4.874670, wall x -32.312334 to -4.748358, none). The 2021 save has the same counts and 44 walls whose copies disagree.

**The reader against main on the samples 2015 to 2024** (run 41864933862, `-f base=main`, 43 pairs): the IFC of **9 saves moved and 34 did not**: the `rac_advanced` saves of 2015 to 2020, 2022, 2023 and 2024. The scorer diff was `schema.txt` of those 9: `sets` +2 and swept solids with a Position of their own +2 (one per Area drawn).

**Per save, Areas** (run 41864937912, IFC kept; run 41871958093 on `6ec876f2`):

| Save | Areas written | with `area_boundary` / `_joined` / `_wall_line` | Areas not written, by reason |
|---|---:|---|---|
| 2015 to 2020, 2022, 2023 | 18 | 6 / 10 / 2 | 7: `point_in_no_single_region` 4 (Areas 3, 7, 8, 10), `two_areas_one_region` 2 (Areas 4 and 6), `wall_line_position_not_proven` 1 (Area 1) |
| 2021 | 0 | none | 25: `boundary_lines_not_readable` 22, `named_lines_not_in_scheme_and_level` 3 (as RE-330) |
| 2024 | 18 with a body, of 25 written | 6 / 10 / 2 | 7 written without a body, the same reasons |

**The 2020 save against the export** (`tools/re/space_profiles_vs_ifc.py model.ifc rac_advanced_sample_project.ifc --match globalid`): `IfcSpace` in rvt-rs 109 (91 rooms and 18 Areas), in Revit 116 (91 and 25); in both 109, **only in rvt-rs 0, only in Revit 7**. For the Areas that is written 18, Revit's 25, extra 0, missing 7. By marker, each Area within 0.1% in area and 0.01 ft in outline, boundary and in the lowest and highest vertex: `area_boundary` 6 of 6, `area_boundary_joined` 10 of 10, `area_boundary_wall_line` 2 of 2. In-sample for the 2 wall-line Areas and the 10 joined: the rule was read off the same export.

**Held out and the office** (counts only): BIM Whale's IFC byte for byte main's (run 41864945459; `IfcSpace` written 0, Revit's 8, extra 0, matched 0 of 0), which is vacuous: the project draws no Area body. The office's IFC of both files byte for byte main's, an empty scorer diff (run 41868440397, `-f nordic=true`; its 689 lines are all axis lines, so no wall is read).

**The unfiltered run** (run 41869173374, `-f base=main -f nordic=true -f r2025=true -f samples=true -f holdout=true`, every scorer, on `77c0716a`): 9 exports moved, exactly the nine `rac_advanced` saves above; 102 were byte for byte main's, all 19 held-out models among them; the scorer diff touched only those 9. The final unfiltered run, on `6ec876f2` (run 41875835920), was green; its figures are not recorded here.

## 8. Where each Area stands now

RE-333 (#1428) draws **all 25 Areas** on every `rac_advanced` save but 2021's, from the plan topology and the stored offsets: Areas 2 and 5 (the two this report drew), Areas 1, 3, 4 and 6 (the compound lines and the 0.101 ft join), and Areas 7, 8 and 10 (the slanted line and the arc). On the 2020 save, 25 of 25 are within 0.1% in area and 0.01 ft in outline of the export, and none is written that it lacks (RE-333 section 8), out of sample for the topology. The 2021 save is #1434.

## 9. Not measured

- Any Revit IFC4 export of these Areas. None exists.
- Whether any model other than `rac_advanced` holds a line with an offset that changes along it (section 5).
- The 2022 to 2024 state of the 16 edited lines against any export (RE-324 section 4, RE-330 section 4).
- Which face or centre an exterior wall of a type the file does not hold would get; the stored offsets answer it for the file that has them, and the area scheme's own rules are not read.

## 10. Reproduce

```bash
# the export, by pin; never stored
curl -fsSL -o rac_advanced_sample_project.ifc \
  https://raw.githubusercontent.com/mrdoob/three.js/7c1f47731608de51a2240ce31655634665cb51ce/examples/models/ifc/rac_advanced_sample_project.ifc
sha256sum rac_advanced_sample_project.ifc   # 9440a72eb2d27e3ba9edeafdcdfb71945e6baec56d5b09211bcde743de7fe0a5
# both probes on the samples, then the join of section 3
gh workflow run measure.yml --ref main -f ref=<sha> -f base=none -f samples=true -f sample_years="2015 2024" \
  -f models="zzz-no-model" -f scorers=none -f probe="probe_re332_wall_hosted_lines probe_re333_area_plan_topology" \
  -f probe_scorer="- -"
python3 tools/re/wall_lines_vs_offsets.py probe_re332.txt probe_re333.txt
# the reader that was not landed: the archived branch, 98ba33b3a4e0b6d9723ee8f5b1f821aaaf9ddb17
git fetch origin refs/archive/2026-11-05/feat/1261-areas-bounded-by-walls
```
