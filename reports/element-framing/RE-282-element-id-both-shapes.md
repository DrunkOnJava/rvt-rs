# RE-282: reading an ElementId in a 4-byte Identifier moves Revit 2023 exports no export checks

**Date:** 2026-10-30
**Issues:** #1158; follows RE-280 (the framing, `Identifier` as 2015 to 2023 write it) and RE-281 (the Element base on both shapes)
**Artefacts:** the unfiltered Measure models and samples (Autodesk's sample projects of 2015 to 2027, BIMcollab's Revit 2015 example, the held-out models, the two offices, the R2025 models, the reference models and Einhoven); no file is stored.
**Runs:** Measure [41130410404](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41130410404) (`-f base=main -f nordic=true -f r2025=true -f samples=true -f bimcollab=true -f holdout=true`, the change at `31ec7edc` against `main` at `86b25184`, which is `3811a654` plus a change to CI alone) and [41135570857](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41135570857) (the same two commits on the 2023 files, `-f keep_ifc=true`, the exported IFCs diffed).
**Status:** negative, and it decides how #1158 goes on. Every native reader that reads an ElementId through `native_json::element_id` or `element_id_i64` reads `{"m_id":{"m_id64":n}}`, Revit 2024's `Identifier`, and finds nothing in the 4-byte `{"m_id":{"m_id":n}}` of 2015 to 2023. Reading both shapes in every reader (the six places that read `/m_id/m_id64`, `31ec7edc`) leaves 91 of 94 exports byte-identical (all 30 samples of 2015 to 2022, BIMcollab, the 3 held-out models, all 25 samples of 2024 to 2027, the 24 R2025 models, the offices, Core Interior and RE1) and moves the three Revit 2023 files that are not scored against a Revit export. The readers behind that are admitted to 2015 to 2022 one at a time, each reading the ids it needs itself (RE-283 and after), and not in a shared helper.

## 1. What was changed and measured

`native_json::element_id` and `element_id_i64` were made to read either shape (`identifier_i64`, the helper RE-281 added for the Element base), and the five places that read the pointer `/m_id/m_id64` by hand (`native_stairs`, `native_family_forms`, `native_type_materials`, `native_parameters`, `partition_materials`) were pointed at it. `Options::releases` was left as it is: the default, 2023 to 2027, so a reader that does not name `NATIVE_RELEASES` is still refused on 2015 to 2022.

## 2. What it did

| group | exports | identical | moved |
|---|---:|---:|---:|
| samples, 2015 to 2022 | 30 | 30 | 0 |
| samples, 2023 (`rac_basic`, `rac_advanced`, `rst_basic`) | 3 | 1 | 2 |
| Einhoven (2023) | 1 | 0 | 1 |
| samples, 2024 to 2027 | 25 | 25 | 0 |
| R2025 models | 24 | 24 | 0 |
| BIMcollab (2015), held-out models, offices, Core Interior, RE1 | 11 | 11 | 0 |

The 2015 to 2022 files do not move because the readers that would now find ids refuse their release (`Options::releases`). The 2024 to 2027 files do not move because their JSON has `m_id64` and `identifier_i64` reads that first. The 2023 files are inside the default gate, so the readers that found no id on them before now find them:

- **Connector ports**: the 2023 `rac_basic` gains 36 `IfcDistributionPort`s and the `IfcRelNests` that nest them, and `rac_advanced` 30 (`ifc/mod.rs`, `native_connectors::connector_ports`). `rst_basic` has none either way. RE-131, RE-138 and RE-169 measured ports on RE1 (2025); nothing measures them on 2023.
- **Wall layer sets**: the 2023 `rac_advanced` gains an `IfcMaterialLayerSet` ("Basic Wall:Generic - 200mm") and four `IfcRelAssociatesMaterial`s, where a layer takes its category's material and the object-styles entry does not give it; Einhoven gains one `IfcRelAssociatesMaterial` (a wall associated with a material already in the file, no layer set). By the code path, the only reader of this kind the 2023 record path reaches is `scan_category_material`'s fallback to the category's graphics styles (`native_category_material`, RE-206), which was measured on Revit 2025 (Snowdon Towers) and never on 2023.

No Revit export of these files exists, and the one held-out Revit 2023 model with one (modelo_bim, exported by Revit 26.2) did not move. So the new ports and layer sets are values that are written and not checked.

## 3. What follows

- `31ec7edc` is reverted. A reader's ids are read in either shape inside that reader (`identifier_i64`), when the reader is admitted to a release: RE-283 does it for a wall's type, and RE-284 needs no id at all.
- The readers that stay refused on 2015 to 2022 and silent on 2023 are the ones that read an ElementId through `native_json::element_id`: types, stairs and landings, symbol cuts, mullion hosts, connectors and ports, column joins, family forms, instance and type materials, saved meshes. Each of them is a separate admission, measured against the 2024 copy of the same sample project or a Revit export, and several move an export when admitted (the two items above are the first two).
- The measurement that decided this is one command and does not need a probe:

```text
gh workflow run measure.yml --ref main -f ref=31ec7edce697f0049f1867ee491f962a6ebeec45 -f base=3811a6545aff148b14b6e69c28be2e26501c812b \
  -f nordic=true -f r2025=true -f samples=true -f bimcollab=true -f holdout=true
gh workflow run measure.yml --ref main -f ref=31ec7edce697f0049f1867ee491f962a6ebeec45 -f base=3811a6545aff148b14b6e69c28be2e26501c812b \
  -f samples=true -f sample_years="2023 2023" -f models="einhoven sample-2023-*" -f scorers=none -f keep_ifc=true
gh run download <run id>   # measure-head/<model>/ifc.sha256 against measure-base/<model>/ifc.sha256; model.ifc
```

## 4. What this does not claim

- That the 36 and 30 ports, or the layer sets, are wrong. They are unmeasured: a Revit 2023 export of a project with MEP connectors, or the 2024 copy of each sample read the same way, would decide, and neither was compared here.
- That the other readers move 2023 exports when admitted to 2015 to 2022: they refuse those releases whatever their ids.
