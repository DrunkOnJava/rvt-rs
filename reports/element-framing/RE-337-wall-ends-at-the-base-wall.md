# RE-337: a wall with an edited profile that stops short of the wall beneath it is drawn to that wall's face

**Date:** 2026-11-04
**Issues:** #1396 (Basement walls end short of Revit's at the low end of a sloped footing); follows RE-336 (#1388), RE-26 and RE-29 (join trims)
**Artefacts:** the BIM4LCA office as built and at building permit (Revit 2024, Revit 24.0.5's IFC4 exports, CC BY-SA 4.0, `research/bim4lca-measure.tsv`), Core Interior, RE1 Architecture, puzzbobb's 24 Revit 2025 models, Autodesk's sample projects and the held-out files, as Measure fetches them
**Probe, tools:** `probes/examples/probe_1067_native_dump.rs` (its `wall <id> step VWallDriver`, `step JoinEndGStep` and `step GLine` lines), `tools/re/wall_top_cover_vs_ifc.py`, `tools/re/wall_heights_vs_ifc.py`
**Runs:** to be filled in section 5.
**Status:** implemented; measurement in progress (section 5).

## 1. What the bytes say

The walls 2057668 and 2057670 as built (and those and 2058499 and 2058501 at permit) are four Basement walls on the sloped footings of RE-336. rvt-rs draws each 0.31 ft (0.09 to 0.10 ft at permit) shorter than Revit at the end by the low edge of the footing: its plan reaches y 75.7285 ft where Revit's reaches 75.4341 (75.4183 ft in rvt-rs's frame), at permit 76.0663 against 75.9784.

Each is drawn from its record box with the joins' trim (RE-26), or, at permit, from its centreline with the reach its join lists give (RE-70): the same end by both. Each has an edited elevation profile, so its native record holds five or six `GLine`s and `native_walls` reads no curve for it, and the passes that read the native curve (RE-222, RE-252, RE-322) leave it out (65 of the office's walls as built have more than one `GLine`).

The wall's line ends at y 75.0149 ft, the centreline of the Level_01 wall 1344124 (1.4272 ft thick, 183.07 to 195.87 ft). The trim solver takes the walls its record names that are perpendicular, whose centreline lies on the end, that span the wall's centreline and that overlap it in elevation by any amount: 1344124 overlaps the wall (174.21 to 183.73 ft) by 0.66 ft, and the end is cut half its thickness in, to its face at 75.7285 ft. But Revit's wall at its base is cut against the walls its end step names, `JoinEndGStep.m_geomJoined` of end 0: 1493966, a Basement wall, 169.62 to 182.09 ft, 1.5584 ft thick about y 74.6391, whose far face is y 75.4183 ft, inside which the line's end lies though not on its centreline (and 3490093 and the abutting 1347199). Revit's body steps up the end: the scorer's horizontal faces of 2057668 are a step of 0.1601 ft2 at 182.087 ft (the top of 1493966) between y 75.4341 and 75.6293, and a step of 0.0943 ft2 at 183.071 ft (the top of 3490093, the base of 1344124) between 75.6293 and 75.7443 (75.6135 is the face of 3490093, 75.7285 that of 1344124).

## 2. What was tried and dropped

- The end's walls from the native record (`m_geomJoined`, in the T-joint pass of RE-222): the four walls have no native curve (above), and on Autodesk's Snowdon Towers Architectural, the one model it moved, there is nothing to score it against.
- The record solver's candidates (RE-26): taking the face of the one named wall that holds the base where every wall on the end lies above the base moved the four walls as built, and not at permit, where the centreline body keeps its reach. It also moved 25 of Autodesk's sample projects (every `rac_basic` save from 2016 to 2027, the `rme` samples and Snowdon Towers Architectural), one held-out model (`bimspeed-gdynia-2021`) and the office's wall 1347199, none of them with a Revit export to score it against. The condition holds on many more ends than the four, so it was dropped.

## 3. The rule

What the four walls share that the walls beside them do not is the edited elevation profile (a `WallProfileOpening` voids each). For such a wall that is a rectangle along a model axis, an end is drawn to the face of the one other rectangle along the perpendicular axis that holds the wall's base, spans its width and stands beyond the end by no more than its own thickness, where no such wall touches the end already. The wall's body is one prism, so it reaches that face at every height: Revit's steps above 182.087 ft and 183.071 ft are not drawn. It runs on the built bodies, before the tops and bases are cut, and the wall carries `BodyEndSource` `wall_end_extended_to_base_wall` and `EndCoverElementId`.

## 4. Reproduce

To be filled.

## 5. Measured

To be filled from the Measure runs.
