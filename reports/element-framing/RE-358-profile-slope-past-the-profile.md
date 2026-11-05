# RE-358: the sloped line of a wall's edited elevation profile goes on, along its slope, to the wall's end

**Date:** 2026-11-05
**Issues:** #1473 (the far-end stub of the sloped top); follows RE-348 (#1410, the wall ends), RE-151 (the openings an elevation profile cuts)
**Artefacts:** the BIM4LCA office as built and at building permit (Revit 2024, Revit 24.0.5's IFC4 exports, CC BY-SA 4.0, `research/bim4lca-measure.tsv`) and Core Interior (Revit 2024, MIT), as Measure fetches them
**Tools:** `tools/re/wall_heights_vs_ifc.py`, `tools/re/wall_quantities_vs_ifc.py`, `tools/re/wall_top_cover_vs_ifc.py`, `tools/re/wall_layers_vs_ifc.py`, `tools/re/wall_rule_counts.py`
**Runs:** pending: no run has measured this rule yet.
**Status:** pending a run. Nothing below section 4 is measured.

## 1. What is known

RE-348 (section 5, Measure 42101944733) left the Basement walls 2057668, 2057670, 2058499 and 2058501 of the BIM4LCA office 0.209 ft3 over Revit's volume as built and 0.207 at permit, the same on all four. A wall whose elevation profile was edited has a sketch of lines whose sloped top line ends at the profile's end (y 79.9705 ft), while the wall's body goes on to the face of the wall at its far end (y 80.3806 ft). Revit's top goes on sloping to that face (183.085 ft there); rvt-rs's opening (RE-151) stops at the profile's end, so the 0.4101 ft stretch stays flat at the box's top (183.7343 ft): a wedge 0.8202 ft thick, 0.4101 ft long and 0.592 ft deep at its near end and 0.650 ft at the far, which is 0.2088 ft3. The record's line is the `GLine` of the profile (origin y 75.0149 ft, z 183.8339 ft, slope -0.13960 ft/ft): carried to y 80.3806 ft it gives 183.0848 ft, Revit's 183.085.

What is read from the file is the line's slope and the profile's ends (the sketch's lines, RE-97, RE-151); the wall's extent past them is the body rvt-rs already draws. That Revit's top goes on along the line's slope is a rule the office's four walls show against Revit's export; it is not stated by any byte of the file, and rests on 4 walls in one project.

## 2. The rule

`profile_extent` (`src/research/partition_schema_mvp/walls.rs`) records, with each wall's profile openings, the lowest and highest points of its sketch along the profile's own axis (`m_profile_extent`, Revit 2024 only, with the openings). `wall_profile::extend_profile_stubs` (`src/ifc/export_content/wall_profile.rs`) runs after the wall ends and the base cuts, when each wall has its final extent. For each opening of a wall that is one plain extrusion carrying Revit's record property set, at each end of the profile: if the body goes on past the end by more than 0.001 ft, and the opening reaches that end with exactly two vertices on one vertical edge, one of which is the end of a sloped line and the other the corner of the profile's rectangle (its other edge level), both move out to the wall's end: the corner straight out, the line's end along the line's slope. The cut stays on its side of the line.

A wall of which this is not read keeps the top the opening gives it past the profile, with the reason as `ProfileStubRefusal` (`exported.wall_profile_refusals`, the decision `wall_profile_extension_refused`):

- `profile_cut_not_read_at_the_wall_end`: the opening reaches the end with other than one such edge (counted only where the opening comes down into the wall's height there);
- `profile_stub_longer_than_the_wall_is_thick`: a join extends a wall by at most its thickness;
- `profile_slope_leaves_the_cut`: the slope would carry the line to the wall's base or across the rectangle's edge before the wall's end.

A wall whose body goes on past the profile where no opening reaches that end, or where the line stays above the wall's top across the stretch, is not changed and not counted. A wall that carries the change has `ProfileStubSource` `wall_profile_sloped_past_the_low_end`, `wall_profile_sloped_past_the_high_end` or `wall_profile_sloped_past_both_ends` (`exported.wall_profile_sources`).

## 3. Predicted before the run

The rule moves the top of the stretch from 183.7343 ft to the line: a drop of 0.1396 ft/ft x 0.4101 ft = 0.0573 ft at the wall's end (RE-348 measured 0.650 - 0.592 = 0.058), and removes 0.8202 x 0.4101 x (0.592 + 0.650) / 2 = 0.2089 ft3. So each of the four walls should move by about 0.209 ft3 as built and 0.207 at permit, to Revit's volume within 0.01 ft3, and their `wall_top_cover_vs_ifc` plan rows (`equal 90 taller 10 lower 0` of 100 as built) should lose the ten `taller` points. `exported.wall_profile_sources` should be 4 on each office file and `exported.wall_profile_refusals` empty. Walls 2056705 and 2056770 (profile-edited too, with the same far end) may move as well; they are #1474's walls. Any other movement is a surprise.

## 4. Measured

Not yet.

## 5. Not measured

Everything: the run has not been made.
