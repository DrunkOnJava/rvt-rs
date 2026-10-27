# RE-254 — A railing's Length

**Date:** 2026-10-27
**Issues:** #1016 (quantities), W7 of [`docs/release-1.0-criteria.md`](../../docs/release-1.0-criteria.md); follows RE-92 (stair runs) and RE-247 (a stair flight's `Length`)
**Artefacts:** the BIM4LCA office as built (Revit 2024, Revit 24.0.5's IFC4 export) and puzzbobb's R2025 13 (Rooms, Stairs; Revit 2025), as Measure fetches them
**Runs:** Measure [40455977588](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40455977588) and [40456875725](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40456875725) for §4; Measure [40446445002](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40446445002), [40447406827](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40447406827), [40448457476](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40448457476) and [40449134993](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40449134993) (`examples/probe_re254_railing_paths.rs` with `tools/re/railing_length_vs_ifc.py`)
**Result:** positive for where the path is, negative for the `Length`: a railing's path is the lines and arcs of the sketch its native record names, and their lengths add up to Revit's `Qto_RailingBaseQuantities.Length` only where the railing is level; on stairs and sloped floors Revit's is the path's length in 3D. Narrowed (added 2026-10-27, §4): on a stair railing whose path is one line along its stair's runs, that length follows from the stair's risers and treads, and is written; every other railing's is not yet.

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

## 4. Stair railings (added 2026-10-27)

`BaseRailing.m_stairsId` names the stair a stair railing belongs to; its `StairsElement` gives the stair's actual riser height and tread depth (`m_actualRiserHeight`, `m_actualTreadDepth`), and each `StairsRun` its stair (`m_stairsId`), its risers (`m_topRiserIndex` less `m_baseRiserIndex`) and, on its first `StairsTreadInfo`, the plan direction it climbs (`m_runDirProj`). Three rules for the 3D length, against Revit's `Length` (Measure [40455977588](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40455977588), the office at both stages, R2025 12 to 18 and the held-out models, none of which has a stair railing):

| rule | office as built (26 stair railings) | R2025 13 to 16 (8) | R2025 17, spiral (2) |
|---|---:|---:|---:|
| every line along a run climbs at the slope, the rest level | 2 | 6 (not 14) | 0 |
| the plan length plus, for each run of the stair, its risers less one times the tread depth climbing | 2 | 8 | 0 |
| a path of one line along one or more of the stair's runs: that line, as much of it as the longest of those runs climbs (its risers less one times the tread depth) climbing, the rest level | 4 of 4 | 8 of 8 | none (arcs) |

The same at permit. R2025 14 ends without a riser: its run's sketch spans the same 7.349 ft as R2025 13's, but its rail climbs over one tread less, as the run's risers less one say. On the office's 20 stair-well railings, whose paths have several lines, the second rule misses by 0.0656 ft whatever the slope, over on the inner railings and under on the outer ones: a plan offset of the rail at the landings that nothing read gives yet.

So a stair railing whose path is one line along its stair's runs carries the third rule's length as its `Qto_RailingBaseQuantities.Length` (`native_stairs::railing_lengths`, one native pass for the railings and one for their sketches, stairs and runs). Against `main` (Measure [40456875725](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40456875725), unfiltered): 4 more of the office's quantities equal at each stage and 2 on each of R2025 13 to 16, none different; no other export moves, and export times are unchanged.

## 5. The stair wells (added 2026-10-27, later)

The office's 20 stair-well railings belong to three-run stairs: on stair 2859203, for one, a run of 8 risers climbing +y, one of 11 climbing −x and one of 8 climbing −y, each tread 0.984 ft (`stairinfo`, `runinfo`; Measure [40470573973](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40470573973)). The inner railing's sketch (`line`) runs 6.890 ft along the first run, 0.361 and 0.180 ft round the corner, 9.843 ft along the second run, 0.180 and 0.361 ft round the next corner and 6.890 ft along the third: so the 9.843 ft side is a run, not a landing, and the climbing plan length is the runs' risers less one times the tread, 24 × 0.984 = 23.622 ft, on the inner and the outer railing alike, as §4's second rule takes it.

Revit's `Length` is that rule's length less 0.065617 ft on each of the 10 inner railings and more by the same on each of the 10 outer ones, to 1e-13 ft. 0.065617 ft is these railings' `BaseRailing.m_placementOffset` (20 mm; the office's railings have 0, −0.0509, 0.0656 or 0.0833 ft). `m_flipped` is false on all 41 of the office's railings and tells nothing. The sign is the railing's own geometry: the sense its path turns (±180° on every well railing) times the side of its first line the run along it lies on (Measure [40472254653](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40472254653), the run's footprint from its decoded sketch): plus where the run lies inside the turn, the outer railings, and minus where it lies outside, the inner ones, on all 20.

What the offset does is not understood. Offsetting the sketch away from the stair by the offset lengthens the outer path and shortens the inner one, as measured, but at the two 90° corners each well path has that would be four times the offset, not once. A straight path is not affected: the R2025 railings' 0.0833 ft and two office railings' 0.0656 ft leave their one-line `Length` exact (§4). On the office every cornered stair railing has exactly two 90° corners, so it cannot tell an effect per path from one per corner; the outer sketches' two landing legs also differ by half the offset (4.2971 and 4.2651 ft) where the inner ones are equal. A model with a stair railing of one corner, or of other angles, would tell. Until then the stair-well railings stay without a `Length`: a fitted correction would be right on all 20 office railings, but it rests on one model and no mechanism, and no held-out model has a cornered stair railing.

## 6. What stays open

Every other railing's `Length`: a stair railing over landings needs its rail's plan offset there; a railing on a sloped floor or roof its host's plane; a level railing something that tells it is level (the plan sum is Revit's on 4 of the office's 15 railings that belong to no stair, and nothing read tells those apart).

## 7. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=dc9ad4003c13bff6a41fbd462d37a2b71bd303de -f base=none -f nordic=true -f r2025=true \
  -f models="nordic-office-as-built r2025-13_Rooms_Stairs" -f scorers=none -f probe=probe_re254_railing_paths -f probe_scorer=railing_length_vs_ifc
gh workflow run measure.yml --ref main -f ref=a6a97dbb1158b245ed5a8563e5a586f2db264d1f -f base=main -f nordic=true -f r2025=true
gh workflow run measure.yml --ref main -f ref=a6a97dbb1158b245ed5a8563e5a586f2db264d1f -f base=none -f holdout=true -f nordic=true -f r2025=true \
  -f models="holdout-* nordic-office-* r2025-1[2-8]*" -f scorers=none -f probe=probe_re254_railing_paths -f probe_scorer=railing_length_vs_ifc
```
