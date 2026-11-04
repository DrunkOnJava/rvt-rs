# RE-332: Areas whose boundary lines are picked from walls

**Date:** 2026-11-04
**Issues:** #1261 (the Areas of 2015 to 2023 files that Revit bounds partly by walls); follows RE-317, RE-324 and RE-330
**Artefacts:** Autodesk's `rac_advanced_sample_project`, saves of 2015 to 2024 (`research/autodesk-sample-projects.tsv`, fetched by Measure's `-f samples=true`); the IFC2X3 export of the same project from the three.js repository (RE-330 section 1: `examples/models/ifc/rac_advanced_sample_project.ifc` at commit `7c1f47731608de51a2240ce31655634665cb51ce`, sha256 `9440a72eb2d27e3ba9edeafdcdfb71945e6baec56d5b09211bcde743de7fe0a5`, fetched by hand, never stored); the BIM4LCA office (Revit 2024); the held-out BIM Whale project, by counts only
**Probe:** `probes/examples/probe_re332_wall_hosted_lines.rs`
**Status:** positive and narrow, on one project, **in-sample**: the rule was read off the IFC2X3 export that scores it, and the Areas it draws are labelled under Griffin's decision of 2026-11-04 on IFC2X3 oracles. Of the nine Areas of `rac_advanced` that RE-330 left out, five have an edge on a compound line, a line picked from a wall (sections 2 and 4), and **two (Areas 2 and 5) are now drawn** on every save but 2021's, saying `SpaceOutlineSource` `area_boundary_wall_line`: 18 of the 25 Areas are drawn (16 before). On the 2020 save all 18 are within 0.1% in area and 0.01 ft in outline of the export, none is written that it lacks, and 7 of its 25 are not written (section 7). Areas 1, 3, 4 and 6 stay out; Areas 7, 8 and 10 have no compound line, and are stopped by a slanted line and an arc. BIM Whale's IFC is byte for byte main's (counts only, 0 extra), which is vacuous: it draws no Area body. **No Area is bounded by a wall where no boundary line is.**

## 0. What the check rests on

- The only external check is the IFC2X3 export of RE-330 section 0: a build of Revit 2021's exporter whose string ends "IU alternativa 21.0.0.383" (publisher not established), the project's 2015 to 2020 state. It is not one of Revit's IFC4 exports, it is counted apart, and it is **never used to remove a value or to settle a disagreement between exporters**. No workflow fetches it (its terms are not known, RE-330 section 1): the scoring here was run by hand from the pin above.
- **The rule of section 6 was read off that export.** Areas 2 and 5 equal its outlines because the rule was chosen so that they would: the agreement is in-sample, and says nothing of a model it was not read off. Griffin decided on 2026-11-04 that Areas checked only against an IFC2X3 export of this kind ship labelled, with a marker of their own (`area_boundary_wall_line`); the docs, the changelog and the PR say the same. Re-check them against the first Revit IFC4 export of these Areas.
- The walls in this report are read from the bytes: the probe prints each host wall's record box (section 2), and rvt-rs's own export of the 2020 save draws all 11 host walls compared to the same bounds as the export's footprints, to 0.001 ft.
- The held-out BIM Whale project is a regression check, by counts. Nothing here was developed on it.

## 1. The nine Areas that RE-330 left out, and what bounds each

Every edge of every Area's outline in the export lies on an area boundary line or is a join of RE-330 (a carry or a connector). There is no edge that a wall bounds and no line bounds. The walls enter through the lines: a line picked from a wall names that wall in its reference list (section 2).

| Area | Level | Lines on its outline (ElementIds) | What stops it | Walls the lines name |
|---|---|---|---|---|
| 1 | 1 | 177354, 177355, 177365, 177377, 177425 | its south edge runs along line 177377, drawn on a wall face for one stretch and on the wall's centre line for another (section 4) | 139856, 148422 (curtain walls), 139857, 169770 |
| 2 | 1 | 177355, 177377, 177425, 177513, 177586, 177634 | along 177377, on the stretch 44 to 74 ft beyond the host wall's east end: **drawn** | 139857, 169770 |
| 3 | 1 | 26 lines, among them 177363, 177377, 178310 | 177363 and 177377 as for Areas 1 and 4, and the slanted line 178310 (RE-330 section 5) | many |
| 4 | 1 | 177363, 177366, 177367, 177772, 177963 | its north edge runs along line 177363, as Area 1's south edge along 177377; and 177772 stops 0.101 ft short of 177963 (as Area 6) | 144837, 148769, 149877 |
| 5 | 1 | 177377, 177513, 177586, 177634 | along 177377, on the stretch 74 to 85 ft beyond the host wall's east end: **drawn** | 169770 |
| 6 | 1 | 177353, 177367, 177772, 177963 | 177772 stops 0.101 ft short of 177963 and names nothing there (RE-330 section 3), so its region is Area 4's, which runs along 177363: expected `two_areas_one_region` (section 7) | 139855, 149877 |
| 7, 8 | 1 | 177368, 177369, 178053, 178096, 178178, 178258, 178310, 178440, 178525 | the slanted line 178310 (RE-330 section 5) | 178310 names none; the others name 150980, 153616 and more |
| 10 | 2 | 178788 | an arc (RE-330 section 5) | 178788 names none |

So the walls decide five Areas, through two lines, 177363 and 177377. Areas 6, 7, 8 and 10 have no compound line on their outline. Some of their lines are picked from walls (the table), and lie on a centre line or a face as every ordinary line does; what stops them is a join, a slanted line or an arc.

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

So the export draws an Area **to the outer face of an exterior wall and to the centre line of an interior wall**. The record box already holds that position, for every line but two, which is why RE-317 and RE-330 draw 16 Areas from the boxes alone. The area scheme's own name and rules are not read. Which of face and centre a wall gets is not separable in this file from its type: the exterior types are the ones on the outside of the building, and the file has no exterior wall in the middle of the plan except the two of section 4.

The probe's counts on the 2020 save: 43 `centre`, 22 `face`, 2 `compound`, 44 `other` (curtain walls), 16 `nohost`. The body rvt-rs draws for each host wall equals the export's footprint (to 0.001 ft): 169770, 144837, 148769, 139854, 139857, 150980, 154427, 155851, 155993, 157706, 161062. The wall's **record box** is wider than its body along the wall, where it runs to the centre line of the wall it joins: 169770's is (-32.312334, 72.769983) to (-4.521160, 73.764077), its body (-31.815, 72.770) to (-4.521, 73.764). Its thickness across (0.994094) and its centre line (73.267030) are the same.

## 3. The joins of RE-330 are wall dimensions

The edges of the 16 Areas that RE-330 draws, laid over the lines' record boxes, leave 51 uncovered pieces (counted per Area, so a shared edge counts twice). Their lengths are the dimensions of the walls the lines name:

| Length (ft) | Pieces | Equals |
|---:|---:|---|
| 0.041 | 23 | a curtain wall's offset from its axis (section 2) |
| 0.328 | 2 | half of Generic - 200mm (0.6562) |
| 0.369 | 2 | half of Generic - 225mm Concrete (0.7382) |
| 0.497 | 13 | half of Exterior - Insulation on Masonry (0.9941) |
| 0.625 | 7 | 0.9941 less 0.3691, the exterior wall and half the concrete one |
| 0.666 | 4 | 0.9941 less 0.3281, the exterior wall and half the 200 mm one |

The caps of RE-330 (a carry or a connector of at most 1 ft) therefore bound distances that are wall dimensions. That is an identification of values, not a proof of the mechanism, and the rule of RE-330 is unchanged.

## 4. The two lines drawn on a face and on a centre line

Lines 177377 and 177363 are picked from the exterior walls 169770 and 144837. Their record boxes are 0.497 ft across, half of the wall's thickness, from the wall's centre line to one face (`compound` in the probe). Their stored curves, read by `probe_re330_area_line_curves`, are the centre lines:

| Line | Host wall (record box) | Stored curve | Record box |
|---|---|---|---|
| 177377 | 169770 (record box x -32.3123 to -4.5212, y 72.7700 to 73.7641) | (80.2785, 73.2670) to (-32.1434, 73.2670) | x -32.1434 to 80.2785, y 72.7700 to 73.2670 |
| 177363 | 144837 (record box x -32.3123 to -4.7484, y 19.4092 to 20.4033) | (-4.8747, 19.9063) to (-32.3123, 19.9063) | x -32.3123 to -4.8747, y 19.9063 to 20.4033 |

In the export, **the Areas that border them follow the face for one stretch and the centre line for another**:

- Area 1 (north of 177377): its south edge runs at y 72.770, the wall's south face, from x -32.184 to -13.405; steps up 0.497 ft at x -13.405; and runs at y 73.267, the centre line, from x -13.405 to 39.969. Area 4 (south of 177363) does the same at y 20.403 and y 19.906, with the step at the same x.
- The step is at x -13.405, the stored x of line 177364 (picked from the curtain wall 145547, whose box is 0.041 ft west of it, at -13.446). Area 3, on the other side of both lines, turns its corner at x -13.446, the box.
- So the export's Areas are not a partition: Areas 1 and 3 overlap by 0.0204 ft2, Areas 3 and 4 by 0.0204 ft2 and Areas 3 and 9 by 0.0303 ft2, strips 0.041 ft wide. No other two Areas of the export overlap by more than 0.0001 ft2.
- West of the step, the south side of 177377 and the north side of 177363 is the courtyard, outside the building; east of it, each wall has an Area on both sides. That is what the export's positions fit (face where the wall's outer side is outside, centre where it is between two Areas) and it is Revit's area rule for exterior walls, but **nothing in the file says it**.

What the bytes hold of the two lines is the centre curve and the union box, and nothing else. A line's element data is a header, the reference lists and one curve record, and its size is that of any other line with the same number of references: the data of 177363 has the shape (8 references, curve at offset 467, 148 bytes after it) of lines 178906 and 178913, and that of 177377 (10, 535, 208) of lines 178912 and 179119, all ordinary lines on a centre line (run 41832621319). There is no second curve, no face curve and no list of stretches. The step is not stored: Revit works it out when it regenerates the line, from the wall's faces and what is on its two sides.

## 5. What cannot be proven

- **The stretch of a compound line within its host wall's span, and within one thickness of its ends.** Both positions occur there in the export (face west of the step, centre east of it), and the step is at a place only the export tells (the stored x of one curtain-wall line). Areas 1, 3 and 4 need it. Reproducing Area 1 would also need the step at the line's stored x while Area 3 needs the box: the export's own Areas disagree by 0.041 ft there, so no single arrangement of the lines gives both exactly.
- **That the enclosure explains the position.** Taking the line to be on the centre line wherever an Area lies on both sides would prove Areas 2 and 5, but the region on the far side of 177377 (Area 3's) does not close among the lines the reader reads (the Python version of the rule finds Area 3's point in no loop, with the slanted line 178310 and the join of 177772 unread), so the reader cannot tell it from the outside of the building. The argument is therefore not used.
- Whether any other model has such a line. This file has two, both the same wall type, both in one courtyard.

## 6. The rule

A line is read as **picked from a wall** when all of these hold:

1. Its reference list names exactly one wall whose record box was read, and no wall whose record box was not read (a wall whose copies disagree, or whose box is not a number). A line that also names such a wall stays unread.
2. Its record box is 0.05 to 1 ft across.
3. The wall's record box runs the same way (its long side along the box's long side).
4. The box's width equals half the wall's thickness (the short side of its record box) to 0.001 ft.
5. One long edge of the box lies on the centre line of the wall's record box, and the other on one of the box's two sides, both to 0.001 ft. For a wall turned off the axes that side is a corner of its record box, so such a wall is refused: its short side is not its thickness.

The line is then the wall's centre line, from one end of the box to the other. Only the stretches of it more than **one wall thickness beyond the wall's record box** are taken as drawn there. On those, no wall face can move the line, and the line is the straight line Revit drew; in the export the outline of Areas 2 and 5 there lies at y 73.267, which is neither the centre line (73.016) nor a face (72.789, 73.243) of the partition 143858 that runs beneath it from x 26.75 to 80.61. A free end that RE-330's rule carries on (by at most 1 ft) is part of the line when an outline is tested, and is never proven: an outline that runs along the stub is refused.

An Area whose outline runs along the line on any other stretch has no body, and says `wall_line_position_not_proven`, **after** every other refusal: an Area whose lines it names do not bound its region, or whose region another Area shares, says that first. An Area whose outline runs along the line only on proven stretches is drawn, and says `area_boundary_wall_line` (taking precedence over `area_boundary_joined`). A line that does not meet the conditions is left out of the arrangement with its box, as before (RE-330).

For 177377 the host wall's record box ends at x -4.521, so the stretch from x -3.527 to 80.279 is read; Area 2's edge runs from x 39.969 to 69.780 and Area 5's from 69.780 to 80.279. For 177363 the host wall ends at x -4.748 and the line at -4.875: no stretch is read, and every Area along it is refused.

The walls' record boxes come in the same pass over the file's records as the lines', so no scan is added; the BIM4LCA office, whose 689 lines are all axis lines, reads no wall.

**How each number was chosen, and what the data can pin.** The file has two such lines, so none of these is fixed by the data; each is chosen to be wide of what was measured and narrow of what was not.

- **Tolerance 0.001 ft.** Over the 10 `rac_advanced` saves the largest residual of any line against its wall is 5e-7 ft (a face, `face` in the probe, at the probe's six printed decimals), 0 for the centre lines and the two compound lines, and the nearest line that is neither on a centre line nor on a face is 0.041 ft off (the 440 `other` rows). Any tolerance from 1e-6 to 0.04 ft classifies every line alike here. 0.001 is 2000 times the residual and a fortieth of the nearest other line. The first version of the reader and the probe used 0.006, which gave the same counts.
- **The margin of one wall thickness.** Areas 2 and 5 run 44.5 ft and more beyond the host wall's end, so **any margin from 0 to 44 ft gives the same Areas here**. One thickness is chosen because the joins of RE-330 act within about a thickness of a wall's end (carries of 0.497 and 0.666 ft against a thickness of 0.994 ft) and because the lines picked from the exterior and concrete walls run past their host's drawn body (177370, on the exterior wall 154427, by 0.33 and 0.50 ft): a margin of one thickness keeps every such join out of the proven stretch. No measurement fixes it larger or smaller.
- **Across 0.05 to 1 ft.** The box width of a compound line is half the wall's thickness, 0.497 ft on both lines, so any lower bound up to 0.497 and any upper bound from it gives the same lines. 0.05 is just above 0.041 ft, the offset of a curtain wall's line from its axis (the smallest distance from a wall feature that any other line has), and keeps a degenerate box out; 1 ft is half of a 2 ft wall.
- **"The stored curve is the centre line."** The reader never reads the curve: it takes the centre line from the box edge that lies on the wall's centre line (condition 5). That this is the line's stored curve was checked on the two lines (`probe_re330_area_line_curves`, run 41832621319: 177377 at y 73.267030 from x 80.278543 to -32.143371, 177363 at y 19.906295 from x -4.874670 to -32.312334), and on no other.

## 7. Results

Measure, `--ref main`, filtered, all green. These runs are of `8ed50748` (the probe alone of `3a3ceeeb`). The reader was then changed (section 6: a stub carried on is checked, the refusal order, the hosts and the walls' scan); **the filtered run on the final head, whose id is in the pull request, re-checks the counts below and the refusal reasons, which are expected to move as the last paragraph of this section says.**

**The probe** (run 41862228192, the 10 `rac_advanced` saves): on every save but 2021's it reads 127 lines and 180 walls, with the counts of section 2 and the two compound lines with their stretches beyond one wall thickness (`177377`: line x -32.143371 to 80.278543, wall x -32.312334 to -4.521160, **-3.527065 to 80.278543**; `177363`: line x -32.312334 to -4.874670, wall x -32.312334 to -4.748358, none). The 2021 save has the same counts and 44 walls whose copies disagree.

**The reader against main on the samples 2015 to 2024** (run 41864933862, `-f base=main`, 43 pairs): the IFC of **9 saves moves and 34 do not**. The 9 are the `rac_advanced` saves of 2015 to 2020, 2022, 2023 and 2024; the 2021 `rac_advanced`, every `rst_basic` save and every other sample are byte for byte as before. The scorer diff is `schema.txt` of those 9 only: `sets` +2 and swept solids with a Position of their own +2 (one per Area drawn), and in no `counts.txt`.

**Per save, Areas** (run 41864937912, IFC kept; the diagnostics' `skipped` / `exported.area_body_refusals` and the markers in the IFC):

| Save | Areas written | with `area_boundary` / `_joined` / `_wall_line` | Areas not written, by reason |
|---|---:|---|---|
| 2015, 2016, 2017, 2018, 2019, 2020 | 18 | 6 / 10 / 2 | 7, at `8ed50748`: `point_in_no_single_region` 4 (Areas 3, 7, 8, 10), `wall_line_position_not_proven` 3 (Areas 1, 4, 6) |
| 2021 | 0 | none | 25: `boundary_lines_not_readable` 22, `named_lines_not_in_scheme_and_level` 3 (as RE-330) |
| 2022, 2023 | 18 | 6 / 10 / 2 | the same 7 |
| 2024 | 18 with a body, of 25 written | 6 / 10 / 2 | 7 written without a body (`exported.area_body_refusals`): the same reasons |

**The 2020 save against the export** (`tools/re/space_profiles_vs_ifc.py model.ifc rac_advanced_sample_project.ifc --match globalid`, with the export fetched by hand from the pin): `IfcSpace` in rvt-rs 109 (91 rooms and 18 Areas), in Revit 116 (91 and 25); in both 109, **only in rvt-rs 0, only in Revit 7**. For the Areas that is, **written 18, Revit's 25, extra 0, missing 7** (Areas 1, 3, 4, 6, 7, 8, 10). By marker, area within 0.1% / outline within 0.01 ft / both / boundary within 0.01 ft / lowest and highest vertex within 0.01 ft:

| Marker | Areas | Area | Outline | Both | Boundary | Height |
|---|---:|---:|---:|---:|---:|---:|
| `area_boundary` | 6 | 6 | 6 | 6 | 6 | 6 |
| `area_boundary_joined` | 10 | 10 | 10 | 10 | 10 | 10 |
| `area_boundary_wall_line` | 2 | 2 | 2 | 2 | 2 | 2 |

(The 91 rooms are not part of this change; they score as before: 42 of 85 `stored_solid` rooms within 0.1% and 0.01 ft, and the 6 `record_box` rooms none.) **This is in-sample for the 2 `area_boundary_wall_line` Areas, and for the 10 joined**: the rule was read off the same export.

The 2022 and 2023 saves have no export of their own; their Areas are drawn from the same lines as the 2024 save's reader, and Areas 2 and 5 are not scored against the export there. Areas 2 and 5 do not touch the 16 lines that moved between the 2020 and 2022 saves, but Area 2's north edge does (RE-330 section 4): in the 2022 to 2024 state it is 0.0246 ft off the export, which is of the 2015 to 2020 state.

**The held-out BIM Whale project** (run 41864945459, `holdout-bimwhale*`, counts only): its IFC is **byte for byte main's** (the same sha256), and the scorer diff is empty. `IfcSpace` written 0, Revit's 8, **extra 0**, global ids equal 0 of 0, spaces matched 0. That is the bar (zero extra Areas), met vacuously, as in RE-324 and RE-330: the project draws no Area body, so none of its Areas goes through the wall rule. Whether it holds a line the rule would read is not measured.

**The office** (`nordic-office-as-built`, `nordic-office-permit`): the first run, 41864941448, ran without `-f nordic=true` and measured nothing; the repeat, 41868440397, is queued. The office's 689 lines are all axis lines, so the walls are not read there and its IFC is expected byte for byte as before; the unfiltered run is the check.

**Expected at the final head, not yet measured.** The 18 Areas and their markers do not move (Areas 2 and 5 meet every condition, and no outline of the 16 others runs along a compound line). What moves is the reason of Areas 1, 4 and 6, because the wall refusal now comes after the others. The Python port of the arrangement (`tools/re/area_joins_vs_ifc.py`'s rule, run on the 2024 save's probe output) puts Areas 4 and 6 in one region of 2638.383 ft2 whose outline holds the lines each of them names (177363, 177963; 177353, 177367), so neither fails the check of the lines it names. Both then share a region, which is the existing `two_areas_one_region`: that is Area 6's true cause (the 0.101 ft join is open), and Area 4's as well as its unproven stretch. Area 1 alone stays `wall_line_position_not_proven`. The expected per-save list is therefore `point_in_no_single_region` 4 (Areas 3, 7, 8, 10), `two_areas_one_region` 2 (Areas 4, 6) and `wall_line_position_not_proven` 1 (Area 1). The run on the final head confirms or corrects this.

Runs 41863421756, 41863426128, 41864308314 and 41864311445 are of `c79a89e7`, which read a wall-hosted line at its host wall's centre rather than at its own edge; they are superseded.

## 8. What it leaves out

- Areas 1, 3 and 4: sections 4 and 5. Area 3 also needs the slanted line 178310 (it says `point_in_no_single_region`); Area 4 the 0.101 ft join of Area 6. Area 1 says `wall_line_position_not_proven`.
- Area 6 has no compound line on its outline, but its region is not closed at the 0.101 ft join (RE-330 section 3), so it is Area 4's region: it, and Area 4, are expected to say `two_areas_one_region` (section 7).
- Areas 7, 8 and 10: unchanged from RE-330 section 5 (the slanted line, the arc). They have no compound line.
- The 2021 save, unchanged (two copies of 16 lines, RE-324 section 7).

## 9. Not measured

- Any Revit IFC4 export of these Areas. None exists.
- That the rule holds on any model it was not read off. The only compound lines found are 177363 and 177377.
- The 2022 to 2024 state of the 16 edited lines against any export (Area 2's north edge is one of them).
- Which face or centre an exterior wall of a type the file does not hold would get, and the area scheme's own rules.
- The office with the rule (run 41868440397 pending), and whether BIM Whale holds a line the rule would read.

## 10. Reproduce

```bash
# the export, by pin; never stored
curl -fsSL -o rac_advanced_sample_project.ifc \
  https://raw.githubusercontent.com/mrdoob/three.js/7c1f47731608de51a2240ce31655634665cb51ce/examples/models/ifc/rac_advanced_sample_project.ifc
sha256sum rac_advanced_sample_project.ifc   # 9440a72eb2d27e3ba9edeafdcdfb71945e6baec56d5b09211bcde743de7fe0a5
# the probe on the samples
gh workflow run measure.yml --ref main -f ref=<sha> -f base=none -f samples=true -f sample_years="2015 2024" \
  -f models="zzz-no-model" -f scorers=none -f probe="probe_re332_wall_hosted_lines"
# the reader, every save with its IFC kept; then score the 2020 save against the export
gh workflow run measure.yml --ref main -f ref=<sha> -f base=none -f samples=true -f sample_years="2015 2024" \
  -f models="zzz-no-model" -f scorers=none -f keep_ifc=true
python3 tools/re/space_profiles_vs_ifc.py model.ifc rac_advanced_sample_project.ifc --match globalid --list 10
```
