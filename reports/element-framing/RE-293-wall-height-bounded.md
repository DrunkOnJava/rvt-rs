# RE-293 — A roof-joined wall's Height

**Date:** 2026-10-31
**Issues:** #1046 (quantities Revit computes from its joined geometry, step 1: walls), #1016 (quantities), W7 of [`docs/release-1.0-criteria.md`](../../docs/release-1.0-criteria.md); follows RE-266 (a wall's `Height` where nothing joins to or cuts it), RE-267 (the top cut at a covering floor), RE-270 (a wall's side areas and volumes) and RE-290 (the cut-back centreline that confirms a roof-joined wall's length)
**Artefacts:** the BIM4LCA office as built and at building permit (Revit 2024, Revit 24.0.5's IFC4 exports, CC BY-SA 4.0, `research/bim4lca-measure.tsv`) and puzzbobb's R2025 models (Revit 2025, Apache-2.0, `research/r2025-models.tsv`), as Measure fetches them; the held-out models of `research/holdout-models.tsv` (counts only)
**Scorer:** `tools/re/wall_quantities_vs_ifc.py` (RE-270)
**Runs:** Measure [41136551451](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41136551451) (`main` at f73908bf: the scorer, which lists Revit's and rvt-rs's `Height` of each wall), [41178688180](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41178688180) (this change at 9c39b0c3 against RE-290's head ada5ca73, every model and scorer) [41180588895](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41180588895) (this change at 6317d42f against RE-290's head a07a850d, every model and scorer, with the held-out models: what `Height` alone moves) and [41183229365](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41183229365) (this change at f5d4ab5e, RE-290 and `main` merged in, against `main` at 9066ded5, every model and scorer, with the held-out models)
**Result:** positive. Revit's export writes a wall's `Height` on exactly the walls it writes `GrossSideArea`, `NetSideArea`, `GrossVolume` and `NetVolume` on (RE-270 §1), and on every wall of the office where those are Revit's, rvt-rs's extrusion is as tall as Revit's solid: 117 of 117 as built and 128 of 128 at permit, none different. The pass that gives a roof-joined wall the four (RE-270's second rule, RE-290) now gives it `Height` too: 47 more office walls as built and 58 at building permit, Revit's on every one, none different and none Revit's export leaves out.

## 1. Why the four and `Height` go together

revit-ifc's `PropertyUtil.CreateWallBaseQuantities` (CLEANROOM.md source 5; read at the tags `IFC_v24.1.0` and `IFC_v24.2.0`) writes `Height` when its caller hands it a positive depth, and `GrossSideArea`, `NetSideArea` and `NetVolume` out of the same call from the wall's solids; `GrossVolume` needs the depth too. On the office Revit writes `Height` on 579 of 607 walls as built and 565 of 589 at permit, and the four on the same walls (the others carry only a `NetVolume`, RE-270 §1).

RE-266 writes `Height` where the wall's record names nothing joined to or cutting it, because only there is the record box as tall as Revit's body. RE-270's second rule writes the four on a roof-joined wall only after its joins are shown to leave the prism whole (every element they name is a slab, roof or footing lying wholly above its top or below its base, RE-267's top cut applied first) and its length is confirmed (RE-290). A wall that passes that is a rectangular prism: its extrusion's height is the vertical span of Revit's solid.

## 2. Measured against Revit's walls

`tools/re/wall_quantities_vs_ifc.py` lists, per wall, Revit's `Height` and the height of rvt-rs's body (`U ... H=`). On the record-box rectangles whose record holds no step but the harmless ones, a roof or floor join and a clean join, where `GrossSideArea`, `NetSideArea` and `NetVolume` of rvt-rs's body are Revit's (the walls the rules take, and a few their other conditions leave out), run 41136551451:

| | as built | at permit |
|---|---:|---:|
| walls with no roof or floor join (RE-266 writes `Height`) | 49 | 48 |
| ... rvt-rs's body is as tall as Revit's `Height` | 49 | 48 |
| roof-joined walls | 68 | 80 |
| ... rvt-rs's body is as tall as Revit's `Height` | 68 | 80 |

Walls were matched by `Tag`, heights compared to a relative 1e-5.

## 3. The rule

`export_content::wall_areas::add_bounded_wall_quantities` writes `Height`, the extrusion's height, beside the four on each wall it vouches for. It adds no condition: a wall is taken exactly as RE-270 and RE-290 say. A wall Revit writes only a `NetVolume` on has no `Height` in Revit's export, and the rule takes none of them (the measured censuses below count none).

## 4. Measured

This change at f5d4ab5e against `main` at 9066ded5, every model and every scorer, with the held-out models (run [41183229365](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41183229365), `-f base=main -f nordic=true -f r2025=true -f holdout=true`). The head holds RE-290 (#1205), which this change is stacked on, so the census moves by both; run [41180588895](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41180588895) put this change's head against RE-290's own (`-f base=a07a850d...`) and shows what `Height` alone moves. Only `ifc.sha256`, `qtos_vs_ifc` and Core Interior's witness observations move on the models Revit's quantities are scored on; the other 1,467 outputs are byte-identical to the base's. The base is a newer `main` than the head by one landing (RE-283's wall parameters, #1182), which is why Einhoven's `diagnostics.json` and modelo_bim's property set census differ in that run and not in the quantities.

| | office as built | at permit |
|---|---:|---:|
| `Height` equal, before | 81 of 579 | 70 of 565 |
| ... after | 128 of 579 | 128 of 565 |
| ... of which this change | 47 | 58 |
| different, before and after | 0 | 0 |
| written that Revit's export omits, before and after | 0 | 0 |
| quantities equal, on `main` | 28,162 | 28,076 |
| ... with RE-290 | 28,278 | 28,220 |
| ... with RE-290 and this change | 28,325 | 28,278 |
| different, before and after | 1 (the mullion) | 1 |

Against RE-290's head alone (run 41180588895) the office goes from 28,181 to 28,228 equal as built and from 28,155 to 28,213 at permit, the same 47 and 58. `GrossSideArea` and the other three quantities of the walls are RE-290's (96 of 579 as built and 106 of 565 at permit). The R2025 models stay at 775 of 1,515 quantities (their `Height` 93 of 138), RE1 Mechanical and Plumbing at 31 of 31 and 63 of 63. Core Interior's and RE1 Architecture's exports were made with base quantities off (RE-242): the 7 `Height` values rvt-rs now writes on Core Interior's walls are counted apart (2,268 on `main`, 2,292 with RE-290, 2,299 with this change), RE1 Architecture's 92 are as they were, and Core Interior's witness observations gain the 7 `IFCQUANTITYLENGTH`.

Held-out models (the same run, scrubbed counts): the AA house (846 quantities, 127 equal, 0 different, 719 missing), modelo_bim (220, 7, 0, 213) and teste_export_2025 (44, 10, 0, 34) have the quantity counts of `main`'s on all three, so the rule writes no `Height` on them. Of their quantities that rvt-rs writes and Revit's export does not, none is a `Height`: the 2 on the AA house (the slab `Width` of RE-295) and the 3 on teste_export_2025 (the covering `Width` of RE-250 and RE-297) are on `main` already.

## 5. What stays unwritten

`Height` on every wall the second rule does not take. Where a wall's top is joined to a roof, Revit's `Height` is the attached wall's (higher on the R2025 roofs, lower on 56 of 57 office walls, RE-266 §5), so it waits on the body Revit cuts (#1067) except on the walls a floor leaves whole, which this pass covers.

## 6. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=f5d4ab5e423ecb20f5c6a507195162764fa88f53 -f base=main -f nordic=true -f r2025=true -f holdout=true
gh workflow run measure.yml --ref main -f ref=6317d42fb09a3f13b249245aa0356783441535cd -f base=a07a850d930332580e239585b49795b42bc5932a -f nordic=true -f r2025=true -f holdout=true
```

The first is the whole change against `main`; the second sets the change against RE-290's head, so it shows what `Height` alone moves. Once RE-290 is on `main`, the first shows the same.
