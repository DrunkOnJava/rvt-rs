# RE-266 — A wall's Height where nothing joins to or cuts it

**Date:** 2026-10-29
**Issues:** #1016 (quantities), W7 of [`docs/release-1.0-criteria.md`](../../docs/release-1.0-criteria.md); follows RE-257 (a wall's `Height` is the span of Revit's body; rvt-rs's body is taller on 78 office walls, #1067) and RE-251; puzzbobb's measurement of the saved parameter -1001101 on #1016 (2026-10-27)
**Artefacts:** the BIM4LCA office as built and at building permit (Revit 2024, Revit 24.0.5's IFC4 exports, CC BY-SA 4.0, `research/bim4lca-measure.tsv`) and puzzbobb's R2025 models (Revit 2025, Revit 25.0.2's IFC4 exports, Apache-2.0, `research/r2025-models.tsv`, with `20_T_U_Roof` and the first export of `07_1Slope_ArrowDefined_2Cut50mmFascia` from #885), as Measure fetches them
**Runs:** Measure [40490349469](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40490349469) (RE-257's probe output, read again by record class) and [40977753002](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40977753002) (this change, 7a635f47, against main at 91076f0b: `examples/probe_re251_wall_numbers.rs` with `tools/re/wall_height_vs_qtos.py`, and every scorer on both sides)
**Result:** positive. On a wall whose native record holds no `JoinToRoofGStep`, no `CutPairGStep` and no `WallCutoutGStep`, the record box rvt-rs extrudes is exactly as tall as Revit's body, and Revit's export writes that as the wall's `Height` on every one: 81 office walls as built, 70 at permit, 93 R2025 walls, none different. rvt-rs now writes it there, and on no other wall.

## 1. The three steps

A wall's native `SWall` record lists the geometry steps Revit applies to its body. Three of them change it after it is extruded from its record box:

- `JoinToRoofGStep`: a roof or floor joined to the wall's top (`m_baseOrTop` 1) or base (0), RE-228;
- `CutPairGStep`: another element (`m_otherId`) cutting the wall or cut by it;
- `WallCutoutGStep`: the cutout of an instance (`m_instId`), a door, window, opening or other family, in this wall or one beside it.

RE-257 found the roof join alone on 77 of the 78 walls rvt-rs draws too tall, but also on 383 of the 501 it draws right, so it does not narrow the misses. Read the other way, it does: a wall without any of the three is never one of them.

## 2. Measured

`tools/re/wall_height_vs_qtos.py` sets the depth of the extrusion rvt-rs draws each wall as (`B.depth`) against Revit's `Qto_WallBaseQuantities.Height`, in feet, to a relative 1e-6, and now counts the walls whose record holds none of the three steps apart:

| | as built | at permit | R2025 |
|---|---:|---:|---:|
| walls rvt-rs draws as an extrusion | 607 | 589 | 138 |
| depth is Revit's `Height` | 501 | 481 | 138 |
| record holds none of the three steps | 81 | 70 | 93 |
| ... depth is Revit's `Height` | 81 | 70 | 93 |
| ... different | 0 | 0 | 0 |
| ... Revit writes no `Height` | 0 | 0 | 0 |

On all 244 of those walls the saved parameter -1001101 equals the depth too.

## 3. Why a cutout leaves a wall out

Without the cutout step, the rule takes 89 walls as built: 87 carry Revit's `Height` and 2 have none.

- 1494012 (`Basic Wall:KS-2`) is cut by 3388647, a fire hose cabinet Revit exports as an `IfcFireSuppressionTerminal` with no `IfcOpeningElement`;
- 1619525 (`Basic Wall:VS-2`) is cut by the door 3064459, which stands in the wall 3064420 and has its opening there; this wall's `WallCutoutGStep` has `m_bCutSomething` 0.

Revit draws both as a `Tessellation`, but so does 1351840, cut by its own opening 3391020, which has a `Height`, and 1728013 has a cutout with `m_bCutSomething` 0 and a `Height` too. Nothing read tells the two apart from the 6 walls with a cutout that get one, so every wall with a cutout is left out: 6 office walls as built, 2 at permit and 12 R2025 walls that would have been right.

## 4. What -1001101 is

RE-251 and RE-257 called the saved parameter -1001101 the wall's Unconnected Height. puzzbobb measured on #1016 that it is the wall's height as its base and top constraints give it, before any attachment: wall 2789 of `04_Gable` holds 13.123 ft in -1001105 and 20.413 ft in -1001101, its top being at a level, and with a Top Offset of -7.290 ft in another model -1001101 is 13.123. Where the record holds no `JoinToRoofGStep`, -1001101 is Revit's `Height` on 94 of 94 R2025 walls and 115 of 119 office walls as built; the other four (1347198, 1347200, 1347201 and 1795193) each have a `CutPairGStep`, so this rule leaves them out. On the first three rvt-rs's depth is Revit's `Height`, and on 1795193 neither is.

## 5. What stays unwritten

Every wall with one of the three steps: 420 of the office's 501 right depths as built, 411 of 481 at permit, 45 of the 138 R2025 walls. Where the top is joined, puzzbobb found Revit's `Height` is the attached wall's (higher on the R2025 roofs, lower on 56 of 57 office walls), so it waits on the body Revit cuts (#1067).

## 6. The export

Against main, every export with a wall the rule takes moves and nothing else does: all 1,179 other scorer outputs are byte-identical. The quantity census (`tools/re/qtos_vs_ifc.py`):

| | equal before | equal after | different | values Revit does not write |
|---|---:|---:|---:|---:|
| BIM4LCA office as built | 21,065 | 21,146 | 1 | 0 |
| at permit | 20,980 | 21,050 | 1 | 0 |
| R2025 models | 318 | 411 | 0 | 0 |

"Different" and "values Revit does not write" are the same before and after.

Core Interior's and RE1 Architecture's exports were made with base quantities off (RE-242): rvt-rs writes 238 and 4 more there, counted apart.

## 7. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=7a635f477d08116c8f4d6293ca1a93660347d7cd -f base=main -f nordic=true -f r2025=true \
  -f probe=probe_re251_wall_numbers -f probe_scorer=wall_height_vs_qtos
```
