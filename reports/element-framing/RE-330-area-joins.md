# RE-330: Areas on Revit 2015 to 2023 against a Revit export, and the joins of area boundary lines

**Date:** 2026-11-04
**Issues:** #1261 (BIM Whale's Areas, the Areas of 2015 to 2023 files), #1194; follows RE-317 (an Area's boundary), RE-324 (Areas on 2015 to 2023)
**Artefacts:** Autodesk's `rac_advanced_sample_project`, saves of 2015 to 2024 (`research/autodesk-sample-projects.tsv`); a public IFC2X3 export of the same project by a third-party build of Revit's 2021 exporter (§1); the BIM4LCA office, as built and at building permit (Revit 2024); the held-out BIM Whale 2021 project, by counts only
**Probe, tool:** `probes/examples/probe_re330_area_line_curves.rs`; `tools/re/area_joins_vs_ifc.py`
**Status:** positive on one project, with one external check. 16 of the 25 Areas of `rac_advanced` are drawn on every save but 2021's, from area boundary lines joined as their records say; the 16 drawn on the 2020 save are within 0.1% in area and 0.01 ft in outline of the public IFC2X3 export, with no Area written that it lacks. The 4 Areas RE-317 and RE-324 called "bounded partly by walls" are among the 16 and no wall bounds any of them. 9 Areas are left out, and why (§5). That export is the only external check there is of the joined Areas (§1).

## 0. What the check rests on

- The only external check of an Area drawn from lines carried on to their joins is **one public IFC2X3 export, by a third-party build of Revit's 2021 exporter, of Autodesk's `rac_advanced`, from the three.js repository, which shows the model's 2015 to 2020 state**. It is not Revit's own IFC4 export. The standing rule counts IFC2X3 views and files made by other tools apart; this is that kind of file, used here as a geometry oracle for these Areas.
- The 2022 and 2023 saves of the project are also checked, against the 2024 save's reader (§4). That is an agreement between two readings of the same project, not an external check.
- **No Revit IFC4 export of these Areas exists.**
- The export is **never used to remove a value** from an rvt-rs export or to settle a disagreement between Revit's exporters.
- The Areas that needed a join say so: `RvtElementRecordGeometry.SpaceOutlineSource` is `area_boundary_joined` for an Area whose outline runs along a stretch of a line carried on past its record or along a connector, and stays `area_boundary` for an Area drawn from the lines as their records give them (the office's 30, and the Areas of `rac_advanced` that need no join).

## 1. The export, and what it is

RE-324 said that no Revit export of any 2015 to 2023 file with Areas was in the corpus. This one is public since 2021-03-30 and is of the project the Areas were developed on:

- `examples/models/ifc/rac_advanced_sample_project.ifc` of mrdoob/three.js at commit `7c1f47731608de51a2240ce31655634665cb51ce` (the only commit that touches the file), 42,831,859 bytes, sha256 `9440a72eb2d27e3ba9edeafdcdfb71945e6baec56d5b09211bcde743de7fe0a5`.
- **Its terms.** The repository is MIT. The file carries no licence of its own that could be found, the pull request that added it (three.js #20598) says nothing of it, and Autodesk's terms for the sample project it exports were not checked (as for the samples `tools/fetch-autodesk-samples.sh` fetches). Whether those terms allow a Measure run to fetch it is therefore **not known**. No workflow fetches it; the check is run by hand from the pin above. If one is made to, it should be handled like the other third-party exports: fetched for the run, never stored.
- What it is: an IFC2X3 Coordination View 2.0 export by Revit 2021 (Spanish edition, wall types named `Muro básico:...`), file created 2021-01-22, by exporter "21.0.0.383 - Exporter 21.0.0.383 - IU alternativa 21.0.0.383", a third-party build of the exporter. Gdynia (exporter 21.0.0.383 too) and BIM Whale (21.1.10.26), held-out, are IFC2X3 exports of Revit 2021 and are counted apart in the same way. The header's "IU alternativa" is Spanish for an alternative user interface, and nothing in the file says the build is Autodesk's own IFC4 exporter; it is labelled a third-party build because that is not established, and because it is IFC2X3.
- What it holds: 116 `IfcSpace`: 91 rooms and the project's 25 Areas (numbers 1 to 25, long names Service, Administration, Cirrculation and Instruction), each with an `IfcExtrudedAreaSolid` 3800 mm deep (12.4672 ft, the distance to the Level above, as RE-317 measured on the office): 14 with an arbitrary closed profile (Area 10's holds one arc segment) and 11 with a rectangle profile.
- Its Areas pair with rvt-rs's by GlobalId and number: **25 of 25**.
- Its units are millimetres and its `IfcSite` is placed at (9201.999, 22635.061) mm, the shared-coordinates offset: Revit's internal coordinates are the export's minus (30.19029, 74.26201) ft, no rotation. `tools/re/ifc_common.py::site_frame` read that placement as metres whatever the file's unit, so the project's scorers cannot place this export until it is fixed. The fix is its own change (PR #1353, with the isolation runs); `tools/re/area_joins_vs_ifc.py` and `space_profiles_vs_ifc.py` need it to read the export.

**Which state it is of.** Autodesk's sample holds 16 area boundary lines at two positions (RE-324 §4): the 2015 to 2020 saves at one, the 2022 to 2024 saves at another 0.0246 ft away, and the 2021 save at both. The export draws the first. Putting those 16 lines' y back to the 2015 to 2020 values on the 2024 save's boxes gives the 2020 save's boxes on **127 of 127** lines (to 1.5e-6 ft), so the export is scored against the 2015 to 2020 saves, and against the others only where their lines are the same.

## 2. What Revit's outlines are made of

Each of the 25 outlines, moved into internal feet and laid over the area boundary lines of its Level (their record boxes, read as RE-317 reads them) and over the footprints of the project's 146 walls in the same export:

- **Edges on area boundary lines.** Most of every outline. Where Revit's edge is longer than the line that carries it, the line's neighbours carry the rest.
- **Lines that stop short of the line they join.** On Level 3 (38 lines) 35 ends lie on no other line; 23 of them are carried on by Revit to the perpendicular line that names their line and that their line names, at distances of 0.041 ft (9 times), 0.497 ft (half of the exterior walls' 0.994 ft, 5 times) and 0.666 ft (3 times), and 6 are already on its line's extension. The largest distance on any of the three Levels is 0.666 ft (Level 1 has 30 such ends of 59, Level 2 31 of 51).
- **Two parallel lines that stop at the same place, 0.625 ft apart.** 4 sites on each Level; Revit draws a 0.625 ft segment across each. The 8 ends of Level 3's four sites are joined; 4 other free ends of that Level (stubs) bound nothing.
- **No wall.** Of the four sites, two lie on the end face of an exterior wall (139854 and 139857, whose faces are 0.994 ft long) and two lie across the body of exterior wall 148769 (x -32.809 to -31.815 ft, y 7.416 to 73.764 ft; the segments are at y 7.644 and 73.417 ft). A segment that crosses a wall's body is not a wall face, so no rule that draws a wall's face explains all four, and the joins explain all four. None of the 16 Areas drawn is bounded by a wall.
- **The record box is where Revit draws the line, not the stored line.** The probe (§5) reads each line's own curve from its element data. On the 2020 save the stored line lies inside the record box for 57 of the 127 lines and outside it for 70, by 0.041 ft (47 lines), 0.369 ft (4), 0.497 ft (18) and, for the arc, a misread (1). The 16 Areas are drawn from the boxes and equal Revit's outlines to 0.0001 ft², with lines whose stored positions are 0.041 to 0.497 ft off those boxes on their outlines. What moves the box off the line is not examined.

## 3. The rule

`partition_area_boundaries::Arrangement::new` takes, with its lines, the ElementIds each line's record names (its reference list), and:

1. **Carries a free end on.** An end that lies on no other line (1e-5 ft) is carried on, along its own line, to the farthest line that is perpendicular, that its line names and that names its line, past any nearer line it crosses, to the point where the two lines meet; no distance limit. A corner that stops short at both ends is closed by each line's own end.
2. **Joins what cannot be carried on.** A free end with no such line is joined by a connector (`CONNECTOR_ID`, never an ElementId) to the free end of a parallel line that names it and is named by it, when the two ends are at the same place along the lines, each is the other's only such end, and neither was carried on.
3. **Leaves the rest free.** An end whose line names no such partner stays free, and the existing rule (RE-317 §3) drops its line where it leads nowhere.
4. **Marks what it needed.** `AreaFace.joined` is true when the outline, a hole's too, runs along a stretch of a line past the extent its record gives, or along a connector; the Area then says `area_boundary_joined`.

A line that is not a straight line along an axis (by its record box: a slanted line, an arc, a point, and a line whose box is wider than the line) is no longer a reason to refuse every Area of its scheme on its Level. It is left out of the arrangement, with its box, and so is a readable line that names it (that line may be joined to it). `partition_area_records` refuses an Area whose region's box comes within 0.01 ft of one of those boxes (`boundary_lines_not_readable`): a line outside the box of a region cannot lie in it, cross it or merge it with another. Group-level refusals (copies that disagree, a design option, a line of unknown group) are unchanged, and two copies of a line that differ in their reference list now make it a conflicted line, as two that differ in box do.

**What decides is the record, not a distance.** The rule has no distance limit. Every carrying-on that the outlines need is between two lines that name each other, and in the one place Revit also closes a gap that the records do not name, the rule leaves the end free and draws nothing: Area 6's east line (177772) stops 0.101 ft short of line 177963, names nothing on that side, and 177963 names it back nowhere, so Area 6 has no region and is not drawn. The 32-bit saves' reference lists equal the 2024 save's on **127 of 127** lines in every save from 2015 to 2023, in the same order (the lines of run 41754347940's probe output).

**The office is inert.** On both office files (689 lines in 21 groups of a scheme on a Level, no line off the axes) the Python version of the rule carries on 0 lines, adds 0 connectors and leaves out 0 lines (run 41713925390's probe output): the two free ends of the Basement's stubs have no line that names them and that they name, at any distance.

## 4. Results

**The rule in Python against the export** (`python3 tools/re/area_joins_vs_ifc.py probe.txt rac_advanced_sample_project.ifc --old-edit`, on the 2024 save's probe output, run 41714706290, by GlobalId): 16 of the 25 Areas drawn (9, 11 to 25 but 10); each within 0.001 ft² of Revit's outline (0.0000 to 0.0002), their areas 67.6 to 6889.1 ft²; 9 not drawn (1 to 8 and 10). Without `--old-edit`, the 2022 to 2024 state: 13 of the 16 within 0.001 ft²; Areas 12, 19 and 22 differ by 2.77, 0.23 and 2.77 ft², the three whose outlines run along the moved lines.

**The Rust reader, at 35a9d322** (the lines, joins and refusals; the marker is the commit after):

- Samples 2015 to 2024 against `main`, run 41832625227: the IFC of the 9 `rac_advanced` saves (2015 to 2020, 2022 to 2024) moves and 34 others do not: the 2021 `rac_advanced`, every `rst_basic` save and every other sample are byte for byte as before. Each of the 9 draws **16 Areas** (`IfcSpace` 91 to 107 on 2015 to 2023, where the 9 others are not written; 116 on 2024, 16 with a body and 9 with a refusal); `skipped.area_without_proven_body` is 9 on each of 2015 to 2020, 2022 and 2023 (all `point_in_no_single_region`), and `exported.area_body_refusals` is 9 on 2024. The 2021 save draws none: 25 skipped (22 `boundary_lines_not_readable`, 3 `named_lines_not_in_scheme_and_level`), as RE-324 §7.
- The 2020 save against the export, run 41832628804 (`model.ifc` kept; `space_profiles_vs_ifc.py model.ifc rac_advanced_sample_project.ifc --match globalid`, with the fixed frame): `IfcSpace` in rvt-rs 107, in Revit 116; in both 107, **only in rvt-rs 0**, only in Revit 9 (the 9 Areas not written); the 16 Areas drawn (then still `area_boundary`): area within 0.1% **16**, outline within 0.01 ft **16**, boundary within 0.01 ft 16, lowest and highest vertex within 0.01 ft 16.
- The 2022 and 2023 saves against the 2024 save's reader, the pair probe, run 41833551078: the 25 Areas pair with number, name, Level, scheme, lines and placement equal; the group lines and points equal (inputs equal 25 of 25); the 16 drawn Areas' fields equal 2024's (16 of 16); the other 9 are written by 2024 with a refusal and not written by 2022 and 2023. The 2015 to 2020 saves differ from 2024 in the 16 moved lines, and 5 of the 16 bodies differ in their outline ring: Areas 12, 19 and 22 by the moved lines, and Areas 11 and 25 in the ring but not in the region (their symmetric difference from the export is 0.0000 ft² in both states of the lines; RE-324 §5 found Area 25's ring to differ only in its starting vertex).
- Office, run 41832632747: both files' IFC byte for byte `main`'s, so the 30 Areas are unchanged and exact. BIM Whale, run 41832635945: the IFC is byte for byte `main`'s, counts only: 0 extras and, as before, 0 of its 8 Areas (still undelivered, #1261).

The Rust at the marker commit is measured by the runs of §7.

## 5. What it leaves out, and what the lines store

The probe reads each line's own curve (run 41832621319, the 10 saves of 2015 to 2024; the readings are the same on every save, the 32-bit ones included):

| Area | Level | Line | What is stored | Why it is out |
|---|---|---|---|---|
| 1, 2, 3, 4, 5 | 1 | 177377 (box 112.422 by 0.497 ft) and 177363 (27.437 by 0.497 ft) | a straight horizontal line on the centreline of a wall (y 73.267 ft, wall 169770; y 19.906 ft, wall 144837); the box reaches 0.497 ft toward the wall's face | Revit's outline runs along the line on one side of a vertical line (x -13.4 ft) and along the box's far edge, the wall's face, on the other: the box is not the outline, and the line is not either |
| 7, 8 | 1 | 178310 (box 0.0075 by 27.337 ft) | a straight line from (41.4846, -49.2139) to (41.4921, -76.5507), 0.016 degrees off the axis, its ends inside its box | the only straight line off the axes: Revit's outline follows it as a slanted edge. A general arrangement of such lines is not built; the stored ends are readable on all 10 saves |
| 6 | 1 | 177772 | an axis line | stops 0.101 ft short of 177963 and names nothing there (§3) |
| 10 | 2 | 178788 (box 2.494 by 14.292 ft) | a circular arc, centre (16.758, -11.783), radius 11.483 ft, from (25.747, -4.637) to (25.747, -18.929) bulging to x 28.241; the record has a marked arc record (`u64` 1) before it that neither reader reads | Revit draws the arc as an arc segment of the profile |

So of the four lines whose box is not a line along an axis, **one is a straight line off the axes (178310), two are axis lines whose box is wider than the line, and one is an arc**. Reading Areas bounded by slanted lines is the next step, and 178310 is the one measured case; what moves a box off its line (§2) would have to be known for the two axis lines.

Also not drawn: the 2021 save (two copies of 16 lines, RE-324 §7), unchanged.

Beside the Areas, the same scoring of the 2020 save against the export gives its 91 rooms: 85 from the stored solid (42 within 0.1% in area and 0.01 ft in outline, the others at most 1.24% off in area) and 6 at their record box (none within tolerance); not examined, and not part of this change.

## 6. Not measured

- **Any Revit IFC4 export of these Areas.** None exists. The only Revit export of a 2015 to 2023 Area is the IFC2X3 one of a third-party build of the 2021 exporter, of one project in its 2015 to 2020 state.
- The marker: that the Areas that say `area_boundary_joined` are the ones whose outline needed a join. The Python version of the rule finds 10 of the 16 joined (9, 12, 14, 16 to 22) and 6 not (11, 13, 15, 23 to 25), the 3 Areas of RE-317's reader among them; the Rust counts are in §7.
- The 2022 to 2024 state of the 16 edited lines against any export, and the 2021 save against anything.
- Areas bounded by a wall: none is, in the one export measured. An Area whose lines do not close even with their joins stays unwritten, and how Revit bounds such an Area is not measured.
- Whether Revit's export would hold an Area that the rule does not draw, beyond the 9 it does hold.
- Revit releases 2008 to 2014 and the top storey, Levels that are not building stories, a Level's story above and an Area's base offset, as RE-317 and RE-324 left them.
- Whether the file's terms allow a Measure fetch (§1).

## 7. Runs

(to be filled with the runs of the marker commit)

## 8. Reproduce

```bash
# the export, by pin; never stored
curl -fsSL -o rac_advanced_sample_project.ifc \
  https://raw.githubusercontent.com/mrdoob/three.js/7c1f47731608de51a2240ce31655634665cb51ce/examples/models/ifc/rac_advanced_sample_project.ifc
sha256sum rac_advanced_sample_project.ifc   # 9440a72eb2d27e3ba9edeafdcdfb71945e6baec56d5b09211bcde743de7fe0a5
# the probe output of the 2024 save (RE-317's probe, Measure -f samples=true -f sample_years="2024 2024")
python3 tools/re/area_joins_vs_ifc.py probe.txt rac_advanced_sample_project.ifc --old-edit
# the reader, the 2020 save with its IFC kept
gh workflow run measure.yml --ref main -f ref=<sha> -f base=none -f samples=true -f sample_years="2020 2020" \
  -f models="zzz-no-model" -f scorers=none -f keep_ifc=true
python3 tools/re/space_profiles_vs_ifc.py model.ifc rac_advanced_sample_project.ifc --match globalid --list 10
# the curves of the area boundary lines
gh workflow run measure.yml --ref main -f ref=<sha> -f base=none -f samples=true -f sample_years="2015 2024" \
  -f models="zzz-no-model" -f scorers=none -f probe="probe_re330_area_line_curves"
```
