# RE-330: Areas on Revit 2015 to 2023 against a Revit export, and the joins of area boundary lines

**Date:** 2026-11-04
**Issues:** #1261 (BIM Whale's Areas, the Areas of 2015 to 2023 files), #1194; follows RE-317 (an Area's boundary), RE-324 (Areas on 2015 to 2023)
**Artefacts:** Autodesk's `rac_advanced_sample_project`, saves of 2015 to 2024 (`research/autodesk-sample-projects.tsv`); a public Revit 2021 IFC export of the same project (below); the BIM4LCA office, as built and at building permit (Revit 2024; no change expected); the held-out BIM Whale 2021 project, by counts only
**Probe, tool:** `probes/examples/probe_re330_area_line_curves.rs`; `tools/re/area_joins_vs_ifc.py`
**Status:** positive on one project. Against the one Revit export of an Area-bearing 2015 to 2023 project that exists, 16 of the 25 Areas of `rac_advanced` equal Revit's outline in area to 0.0001 ft² once the area boundary lines are joined as their records say. The 4 Areas RE-317 and RE-324 called "bounded partly by walls" are among the 16 and no wall bounds any of them. 9 Areas are left out, and why. The Rust reader is not yet measured: its runs are in section 7.

## 1. A Revit export of a 2015 to 2023 project with Areas

RE-324 said that no Revit export of any 2015 to 2023 file with Areas was in the corpus. There is one, public since 2021-03-30, and it is of the project the Areas were developed on:

- `examples/models/ifc/rac_advanced_sample_project.ifc` of mrdoob/three.js at commit `7c1f47731608de51a2240ce31655634665cb51ce` (the only commit that touches the file), 42,831,859 bytes, sha256 `9440a72eb2d27e3ba9edeafdcdfb71945e6baec56d5b09211bcde743de7fe0a5`. The repository is MIT; the file's own terms are not stated and were not checked, so it is fetched for a run and never stored, as the other third-party exports are.
- What it is: an IFC2X3 Coordination View 2.0 export by Revit 2021 (Spanish edition, wall types named `Muro básico:...`), file created 2021-01-22, by exporter "21.0.0.383 - Exporter 21.0.0.383 - IU alternativa 21.0.0.383", a third-party build of the exporter. It is counted apart from Revit's own IFC4 exports, as the held-out IFC2X3 exports are (Gdynia, BIM Whale), and it is a different kind of evidence from the office's: one project, exported once.
- What it holds: 116 `IfcSpace`: 91 rooms and the project's 25 Areas (numbers 1 to 25, long names Service, Administration, Cirrculation and Instruction), each with an `IfcExtrudedAreaSolid` 3800 mm deep (12.4672 ft, the distance to the Level above, as RE-317 measured on the office): 14 with an arbitrary closed profile (Area 10's holds one arc segment) and 11 with a rectangle profile (Revit writes a rectangular Area as a rectangle).
- Its Areas pair with rvt-rs's by GlobalId and number: **25 of 25** (the GlobalIds the 2024 save derives, `probe_re317_area_boundaries`).
- Its units are millimetres and its `IfcSite` is placed at (9201.999, 22635.061) mm, the shared-coordinates offset: Revit's internal coordinates are the export's minus (30.19029, 74.26201) ft, no rotation. `tools/re/ifc_common.py::site_frame` read the placement as metres whatever the file's unit; it now scales it, which changes nothing for a file in metres or placed at the origin, and puts this export in the project's internal feet. This is a defect of the shared scorer found here, not a finding about any reader: **any scorer line of a millimetre export with a placed site would have been read through the wrong frame before**, and an unfiltered Measure run is what shows which held-out lines it moves.

**Which state it is of.** Autodesk's sample holds 16 area boundary lines at two positions (RE-324 §4): the 2015 to 2020 saves at one, the 2022 to 2024 saves at another 0.0246 ft away, and the 2021 save at both. The export draws the first. Putting those 16 lines' y back to the 2015 to 2020 values on the 2024 save's boxes gives the 2020 save's boxes on **127 of 127** lines (to 1.5e-6 ft), so the export is scored against the 2015 to 2020 saves, and against the others only where their lines are the same.

## 2. What Revit's outlines are made of

Each of the 25 outlines, moved into internal feet and laid over the area boundary lines of its Level (the lines' boxes, read as RE-317 reads them) and over the footprints of the project's 146 walls in the same export:

- **Edges on area boundary lines.** Most of every outline. Where Revit's edge is longer than the line that carries it, the line's neighbours carry the rest.
- **Lines that stop short of the line they join.** A line drawn along a wall stops at the centre of the wall it meets: on Level 3 (38 lines) 35 ends lie on no other line; 23 of them are carried on by Revit to the perpendicular line that names their line and that their line names, at distances of 0.041 ft (9 times), 0.497 ft (half of the exterior walls' 0.994 ft, 5 times) and 0.666 ft (3 times), and 6 are already on its line's extension. The largest distance on any of the three Levels is 0.666 ft (Level 1 has 30 such ends of 59, Level 2 31 of 51).
- **Two parallel lines that stop at the same place, 0.625 ft apart.** 4 sites on each Level; Revit draws a 0.625 ft segment across each. The 8 ends of Level 3's four sites are joined; 4 other free ends of that Level (stubs) bound nothing.
- **No wall.** Of the four jog sites, two lie on the end face of an exterior wall (139854 and 139857, whose faces are 0.994 ft long) and two lie across the body of exterior wall 148769 (x -32.809 to -31.815 ft, y 7.416 to 73.764 ft; the segments are at y 7.644 and 73.417 ft). A segment that crosses a wall's body is not a wall face, so no rule that draws the wall's face explains all four, and the joins explain all four.
- **Wall faces and centrelines do appear, in 5 Areas.** The outlines of Areas 1 to 5 (Level 1) follow the south face of wall 169770 (y 72.770 ft) west of x -13.405 ft and its centreline (y 73.267 ft) east of it; Area 4's north edge follows the north face of wall 144837 (y 20.403 ft) west of x -13.405 and its centreline (y 19.906 ft) east of it. The step at x -13.405 ft is 0.041 ft from area boundary line 177364 (x -13.446 ft). The two area boundary lines that name those walls, 177377 (box 112.422 by 0.497 ft) and 177363 (27.437 by 0.497 ft), are the two whose boxes are slanted; their stored curves are the subject of the probe (§7).

## 3. The rule

`partition_area_boundaries::Arrangement::new` takes, with its lines, the ElementIds each line's record names (its reference list), and:

1. **Carries a free end on.** An end that lies on no other line (1e-5 ft) is carried on, along its own line, to the farthest line that is perpendicular, that its line names and that names its line, past any nearer line it crosses, to the point where the two lines meet; no distance limit. A corner that stops short at both ends is closed by each line's own end.
2. **Joins what cannot be carried on.** A free end with no such line is joined by a connector (`CONNECTOR_ID`, never an ElementId) to the free end of a parallel line that names it and is named by it, when the two ends are at the same place along the lines, each is the other's only such end, and neither was carried on.
3. **Leaves the rest free.** An end whose line names no such partner stays free, and the existing rule (RE-317 §3) drops its line where it leads nowhere.

A line that is not a straight line along an axis (a slanted line, an arc, a point: its record holds only its box) is no longer a reason to refuse every Area of its group. It is left out of the arrangement, with its box, and so is a readable line that names one (that line may be joined to it). `partition_area_records` refuses an Area whose region's box comes within 0.01 ft of one of those boxes (`boundary_lines_not_readable`): a line outside the box of a region cannot lie in it, cross it or merge it with another.

**What decides is the record, not a distance.** The rule has no distance limit. Every carrying-on that the outlines need is between two lines that name each other, and in the one place Revit also closes a gap that the records do not name, the rule leaves the end free and draws nothing: Area 6's east line (177772) stops 0.101 ft short of line 177963, names nothing on that side, and 177963 names it back nowhere, so Area 6 has no region and is not drawn. The 32-bit saves' reference lists equal the 2024 save's on **127 of 127** lines in every save from 2015 to 2023, in the same order (the lines of run 41754347940's probe output).

**The office is inert.** On both office files (689 lines in 21 groups of a scheme on a Level, no line off the axes) the Python version of the rule carries on 0 lines, adds 0 connectors and leaves out 0 lines (run 41713925390's probe output): the two free ends of the Basement's stubs have no line that names them and that they name, at any distance. The Rust reader's runs are in section 7.

## 4. Results of the rule, as a Python tool against the export

`python3 tools/re/area_joins_vs_ifc.py probe.txt rac_advanced_sample_project.ifc --old-edit` on the 2024 save's probe output (run 41714706290), the lines of the three Levels, by GlobalId:

| | Areas |
|---|---:|
| in Revit's export | 25 |
| drawn by the rule | 16 (9, 11 to 25 but 10) |
| of those, symmetric difference from Revit's outline within 0.001 ft² | **16** (0.0000 to 0.0001 ft²; their areas 67.6 to 6889.1 ft²) |
| not drawn | 9 (1 to 8, 10) |

Without `--old-edit`, the 2022 to 2024 state of the 16 edited lines: 13 of the 16 stay within 0.001 ft²; 12, 19 and 22 differ by 2.77, 0.23 and 2.77 ft², the three whose outlines run along the moved lines. Areas 23, 24 and 25 are the three RE-317's reader already draws; the 13 others are new.

The first prototype of the rule (Python over shapely, before the tool) drew the same 16 with the same differences. Two choices in it were fixed by the data: **farthest, not nearest** (line 179110's end must pass line 179091, which begins on it, to reach 179090: Areas 20 and 21 share the line and need different lengths of it), and **a connector only between ends that were not carried on** (the nearest-line variant gave Area 9 a 0.04 ft² notch where a 0.041 ft connector crossed the pocket the Area occupies).

## 5. What it leaves out

| Area | Level | Why |
|---|---|---|
| 1, 2, 3, 4, 5 | 1 | lines 177363 and 177377 are slanted by their boxes; the outline follows wall 169770 and wall 144837 as in §2 |
| 7, 8 | 1 | line 178310 (box 0.0075 by 27.337 ft) is a nearly vertical straight line that Revit draws as a slanted edge; a general arrangement of slanted lines is not built |
| 6 | 1 | line 177772 stops 0.101 ft short of 177963 and names nothing there (§3) |
| 10 | 2 | line 178788 (box 2.494 by 14.292 ft) is an arc, which Revit draws as an arc segment of the profile |

Also not drawn: Areas of the 2021 save whose lines have two copies (the group is refused, RE-324 §7), which is **not** changed here.

## 6. Not measured

- The stored curves of lines 177363, 177377, 178310 and 178788 (the probe below).
- Any release but the 2020 save against the export: the other 2015 to 2020 saves are the same lines; the 2021 save's two copies and the 2022 to 2024 state differ from the export by the 16 edited lines (§1).
- Areas bounded by a wall: none is, in the one export measured (§2). An Area whose lines do not close even with their joins stays unwritten, and how Revit bounds such an Area is not measured.
- Any Revit export of an Area-bearing 2015 to 2023 project but this one.
- Whether the export would hold an Area the rule does not draw if its lines were joined differently: only the outlines of Areas 1 to 8 and 10 were laid over the lines, not compared by a rule.

## 7. Runs

(to be filled by the Measure runs of this change)

## 8. Reproduce

```bash
# the export, by pin; never stored
curl -fsSL -o rac_advanced_sample_project.ifc \
  https://raw.githubusercontent.com/mrdoob/three.js/7c1f47731608de51a2240ce31655634665cb51ce/examples/models/ifc/rac_advanced_sample_project.ifc
sha256sum rac_advanced_sample_project.ifc   # 9440a72e...e0a5
# the probe output of the 2024 save (RE-317's probe, Measure -f samples=true -f sample_years="2024 2024")
python3 tools/re/area_joins_vs_ifc.py probe.txt rac_advanced_sample_project.ifc --old-edit
```
