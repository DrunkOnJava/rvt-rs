# RE-293 — A roof-joined wall's Height

**Date:** 2026-10-31
**Issues:** #1046 (quantities Revit computes from its joined geometry, step 1: walls), #1016 (quantities), W7 of [`docs/release-1.0-criteria.md`](../../docs/release-1.0-criteria.md); follows RE-266 (a wall's `Height` where nothing joins to or cuts it), RE-267 (the top cut at a covering floor), RE-270 (a wall's side areas and volumes) and RE-290 (the cut-back centreline that confirms a roof-joined wall's length)
**Artefacts:** the BIM4LCA office as built and at building permit (Revit 2024, Revit 24.0.5's IFC4 exports, CC BY-SA 4.0, `research/bim4lca-measure.tsv`) and puzzbobb's R2025 models (Revit 2025, Apache-2.0, `research/r2025-models.tsv`), as Measure fetches them
**Scorer:** `tools/re/wall_quantities_vs_ifc.py` (RE-270)
**Runs:** Measure [41136551451](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41136551451) (`main` at f73908bf: the scorer, which lists Revit's and rvt-rs's `Height` of each wall) and RUN_MEASURE (this change at 9c39b0c3 against RE-290's head ada5ca73, every model and scorer)
**Result:** positive. Revit's export writes a wall's `Height` on exactly the walls it writes `GrossSideArea`, `NetSideArea`, `GrossVolume` and `NetVolume` on (RE-270 §1), and on every wall where those four are Revit's, rvt-rs's extrusion is as tall as Revit's solid: 117 of 117 as built and 128 of 128 at permit, 0 different. The pass that gives a roof-joined wall the four (RE-270's second rule, RE-290) now gives it `Height` too: BUILT_N more office walls as built and PERMIT_N at building permit, Revit's on every one, none different and none Revit's export leaves out.

## 1. Why the four and `Height` go together

revit-ifc's `PropertyUtil.CreateWallBaseQuantities` (CLEANROOM.md source 5; read at the tags `IFC_v24.1.0` and `IFC_v24.2.0`) writes `Height` when its caller hands it a positive depth, and `GrossSideArea`, `NetSideArea` and `NetVolume` out of the same call from the wall's solids; `GrossVolume` needs the depth too. On the office Revit writes `Height` on 579 of 607 walls as built and 565 of 589 at permit, and the four on the same walls (the others carry only a `NetVolume`, RE-270 §1).

RE-266 writes `Height` where the wall's record names nothing joined to or cutting it, because only there is the record box as tall as Revit's body. RE-270's second rule writes the four on a roof-joined wall only after its joins are shown to leave the prism whole (every element they name lies wholly above its top or below its base, RE-267's top cut applied first) and its length is confirmed (RE-290). A wall that passes that is a rectangular prism: its extrusion's height is the vertical span of Revit's solid.

## 2. Measured against Revit's walls

`tools/re/wall_quantities_vs_ifc.py` lists, per wall, Revit's `Height` and the height of rvt-rs's body (`U ... H=`). On the record-box rectangles whose record holds no step but the harmless ones, a roof or floor join and a clean join, where `GrossSideArea`, `NetSideArea` and `NetVolume` of rvt-rs's body are Revit's (the walls the rules take, and a few their other conditions leave out):

| | as built | at permit |
|---|---:|---:|
| walls with no roof or floor join (RE-266 writes `Height`) | 49 | 48 |
| ... rvt-rs's body is as tall as Revit's `Height` | 49 | 48 |
| roof-joined walls | 68 | 80 |
| ... rvt-rs's body is as tall as Revit's `Height` | 68 | 80 |

(Run 41136551451, walls matched by `Tag`, a relative 1e-5.) Where `GrossSideArea` is `Length` times `Height` in Revit's export and the length is the box's, equal side areas imply equal heights; that is why the check cannot be circular only on the wall list: the roof-joined walls' heights include the ones RE-267 cut at a covering floor (BUILT_CUT as built, PERMIT_CUT at permit among those the rule takes), and rvt-rs's cut height is Revit's on all of them.

## 3. The rule

`export_content::wall_areas::add_bounded_wall_quantities` writes `Height`, the extrusion's height, beside the four on each wall it vouches for. It adds no condition: a wall is taken exactly as RE-270 and RE-290 say. Walls Revit writes only a `NetVolume` on carry no `Height`, and the rule takes none of them.

## 4. Measured

MEASURED_SECTION

## 5. What stays unwritten

`Height` on a roof-joined wall the second rule does not take: the 483 office walls as built (459 at permit) still without the four are also without `Height`, apart from the uncut ones RE-266 covers. Where a wall's top is joined to a roof Revit's `Height` is the attached wall's (higher on the R2025 roofs, lower on 56 of 57 office walls, RE-266 §5), so it waits on the body Revit cuts (#1067) except on the walls a floor leaves whole.

## 6. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=9c39b0c310f99e1dbd501bc30dd397132c6ca971 -f base=ada5ca7340ee1f3de53ea1c69b3d7e9d208448e7 -f nordic=true -f r2025=true
```

`base` is RE-290's head, so the run shows what `Height` alone moves; once RE-290 is on `main`, use `-f base=main`.
