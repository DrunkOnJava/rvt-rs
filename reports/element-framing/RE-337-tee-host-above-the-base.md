# RE-337: a T joint whose host lies above a wall's base stops the wall at the wall that holds its base

**Date:** 2026-11-04
**Issues:** #1396 (Basement walls end short of Revit's at the low end of a sloped footing); follows RE-336 (#1388), RE-222 (T joints), RE-70 (join lists)
**Artefacts:** the BIM4LCA office as built and at building permit (Revit 2024, Revit 24.0.5's IFC4 exports, CC BY-SA 4.0, `research/bim4lca-measure.tsv`), Core Interior, RE1 Architecture, puzzbobb's 23 Revit 2025 models, Autodesk's sample projects and the held-out files, as Measure fetches them
**Probe, tools:** `probes/examples/probe_1067_native_dump.rs` (its `wall <id> step VWallDriver` and `step JoinEndGStep` lines), `tools/re/wall_top_cover_vs_ifc.py`, `tools/re/wall_heights_vs_ifc.py`
**Runs:** to be filled in section 4.
**Status:** implemented; measurement in progress (section 4).

## 1. What the bytes say

The walls 2057668 and 2057670 as built (and those and 2058499 and 2058501 at permit) are four Basement walls on the sloped footings of RE-336. rvt-rs draws each 0.31 ft (0.09 to 0.10 ft at permit) shorter than Revit at the end by the low edge of the footing: rvt-rs's plan reaches y 75.7285 ft where Revit's reaches 75.4341 (75.4183 ft in rvt-rs's frame), at permit 76.0663 against 75.9784.

The wall's end there is a node of five walls. Its record says, in `VWallDriver.m_controlJoinsSet`, that the wall's end is joined to 1344124 by that wall's side (`m_joinedEnd` 3), and rvt-rs's T-joint rule (RE-222) stops the wall half the host's thickness short of the wall's line: 0.7136 ft from 75.0149, the face of 1344124 at 75.7285. But 1344124 is a Level_01 wall, 183.07 to 195.87 ft; the wall is 174.21 to 183.73 ft, so the host cuts the wall's top 0.66 ft only (0.07 of its height; at permit 1.64 ft, 0.19). The wall's end step, `JoinEndGStep` of end 0, names the walls its geometry is cut against, `m_geomJoined`: 1493966 (joined by its side) and 3490093 (by its side) and 1347199 (by its end). 1493966 is a Basement wall, 169.62 to 182.09 ft, whose far face is y 75.4183: the end of Revit's wall at its base.

Revit's body steps up the end: the scorer's horizontal faces of 2057668 are a step of 0.1601 ft2 at 182.087 ft (the top of 1493966) between y 75.4341 and 75.6293, and a step of 0.0943 ft2 at 183.071 ft between 75.6293 and 75.7443 (75.7285 is the face of 1344124, 75.6135 that of 3490093, 183.071 the top of 3490093 and the base of 1344124).

## 2. The rule

An end that is a T joint (RE-222, one host) stops at the face of the one wall the end's `JoinEndGStep.m_geomJoined` names, other than the host, that meets it with its side and holds the wall's base, when the host does not hold the wall's base. The wall holds the base when its record box's range, base to base plus `m_bboxHeight`, contains the base within 1e-3 ft. The face is the far face of that wall along the wall's line (`tee_face_reach`: its curve's place along the line, half its thickness, at a right angle), and it replaces the host's where it differs by more than 1e-3 ft. Where several others hold the base, none does, or the wall does not cross at a right angle, the host's reach stands.

The wall's body is one prism, so it reaches this face at every height: Revit's steps above 182.087 ft (and at 183.071 ft) are not drawn.

## 3. Reproduce

To be filled.

## 4. Measured

To be filled from the Measure runs.
