# RE-297 — A ceiling's GrossArea, and the Width of ceilings in pieces and of walls written as coverings

**Date:** 2026-10-30
**Issues:** #1016 (quantities), W7 of [`docs/release-1.0-criteria.md`](../../docs/release-1.0-criteria.md); follows RE-250 (a ceiling's Width), RE-295 (the cuts a floor's record shows) and #331 (a sketch of separate loops is several elements)
**Artefacts:** the BIM4LCA office as built and at building permit (Revit 2024, Revit 24.0.5's IFC4 exports), RE1 Architecture (whose export has no base quantities, RE-242) and puzzbobb's R2025 models (no coverings), as Measure fetches them
**Runs:** Measure [41138942086](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41138942086) (`probes/examples/probe_re297_covering_records.rs`), [41140001298](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41140001298) (the quantity census) and [41140501767](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41140501767) (this change against RE-295's, unfiltered)
**Result:** positive, narrowed: a compound ceiling drawn as one extrusion of its sketch's outline has Revit's `Qto_CoveringBaseQuantities.GrossArea` as that outline's area, its holes taken off, unless its native record holds a `CutPairGStep`, which marks the ceilings Revit cuts; a ceiling of several pieces and a wall Revit writes as a covering have its `Width`. All written, none different. The area of a ceiling in pieces or one a join cuts, and the covering Revit writes for the office's ventilation units, stay unwritten.

## 1. A ceiling's GrossArea

RE-250 found a ceiling's `GrossArea` equal to the area of the profile its sketch gives on 57 of the 59 ceilings of the office as built that it could compare, and 0.7 to 1.1 percent larger on 2 (9 at building permit), and left it out for want of a field that told them apart. The record tells them. `probe_re297_covering_records` lists the steps of every covering's native record:

| ceilings drawn as one extrusion of their sketch | as built | at permit |
|---|---:|---:|
| record holds no `CutPairGStep`: profile area is `GrossArea` | 57 of 57 | 45 of 45 |
| record holds a `CutPairGStep` (and a `CleanPairGStep`): profile area is `GrossArea` | 0 of 2 | 1 of 10 |

Every ceiling Revit cuts holds the join step; rvt-rs's sketch outline is the ceiling before the cut. Revit's value for those is the area of what is left, 8.8207 m2 against the sketch's 8.9203 m2 on ceiling 2095292. The same step marks a floor Revit cuts (RE-295), so the reader that finds it for floors reads ceilings too: `attach_slab_uncut`.

## 2. A ceiling of several pieces

A sketch of separate loops is several elements sharing one `Tag` (#331): 19 of the office's compound ceilings as built and 20 at permit, which RE-250 left out. Revit's `Width` is the type's thickness on all of them, as on the others, and rvt-rs writes it where every piece is as deep as the type's layers are thick. Their `GrossArea` is the area of one piece, and which one is Revit's: of the pieces of ceiling 2095436 (5.520, 3.910 and 3.244 m2) it is the last and smallest, of those of 2095364 (1.455, 4.639 and 1.467 m2) the middle and largest. Nothing in the record says, and it stays unwritten.

## 3. A wall written as a covering

The 21 walls of the type `AK-1.otsa` (a `Basic Wall`) that Revit's export writes as `IfcCovering` get its `Qto_CoveringBaseQuantities.Width` only: the wall's type thickness, 0.052 m on all 21, which rvt-rs knows from the wall's type as it does for every wall's own `Width`. Revit gives them no `GrossArea`.

## 4. What rvt-rs does

`attach_slab_uncut` marks floors and ceilings whose native record holds neither a `CutPairGStep` nor a `WallCutoutGStep`. `add_ceiling_quantities` writes `GrossArea` on a marked ceiling drawn as one extrusion of its sketch's outline, and `Width` where the layers' total is the depth of the body and of every piece. `add_wall_covering_width` writes a wall-written covering's `Width`.

## 5. Measured

This change against RE-295's (Measure 41140501767, unfiltered, `-f nordic=true -f r2025=true`, every scorer):

| model | `GrossArea` equal | `Width` equal | different | Revit does not write |
|---|---:|---:|---:|---:|
| office as built | 0 → 57 of 90 | 59 → 99 of 111 | 0 | 0 |
| office at permit | 0 → 45 of 75 | 55 → 75 of 75 | 0 | 0 |

The office as built goes from 28,065 to 28,162 of the 39,388 quantities equal and at permit from 28,011 to 28,076 of 39,679, with the mullion's one difference and none Revit's export leaves out. RE1 Architecture's 6 ceilings carry a `GrossArea` too, counted apart since that export has no base quantities (86 → 92). No R2025 export moves (they have no coverings), nor Core Interior's, and the only other output that moves is `diagnostics.json`'s `total_entities` (+87 as built). Every other scorer's output (properties, geometry, validation) is as RE-295's. The run measures the commit before the cut marker was made to read ceilings and roofs in the pass RE-295 adds; Measure [41141309271](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41141309271) at the commit after it gives the same census.

## 6. What stays open

- The 12 roofing coverings of the office as built (the ventilation units of the type `Ilmanvaihtokoneet`, specialty equipment that Revit's export writes as `IfcCovering` `ROOFING`): their `GrossArea` is the area of rvt-rs's box and their `Width` its X side on all 12, and none is written. The box is the instance's record box and the family's footprint is not read; 11 of the 12 are squares, so which side Revit's `Width` is, a side along the axis of the plane Revit extrudes from, is tested by one unit (4.4 by 2.3 m, `Width` 4.4 m). The permit has none.
- The `GrossArea` of the 19 ceilings in pieces and of the 2 cut ones as built (20 and 10 at permit): 33 in all as built.
- Covering `GrossArea` of any file whose native record rvt-rs does not read.

## 7. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=8ced5de49353bbf6afd3fbb27fd388d12ee3c0d8 -f base=main -f nordic=true -f r2025=true
gh workflow run measure.yml --ref main -f ref=8090d379266f5e7ec7ddf508e9625b9b6804a85f -f base=none -f nordic=true -f models="nordic-office-*" -f scorers=none -f probe=probe_re297_covering_records -f probe_scorer=-
```
