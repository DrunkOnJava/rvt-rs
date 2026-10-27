# RE-254 — A railing's Length

**Date:** 2026-10-27
**Issues:** #1016 (quantities), W7 of [`docs/release-1.0-criteria.md`](../../docs/release-1.0-criteria.md); follows RE-92 (stair runs) and RE-247 (a stair flight's `Length`)
**Artefacts:** the BIM4LCA office as built (Revit 2024, Revit 24.0.5's IFC4 export) and puzzbobb's R2025 13 (Rooms, Stairs; Revit 2025), as Measure fetches them
**Runs:** Measure [40446445002](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40446445002), [40447406827](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40447406827), [40448457476](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40448457476) and [40449134993](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40449134993) (`examples/probe_re254_railing_paths.rs` with `tools/re/railing_length_vs_ifc.py`)
**Result:** positive for where the path is, negative for the `Length`: a railing's path is the lines and arcs of the sketch its native record names, and their lengths add up to Revit's `Qto_RailingBaseQuantities.Length` only where the railing is level; on stairs and sloped floors Revit's is the path's length in 3D, which nothing read gives yet. Nothing is written.

## 1. Which length Revit writes

revit-ifc's `LengthCalculator` (CLEANROOM.md source 5) takes a railing's `Length` from its `CURVE_ELEM_LENGTH` parameter. Revit's export gives one to each of the office's 41 railings and to both of R2025 13's.

## 2. What the records hold

- A railing's native record holds one `BaseRailing` object and its parameter sets, and no curve. Its only stored double parameter is -1008621, 0 on every railing read, so the length is not stored there.
- `BaseRailing.m_sketchId` names the railing's sketch, a `VarSketch` record whose `GLine` and `GArc` objects are the path in plan (`CurveElemData` and `VarSketchLineSegObj` per curve, and `RailingSketchCurveData` on 7 of the office's railings, `StairsRailingCurveData` on R2025 13's). `BaseRailing.m_stairsId` names the stair a stair railing belongs to.
- The sketch is level: the office's first sketch read holds four collinear lines at one height (182.087 ft) and R2025 13's one line at height 0.

## 3. Measured

The sums of the sketch's line spans (`m_endParams`) and arc lengths (radius times span), against Revit's `Length` (within 1e-4 ft):

| office as built, railings | Revit's `Length` less the sum |
|---|---|
| 4 (1469144, 1470882, 2903645, 2903804) | 0 |
| 20 in the stair wells, whose sketches are 24.706 or 41.076 ft in plan in every one | 2.40 to 3.22 ft, by storey: Revit's 27.110 to 27.790 and 43.612 to 44.292 ft |
| 5 with `RailingSketchCurveData` (1749890, 2598350, 2598353, 2598405, 2902063) | 0.32 to 4.41 ft |
| 12 others, 3 of them with arcs | 0.0106 to 2.59 ft |

On R2025 13 each railing's sketch is one line of 7.349 ft, its flight's `Length` (RE-247), and Revit's `Length` is 8.651 ft: the line along a slope rising 4.565 ft over it, a 0.621 rise per foot.

Revit's value is never shorter than the sum, and the stair wells' railings, whose paths in plan are the same, get different lengths, so it is the path's length in 3D, along the stair or the sloped floor or roof the railing stands on (on 1467318, 0.0106 ft over 19.52 ft, a rise of about 0.64 ft, a fall of about 3%). Nothing read says which railings are level: the four that are include two with a `PointOnFaceDriver` and `RailingSketchCurveData` and two with neither, and 32 of the 37 that are not have neither.

## 4. What stays open

A railing's `Length` needs its path's height along it: over a stair flight the flight's slope (its riser height over its tread depth, RE-92 and native stairs), on a sloped floor or roof that host's plane. Until that is read, no railing gets a `Length`: Revit's export writes one on every railing, and on 37 of the office's 41 it is not the plan sum, with nothing read telling the other 4 apart.

## 5. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=dc9ad4003c13bff6a41fbd462d37a2b71bd303de -f base=none -f nordic=true -f r2025=true \
  -f models="nordic-office-as-built r2025-13_Rooms_Stairs" -f scorers=none -f probe=probe_re254_railing_paths -f probe_scorer=railing_length_vs_ifc
```
