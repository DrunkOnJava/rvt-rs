# RE-377: Revit's material for a curtain panel is the Curtain Panels category's, where the category holds one

**Date:** 2026-11-09
**Issues:** #1593 (follow-up of RE-367, #1520); follows RE-166, RE-201 (a curtain part type's material is its `m_renderStyleId`) and RE-367 (Revit 2015 to 2023 parts, and the panels left without a material)
**Artefacts:** Vitoria's `ALD26 Before` (Revit 2018, BIM-Speed demonstration projects, TU Berlin DepositOnce, CC BY 4.0, `research/holdout-archive-models.tsv`, spent) and `ALD26 Project` (held out, counts only) with Revit's own IFC2X3 exports of both; Autodesk's `rac_advanced` of 2015 to 2023 (`research/autodesk-sample-projects.tsv`) with the IFC2X3 export of the project from the three.js repository (`research/rac-advanced-export.tsv`, counts only, counted apart as IFC2X3); the BIM4LCA office as built and at building permit (Revit 2024, IFC4); RE1 Architecture (Revit 2025); the held-out pairs of Revit 2022 to 2027 (counts only); puzzbobb's 24 Revit 2025 models. Every model Measure reads that holds a curtain panel is in section 3.
**Probe:** `probes/examples/probe_re377_panel_material_sources.rs`.
**Scorer:** `tools/re/panel_material_sources_vs_ifc.py`, and `tools/re/element_materials_vs_ifc.py` for the result.
**Status:** positive on Revit 2015 to 2023, measured on one project (two files). **Open on Revit 2024 and later:** no model of those releases has a Curtain Panels category that holds a material, so which of the category's and the type's Revit's exporter prefers there is not measured.

## 1. The question

RE-367 left the curtain panels of Revit 2015 to 2023 without a material. A part's material is its type's `m_renderStyleId` (RE-166, RE-201), which is Revit's on every mullion and panel of Autodesk's samples and of the BIM4LCA office, and it is not Revit's on any of the 67 plates of `ALD26 Before`. Where does Revit's export take those plates' material from?

## 2. What the probe finds on `ALD26 Before`

For each of the file's curtain panels (an element record of `OST_CurtainWallPanels`) the probe lists every field of its own native record, of its type's record and of the records its reference list names that holds the ElementId of a material, and where a record's body holds a `u32` that is one whatever its field (`byte_hits`). A second pass over every record counts, per material, the classes of record whose body holds its ElementId. The 67 plates share one panel type.

- **No record of a plate holds Revit's material.** The 67 plates' own records hold no material id at all. Their type's record holds one, its `m_renderStyleId`, which is not Revit's on any of the 67; its `m_geomTag2MaterialId` is empty; and the walls and groups its list names hold only the file's default and phase materials. No source the probe lists equals Revit's material on any plate (0 of 67).
- **Revit's plate material is named by two records in the whole file, and both are graphics styles.** They are the `GStyleElem` records whose `m_categoryId` is −2000170, the Curtain Panels category (`OST_CurtainWallPanels`), one with `m_gstyleType` 1 and one with 2 (a projection style and a cut style, the two rows Object Styles keeps for a category). The material is in the `GStyle` object each points to (`m_pGStyle`), as `m_materialElemId`. The two hold the same one. The panel type's render-style material, by contrast, is named by 35 records of six classes (its own type, a family, 13 family symbols and others).

So the plates have the material of their category's Object Styles, which is what a face with no material of its own, "by category", has in Revit.

## 3. Every model with plates

Revit's export of each model, against the materials the Curtain Panels category's graphics styles hold (`panel material references`, counts):

| model | Revit | plates in Revit's export | Curtain Panels category styles holding a material | plates whose Revit material is the category's |
|---|---|---:|---:|---:|
| `ALD26 Before` | 2018, IFC2X3 | 67 | 2 | 67 |
| `ALD26 Project` (held out) | 2018, IFC2X3 | 67 | 2 | 67 |
| `rac_advanced`, saves 2015 to 2020 (three.js export) | 2021 exporter, IFC2X3 | 1,349 | 0 | 0 |
| BIM4LCA office as built | 2024, IFC4 | 260 | 0 | 0 |
| BIM4LCA office at building permit | 2024, IFC4 | 269 | 0 | 0 |
| RE1 Architecture | 2025, IFC4 | 2 | 0 | 0 |
| hemanth1986's office of 2022 (held out) | 2022 | 1 | 0 | 0 |

No other model Measure reads holds a plate in Revit's export. Where the category holds no material, Revit gives the plates the type's: Glass and Stone - Granite on `rac_advanced`, as the type's `m_renderStyleId` reads (RE-367).

## 4. The rule

A curtain panel of Revit 2015 to 2023 has the material the Object Styles give the Curtain Panels category where they give one (`partition_curtain_materials::category_material`: every graphics style of the category holds the same material, one of the file's named materials), and its type's `m_renderStyleId` otherwise. Where the category's styles disagree, or one holds none, the category is taken to hold none. A mullion keeps its type's material: of the 41 models whose probe output is complete (`ALD26 Before`, the BIM4LCA office, RE1 Architecture, puzzbobb's 24 Revit 2025 models and Autodesk's samples of 2023 and 2024) only `ALD26 Before` has a Curtain Panels category that holds a material, and none has a Curtain Mullions one. Revit's export of `ALD26 Before` writes no material on its 206 mullions, and rvt-rs writes none either.

## 5. Result

Measure run [42610822894](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42610822894) (the rule at `5fd97f7a` against `main` at `b969296e`, every flag, every scorer, 118 models a side), scored by `tools/re/element_materials_vs_ifc.py` against Revit's own exports:

| model | plates Revit gives a material | before | now |
|---|---:|---:|---:|
| `ALD26 Before` (IFC2X3) | 67 | 0 of 67 | 67 of 67 |
| `ALD26 Project` (held out, counts) | 67 | 0 of 67 | 67 of 67 |
| `rac_advanced` 2015 to 2020, each save against the three.js IFC2X3 export (counts) | 1,349 | 0 of 1,349 | 1,349 of 1,349 |
| hemanth1986's office of 2022 (held out, counts) | 1 | 0 of 1 | 1 of 1 |

On each, the elements whose material set equals Revit's rise by exactly the plates (`ALD26 Before` 375 to 442, `ALD26 Project` 481 to 548, `rac_advanced` 3,903 to 5,252 on every save, the 2022 office 31 to 32), no set is different, and the plates leave the list of elements Revit gives a material and rvt-rs none. The 1,349 `rac_advanced` plates are the ones the type's render style gives (Glass, Stone - Granite), as before RE-367 narrowed them; the 67 and 67 are the category's.

What else moves: 31 of the 118 exports, all of Revit 2015 to 2023 saves with curtain panels (the three Vitoria and 2022 files, `racadv-2015` to `racadv-2020`, and 22 of Autodesk's sample saves: `rac_advanced` and `rac_basic` of every year 2015 to 2023, `rme_advanced` and `rme_basic` of 2015 and 2019), by the panels' materials alone. No scorer output but `element_materials_vs_ifc` and the held-out counts moves, apart from one: `glb_material_colours_vs_ifc` counts 67 more `('revit', 'no colour')` on `ALD26 Before`, the plates, which the GLB now draws in a model material where it drew a category colour, and for which the scorer finds no colour on Revit's side of an IFC2X3 export. `diagnostics.json` is identical on all 118, no file of Revit 2024 or later moves, and `same-project gates` pass.

## 6. Not shown

- **Which rule Revit's exporter follows.** "The category's material first" and "the 2018 exporter takes the category's, newer exporters the type's" give the same answers on every model measured, because the only model whose category holds a material has an export of 2018. The rule is applied on Revit 2015 to 2023, where the file and its export are of those releases or older; on 2024 and later a panel keeps its type's material.
- That the type's `m_renderStyleId` is the type's Material parameter: its value is the material Revit gives wherever the category holds none, and is another where it does.
- A panel's material on a file whose Curtain Panels category holds a material that is not one of the file's named materials.

## 7. Reproduce

```bash
# The probe on Vitoria's spent pair, the held-out pairs and the models with plates, from a commit that holds it:
gh workflow run measure.yml --ref main -f ref=<a commit that holds the probe> -f base=none -f holdout=true -f racadv=true -f nordic=true -f r2025=true \
    -f samples=true -f sample_years="2023 2024" -f scorers=none \
    -f models="spent-bimspeed-vitoria-before-2018 holdout-* racadv-* nordic-office-as-built nordic-office-permit re1-architecture r2025-*" \
    -f probe=probe_re377_panel_material_sources -f probe_scorer=panel_material_sources_vs_ifc
```
