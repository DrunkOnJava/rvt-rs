# RE-252 — A wall's Length

**Date:** 2026-10-26
**Issues:** #1046 (quantities Revit computes from its joined geometry, step 1: walls), #1016 (quantities), W7 of [`docs/release-1.0-criteria.md`](../../docs/release-1.0-criteria.md); follows RE-70, RE-71, RE-73 and RE-222 (wall join reaches) and RE-251 (a wall's record keeps no Length)
**Artefacts:** the BIM4LCA office as built and at building permit (Revit 2024, Revit 24.0.5's IFC4 exports) and puzzbobb's R2025 models (Revit 2025), as Measure fetches them; held out, the flowbim.ee house (Revit 2026, Revit 26.4's IFC4X3 export) and 255ribeiro's `modelo_bim` and `teste_export_2025` (`research/holdout-models.tsv`, fetched for the run, only their counts uploaded)
**Runs:** Measure (`examples/probe_re252_wall_ends.rs` with `tools/re/wall_length_vs_ends.py`) [40419110452](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40419110452) to [40429593111](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40429593111); the change against `main`, [40430218765](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40430218765) (filtered) and [40434067592](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40434067592) (unfiltered, every scorer); the held-out models, [40433993376](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40433993376)
**Result:** positive, narrowed: Revit's export writes a wall's `Length` as either its curve's length or the length of that curve cut back at its joins, by how it draws the wall; rvt-rs reads where the cut-back curve ends and writes the `Length` only where the two lengths are the same, which is then Revit's on every wall.

## 1. Which length Revit writes

revit-ifc (CLEANROOM.md source 5) has two sources for a wall's `Qto_WallBaseQuantities.Length`: `PropertyUtil.CreateWallBaseQuantities` writes the length of the curve `WallExporter` trims at the wall's joins (`ExporterIFCUtils.GetWallTrimmedCurve`), which is also the wall's exported `Axis`, and falls back on the location curve's length, as `LengthCalculator` takes it. Each office wall carries one `Qto_WallBaseQuantities` set (607 of 607 as built, 589 of 589 at permit), and its `Length` is the length of either the wall's native curve (below) or its exported `Axis`, never anything else:

| office as built | `Length` = curve | = `Axis` | = both |
|---|---:|---:|---:|
| walls drawn as an extrusion (`IfcExtrudedAreaSolid`) | 195 | 15 | 66 |
| walls drawn as a mesh (`IfcPolygonalFaceSet`, one with a triangulated part) | 96 | 171 | 64 |

At permit the split is 217 and 14 against 177 and 52, with 129 walls where the two agree. Mostly the curve on an extrusion and the trimmed curve on a mesh, but not always, and nothing in the model says which way the export goes: the maintainer chose (2026-10-26) to treat the switch as an inconsistency of Revit's export and to write a wall's `Length` only where both lengths are the same, computing the trimmed curve from the joins rather than limiting it to walls with no joins.

## 2. What the record holds

A wall's native `SWall` record (RE-222):

- `VWallDriver.m_pCrv` points at the record's `GLine`, the wall's curve: its `m_endParams` end on the middle plane of a wall joined there, on every joined end of the office's 607 walls.
- Its upright `Plane` objects along the curve are its faces (and its location planes): the outermost two are where the wall's sides are, also on a wall whose curve is not its centreline.
- `VWallDriver.m_controlJoinsSet` names the walls joined at each end (`m_index`), each with the end it joins by (2 or 3 its middle, a T joint), and `CDJoinInfo` names the walls whose end meets this wall's middle; `CDJoinInfo` alone misses most of the joins at a wall's own ends.
- `JoinEndGStep.m_exposedFaceMap` names a face of another wall at an end that runs through to that wall's far face: on the office as built, every one of the 231 such ends of the exported `Axis` has one, and 623 of the 631 that stop at a joined wall's near face have none.
- `CleanPairGStep`, `CutPairGStep` (`m_otherId`) and `WallEmbedWallGStep` (`m_instId`) name the other elements cutting the wall; where one is not a wall (a column, a curtain wall), the `Axis` ends where that element cuts it, which the joins do not give.

## 3. The rule

`native_walls::trimmed_ends` reads each end of the cut-back curve twice:

1. from the record: the curve's own end where no wall is joined there; the far face of the wall the end's exposed face map names; or else the nearest face of the walls joined there;
2. from the partition reader's join reach (RE-70, RE-73, RE-222; the largest of its layers' reaches where they end apart, RE-71).

An end is taken where the two agree within 1e-4 ft, or where no wall is joined, no reach is given and nothing cuts the wall. A wall none of whose join step partners is anything but a wall, both of whose ends are taken and whose cut-back curve is as long as its curve within 1e-4 ft gets that length as its `Length` (`partition_schema_mvp::WALL_LENGTH_FIELD`, written by `add_wall_quantities`).

Alone, each reading is right on most ends but not all, over the office and the R2025 models: the record's on 1,783 of 2,375 joined ends, the reaches on 1,108 of 1,186 single-layer ends and 667 of 705 layered ones. Where the two agree they are right on 1,458 of 1,550 ends; 83 of the 92 others belong to walls an element other than a wall cuts, and none of the 9 left gives a wall a wrong `Length`. A rule taking the GLine's length on walls with no joined wall at all is right too (17 of 17 office walls as built), but covers few.

## 4. Measured

This change against `main`, every scorer (run 40434067592, `-f nordic=true -f r2025=true`):

| model | `Length` equal | different | missing | Revit does not write |
|---|---:|---:|---:|---:|
| office as built | 88 of 607 | 0 | 519 | 0 |
| office at permit | 81 of 589 | 0 | 508 | 0 |
| R2025 models (the 21 with walls and an export) | 46 of 117 | 0 | 71 | 0 |

Held out (run 40433993376, `-f holdout=true`, only counts uploaded), on models the rule was not derived on: the flowbim.ee house's `Length` is equal on 5 of its 59 walls, none different; neither 255ribeiro project gets one (4 and 3 walls). On those runs the probe's own reading of the rule agrees: 5 walls on the house, each Revit's.

The office's quantities equal go from 20,247 to 20,335 as built (20,151 to 20,232 at permit). Core Interior's and RE1 Architecture's exports were made with base quantities off (RE-242), so the 38 and 1 wall `Length`s rvt-rs now writes on them are counted apart; Core Interior's witness observations gain those 38 `IfcQuantityLength` (both verdicts PASS, unchanged). Every other scorer's output is as on `main`.

## 5. Negative results

- The wall's location line (its centreline moved by `SWall.m_keyRefOffset` along its exterior, by `m_wallKeyRef`) met with the joined walls' gives Revit's `Length` on 422 of 602 office walls: Revit does not measure it to where location lines cross.
- Sums of one term per end (none, or plus or minus half or the whole of a joined wall's thickness, or half the wall's own) give it on 549 of 602 walls, but take a different term on walls of the same join, so they do not say which.
- A wall's record and saved parameters hold no `Length` other than the curve's (RE-251): the exporter computes the trimmed curve.

## 6. What stays open

A wall whose cut-back curve is not as long as its curve, or whose ends the readings do not agree on, has no `Length`: Revit's export gives one of two values there and nothing in the model says which. Its `GrossFootprintArea` (`Length` × `Width` on all 279 walls Revit writes it on) and its side areas and volumes (`GrossSideArea` = `Length` × `Height` on only 206 of 579) are #1046's next steps. Revit writes the footprint only where its exporter draws the wall as an extrusion (revit-ifc's `TryToCreateAsExtrusion` gives the profile's area), and of the 88 walls that now get a `Length` as built, 45 are extrusions and 43 meshes; no pair of the record's join step classes and join kinds tells that choice (the best, a layered join or else no `JoinToRoofGStep`, 494 of 607 as built and 466 of 589 at permit), so it stays out.

## 7. The walls whose two lengths differ (added 2026-10-26)

The maintainer's choice counts a wall's `Length` apart where its two possible values differ, rather than as missing. Revit's export draws the cut-back curve as the wall's `Axis` but does not carry the curve's own length, so rvt-rs's export diagnostics list it (`decoded.wall_curve_lengths_feet`, by ElementId; the maintainer chose on 2026-10-27 to keep it out of the IFC file, which would carry a value only the census reads), and `tools/re/qtos_vs_ifc.py` counts apart a missing `Length` where that and the `Axis`'s length differ by more than 1e-4 ft. Half of that comparison is rvt-rs's reading, which Revit's export confirms only where it wrote the curve's length; so the census counts walls apart only on a model where Revit's `Length` is either rvt-rs's curve's or its own `Axis`'s on every wall, none neither. On the office as built it is the curve's on 407 walls and the `Axis`'s on 184, neither on 0, and 461 walls are counted apart, leaving 146 to score: 88 equal, 58 missing (whose two lengths agree but whose ends the readings do not). At permit: 329, 181 and 0; 388 apart; 81 equal, 120 missing (Measure 40445008095). On the 21 R2025 models with walls and an export, 71 of the 117 walls are counted apart and the other 46 are equal, none missing (Measure [40448077811](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40448077811), unfiltered against `main`, where every other scorer's output is as on `main`). No export changes: every model's IFC is byte-identical to `main`'s, the curve lengths being in the diagnostics only.

## 8. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=f49308e8ca7ccf3dc6aa4f2822a23323f0c3b874 -f base=main -f nordic=true -f r2025=true
gh workflow run measure.yml --ref main -f ref=f49308e8ca7ccf3dc6aa4f2822a23323f0c3b874 -f base=none -f holdout=true \
  -f models="holdout-*" -f scorers=qtos_vs_ifc -f probe=probe_re252_wall_ends -f probe_scorer=wall_length_vs_ends
gh workflow run measure.yml --ref main -f ref=f49308e8ca7ccf3dc6aa4f2822a23323f0c3b874 -f base=none -f nordic=true -f r2025=true \
  -f models="nordic-office-* r2025-*" -f scorers=none -f probe=probe_re252_wall_ends -f probe_scorer=wall_length_vs_ends
```
