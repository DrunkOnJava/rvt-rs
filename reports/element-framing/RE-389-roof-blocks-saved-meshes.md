# RE-389: a roof drawn as its sketch outline's level block is its saved mesh

**Date:** 2026-11-09
**Issues:** #1612 (one project's saves disagree about a roof); follows #1191 and #1610 (every element that would be a box is drawn from the mesh Revit saved for it), RE-199 and RE-211 (the roofs the slope readers draw), RE-224 (when a saved mesh is whole) and RE-341 section 5
**Artefacts:** Autodesk's sample projects `rac_advanced` and `rac_basic` (2023 to 2027 saves, pinned in `research/autodesk-sample-projects.tsv`); puzzbobb's `20_T_U_Roof` (Revit 2025, Apache-2.0) with Revit's IFC4 Reference View export; Autodesk's Snowdon Towers Architectural (Revit 2024) with Revit's own IFC4 export (local only, no licence, never committed; per-roof percentages and counts only here); seven held-out exports (counts only)
**Runs:** Measure [42670013230](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42670013230) (unfiltered, `nordic`, `r2025`, `samples`, `racadv`, `bimcollab` and `holdout`; code `245245d2` against `main` at `cbeba4c7`, 118 models), [42670020145](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42670020145) (the kept exports of the two sample projects), [42675942905](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42675942905) and [42675946767](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42675946767) (the kept exports of Snowdon Towers Architectural, the branch and `main`), and [42675593198](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42675593198) (the same-project gates on `b1b01f36`)
**Status:** Positive. Of the 11 moved roofs that Revit's own export holds (the T/U roof and Snowdon Towers' 10), all 11 now have their volume within 0.4% of Revit's and their box within 0.01 ft; before, 8 of them were a level block 40% or more too large (the T/U roof 20 times). The rule rests on two models, one of them local only; for the 7 held-out roofs that move, only counts leave the runner, and on 3 of them two scorers disagree (section 4).

## 1. What was drawn

`saved_meshes::attach` replaces the record box of a body with the element's saved mesh where all of the mesh decodes, and skipped every body whose sketch outline was resolved: a floor's plan or a ceiling's is the shape. A roof whose outline the readers recover (Revit 2024 and 2025 and, on `rac_advanced`'s 142430, 2026 and 2027) but whose slope they do not draw was therefore the outline extruded over its record box's full height, a level block, while a roof of the same project whose outline is not recovered was drawn from its mesh. The record box holds the roof's reference surface, not its slope or its thickness.

## 2. What rvt-rs writes

A body with a resolved outline is a candidate when it is an `IFCROOF`; every other class keeps the outline as its body. Where the mesh decodes whole (RE-224) it replaces the block, and `ProfileSource`, `ProfileVertexCount`, `ProfileVoidCount` and `SlabShapeRefusal`, which described the block, go with it while `ProfileResolved` reads false. A mesh that does not decode whole keeps the block, as before.

## 3. Measured

**Which exports move.** Unfiltered Measure 42670013230: 15 of the 118 exports differ and 103 are byte-identical. The body sources move by roofs only, from `partition_element_record_bbox` to `saved_graphics_mesh`:

| Export | Roofs that move |
|---|---:|
| `rac_advanced` 2024, 2025 | 4 each (142430, 165755, 167036, 167158) |
| `rac_advanced` 2026, 2027 | 1 each (142430) |
| `rac_basic` 2024, 2025 | 1 each (724430) |
| Snowdon Towers Architectural (2024) | 10 |
| `20_T_U_Roof` (Revit 2025) | 1 |
| 7 held-out exports (3 saves of one project, 4 of another) | 1 each |

No property or quantity census moves (`psets_vs_ifc` and `qtos_vs_ifc` are equal on every model, 0 regressions), and `exported.slab_shape_refusals` does not change on any model.

**Where Revit's export holds the roof.**

- **`20_T_U_Roof`, the roof of the T and U footprints** (`roof_bodies_vs_ifc.py`): the level block was 37,356.98 ft3 against Revit's 1,845.58, its farthest vertex 18.4 ft from Revit's; the mesh is 1,845.578 ft3 and every vertex of Revit's body lies within 4.2e-06 ft of it (`mesh_bodies_vs_ifc.py`: volume ratio 1.0000, `saved_meshes_vs_ifc.py`: the box within 0.01 ft). The other roof of the model matched before and still does: 2 of 2 equal.
- **Snowdon Towers Architectural, the 10 roofs that move** (`roof_bodies_vs_ifc.py`, the roof's own and its parts' meshes together): 3 were already equal as blocks (flat roofs whose block is the roof); the other 7 were 40.3%, 40.7%, 44.9%, 87.3%, 116.3%, 124.5% and 124.8% above Revit's volume and are now between -0.2% and +0.4% of it. `saved_meshes_vs_ifc.py`: the box of all 10 is within 0.01 ft of Revit's (IfcRoof 10 of 10); `mesh_bodies_vs_ifc.py`: every vertex of the roofs paired with Revit's roof within 0.001 ft in both directions.
- **7 held-out exports**, one roof each (counts only): in the 4 saves of one project the roof's box is within 0.01 ft of Revit's; in the 3 saves of the other `saved_meshes_vs_ifc.py` puts it more than 1 ft off, while `mesh_bodies_vs_ifc.py` puts every vertex of the paired body within 0.001 ft in both directions. See section 4.

**The same-project gates.** The 2026 and 2027 saves of `rac_advanced` pair 5,526 of their 5,527 bodies with the 2025 save's, the one difference being roof 201262 as pinned, and the roofs of `rac_basic` are as pinned: 165755, 167036 and 167158, the 15 degree gables the 2025 save drew as a level block, are the 2025 save's meshes now, within 3 mm and 0.1%, and leave `ROOFS_NOT_THE_2025_BODY`. The gate that counts sketch outlines falls short of its floors by the roofs that are no outline any more (`OUTLINE_LEAST`: `rac_advanced` 86 to 82 on the 2025 save and 82 to 81 on 2026 and 2027, `rac_basic` 13 to 12 on 2025), which the floors now say; the verdict on `b1b01f36` is a pass, 15 samples held to their earlier saves and the Areas on 4 saves (Measure 42675593198).

## 4. Not shown

- **The Autodesk samples' roofs have no Revit export.** `rac_advanced`'s and `rac_basic`'s are held to each other, not to Revit's. Snowdon Towers (Revit 2024) and `20_T_U_Roof` (Revit 2025) are the roofs this was measured on; no Revit 2026 or 2027 roof has an export here.
- **Three held-out roofs on which the scorers disagree.** The box scorer pairs each Tag with Revit's last body of that Tag and the shape scorer pairs it with the body of the roof and its parts; where Revit writes a roof as an `IfcRoof` with `IfcSlab` parts under one Tag (7 of the 20 roofs of Revit's export of Snowdon Towers are an `IfcRoof` without a body and one `IfcSlab` with it) that need not be one element. The held-out counts cannot say whether that explains the 3.
- **A volume ratio of 0.5 in `mesh_bodies_vs_ifc.py`.** On 7 of Snowdon Towers' roofs the ratio of the mesh's volume to that of the Revit body it pairs with is 0.5003, with every vertex of the two within 0.01 ft of the other's surface, while `roof_bodies_vs_ifc.py`, which takes the roof's parts together, finds the volumes equal. The ratio looks like the pairing's (one of two equal parts) and not the mesh's; that is not established.
- **The other roofs.** 10 of Snowdon Towers' 20 roofs stay a level block (candidates whose saved graphics did not decode whole, or that have none; their volumes are 5% to 133% above Revit's), and on the Autodesk samples 201262 and `rac_basic`'s 243274 are as RE-341 and the same-project gate state them.

## 5. Reproduce

```bash
# The unfiltered run of section 3 (the code commit against the main it branched from):
gh workflow run measure.yml --ref feat/roof-level-block-meshes -f ref=245245d287071f66ac7b862a6514625eb1e29ad0 -f base=cbeba4c7 \
    -f nordic=true -f r2025=true -f samples=true -f racadv=true -f bimcollab=true -f holdout=true
# The kept exports of Snowdon Towers (score them locally against Revit's export with tools/re/roof_bodies_vs_ifc.py,
# saved_meshes_vs_ifc.py and mesh_bodies_vs_ifc.py; `-f models` does not narrow samples, so every sample is kept):
gh workflow run measure.yml --ref feat/roof-level-block-meshes -f ref=b1b01f3671b61ce2b40d724a42551958f1bf589c -f base=none \
    -f samples=true -f models="sample-2024-snowdon_towers_sample_architectural" -f scorers=none -f keep_ifc=true
# The same-project gates on the head:
gh workflow run measure.yml --ref feat/roof-level-block-meshes -f ref=b1b01f3671b61ce2b40d724a42551958f1bf589c -f base=none \
    -f samples=true -f sample_years="2024 2027" -f scorers=bodies_across_saves
```
