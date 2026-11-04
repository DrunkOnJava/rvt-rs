# RE-337: a wall whose end candidates stand above its base ends at the face of the wall that holds the base

**Date:** 2026-11-04
**Issues:** #1396 (Basement walls end short of Revit's at the low end of a sloped footing); follows RE-336 (#1388), RE-26 and RE-29 (join trims)
**Artefacts:** the BIM4LCA office as built and at building permit (Revit 2024, Revit 24.0.5's IFC4 exports, CC BY-SA 4.0, `research/bim4lca-measure.tsv`), Core Interior, RE1 Architecture, puzzbobb's 24 Revit 2025 models, Autodesk's sample projects and the held-out files, as Measure fetches them
**Probe, tools:** `probes/examples/probe_1067_native_dump.rs` (its `wall <id> step VWallDriver`, `step JoinEndGStep` and `step GLine` lines), `tools/re/wall_top_cover_vs_ifc.py`, `tools/re/wall_heights_vs_ifc.py`
**Runs:** to be filled in section 4.
**Status:** implemented; measurement in progress (section 4).

## 1. What the bytes say

The walls 2057668 and 2057670 as built (and those and 2058499 and 2058501 at permit) are four Basement walls on the sloped footings of RE-336. rvt-rs draws each 0.31 ft (0.09 to 0.10 ft at permit) shorter than Revit at the end by the low edge of the footing: its plan reaches y 75.7285 ft where Revit's reaches 75.4341 (75.4183 ft in rvt-rs's frame), at permit 76.0663 against 75.9784.

They are drawn from their record box with the joins' trim (`BodySource` `partition_element_record_join_trimmed`, RE-26), not from a centreline: each has an edited elevation profile, so its native record holds five or six `GLine`s and `native_walls` reads no curve for it (the passes that read the native curve, RE-222, RE-252 and RE-322, leave it out; 65 of the office's walls as built have more than one `GLine`).

The wall's line ends at y 75.0149 ft, the centreline of the Level_01 wall 1344124 (1.4272 ft thick, 183.07 to 195.87 ft). The trim solver takes the walls its record names that are perpendicular, whose centreline lies on the end, that span the wall's centreline and that overlap it in elevation by any amount: 1344124 overlaps the wall (174.21 to 183.73 ft) by 0.66 ft, and the end is cut half its thickness in, to its face at 75.7285 ft. But Revit's wall at its base is cut against the walls its end step names, `JoinEndGStep.m_geomJoined` of end 0: 1493966, a Basement wall, 169.62 to 182.09 ft, 1.5584 ft thick about y 74.6391, whose far face is y 75.4183 ft, inside which the line's end lies though not on its centreline (and 3490093 and the abutting 1347199). Revit's body steps up the end: the scorer's horizontal faces of 2057668 are a step of 0.1601 ft2 at 182.087 ft (the top of 1493966) between y 75.4341 and 75.6293, and a step of 0.0943 ft2 at 183.071 ft (the top of 3490093, the base of 1344124) between 75.6293 and 75.7443 (75.6135 is the face of 3490093, 75.7285 that of 1344124).

## 2. The rule

Where every candidate that has its centreline on an end lies above the wall's base (none of their elevation ranges contains the base), and exactly one other wall that the record names, perpendicular to the wall and spanning its centreline, holds the base and has the end inside its thickness without having its centreline on it, the end is cut at that wall's face on the wall's side: its centreline plus half its thickness less the end for the low end, and the reverse for the high end. It replaces the candidates' half thickness where it differs from it by more than 1e-3 ft. Where several walls hold the base, none does, the candidates include one that holds it, or no wall is named, the trim stays the candidates'.

The wall's body is one prism, so it reaches this face at every height: Revit's steps above 182.087 ft and 183.071 ft are not drawn.

An attempt on the native curves (the end's `m_geomJoined` in `attach_native_tee_reaches`) was dropped: the walls it was written for have no native curve, and on the one model it did move, Autodesk's Snowdon Towers Architectural, there is nothing to score it against.

## 3. Reproduce

To be filled.

## 4. Measured

To be filled from the Measure runs.
