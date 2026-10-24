# RE-200: a family instance whose type's map holds no material takes the materials its own parameters name

**Date:** 2026-10-20
**Issue:** #355 (the element-to-material link for family instances); follows RE-82, RE-149 and #580
**Artefacts:** `R2025_Models.zip` from #356 (Apache-2.0, sha256 `8673d803…6a`, `research/r2025-models.tsv`): models 12 to 17 hold one window with nested trims and muntins, and Revit's own IFC4 Reference View export. Checked for regressions on `2024_Core_Interior.rvt` (sha256 `c805df44…8014`) against `2024_Core_Interior_slim.ifc` and on the four RE1 models (`Drshelden/IFC-ECS`, MIT, the CI pins). Snowdon Towers Sample Architectural (Revit 2025 copy, sha256 `8ac90a85…d334`) against Revit's IFC4 export (`bldrs-ai/test-models`, sha256 `ecfcb04e…985a`) was measured locally only: it cannot be fetched in CI.
**Probe:** `examples/probe_re200_instance_material_params.rs`.
**Status:** positive. Exported: such an instance takes those materials before `<Unnamed>` (RE-149).

## 1. The question

#580 gives `<Unnamed>` to every column and product whose type's geometry-material map (RE-82, `FamilySymbol.m_geomTag2MaterialId`) holds only unset values (RE-149). An unset value is geometry with no material fixed by the type. puzzbobb's comment on #355 (2026-10-02) found two kinds: geometry with no material, which Revit's export writes `<Unnamed>`, and geometry bound to an instance material parameter, which takes the instance's value. #580 implements the first. Does the second hold on a model CI can fetch, and what does `<Unnamed>` cost there?

## 2. Method

For each column and product of the partition decode whose type has an all-unset map, the probe reads the instance's native record with `native_document::extract_graphs`. It follows the root `FamilyInstance`'s `m_pInstParams` pointer to its `FamilyParams` object and takes each `m_params` entry's `m_elemId` that is one of the file's materials (`partition_materials::scan_material_names`), in order, each once. The export is scored with `tools/re/element_materials_vs_ifc.py` (IfcOpenShell 0.9.0.post1, as `tools/ci/requirements-ifc.txt` pins), on `main` (`e56245f`) and on this change.

## 3. Result

### 3.1 The R2025 models

Each of models 12 to 17 has one window type, `Window-Fixed:24" x 36"`, placed three times. Each placement has three nested families, which Revit's export writes as their own `IfcBuildingElementProxy`. All three types have only unset values in their maps.

| family | map | instance parameters' materials | Revit's export |
|---|---|---|---|
| `Trim-Window-Interior-Flat` | 4 × unset | Wood - Stained | `IfcMaterial` Wood - Stained |
| `Trim-Window-Exterior-Flat` | 4 × unset | Clad - White | `IfcMaterial` Clad - White |
| `Muntin Pattern_2x2` | 2 × unset | Wood - Stained, Clad - White | `IfcMaterialConstituentSet` of both |

Model 12 (`24a0eabd…`), its instances 12540 to 12577:

```text
{"class":"GenericModel","id":12540,"instance_materials":["Wood - Stained","Clad - White"],"type":12362}
{"class":"GenericModel","id":12541,"instance_materials":["Clad - White"],"type":12372}
{"class":"GenericModel","id":12542,"instance_materials":["Wood - Stained"],"type":12382}
… (the same for 12562 to 12564 and 12575 to 12577)
{"instances":9,"unset_map_types":14,"with_instance_materials":9}
```

The values sit in `m_params` entries whose `m_paramId` is a family parameter of the window (12363 and 12364 on the muntin pattern, 12369 on the exterior trim, 12378 on the interior trim). Their `m_value` is 0.0 and `m_instance` 0.

Elements with Revit's exact material set:

| models | elements Revit gives materials | `main` | this change |
|---|---:|---:|---:|
| 12 to 17, each | 18 | 4 of 13 given (the 9 trims and muntins `<Unnamed>`) | 13 of 13 given |

The 5 elements of each model rvt-rs gives none are walls, and are not changed here.

### 3.2 No regression on the CI models

| model | instances of all-unset types | with instance materials | score, `main` and this change |
|---|---:|---:|---|
| Core Interior | 256 | 0 | 836 of 842 given, the same |
| RE1 Architecture | 0 | 0 | 70 of 70 given, the same |
| RE1 Electrical | 47 | 0 | 47 of 47, the same |
| RE1 Mechanical | 43 | 0 | 43 of 43, the same |
| RE1 Plumbing | 61 | 0 | 60 of 60, the same |

Every one of those exports, and of the other 16 R2025 models, is byte-identical to `main` apart from its timestamps. The 256 Core Interior columns keep #580's `<Unnamed>` and joined-wall materials.

### 3.3 Snowdon Towers (local only)

767 instances have an all-unset type, and 232 of them have material parameters. Element by element against Revit's export, 232 sets go from wrong to Revit's and none from Revit's to wrong:

| family | elements |
|---|---:|
| Trim-Window-Interior-Flat | 66 |
| Muntin Pattern_2x3 | 64 |
| Trim-Window-Exterior-Flat | 56 |
| Muntin Pattern_3x1 | 24 |
| Muntin Pattern_2x2 | 22 |

Elements with Revit's set: 3,457 → 3,689 of the 4,042 rvt-rs gives materials. That is the 232 of 232 of puzzbobb's rule on #355.

## 4. What is still different

On Snowdon, after this change (none of it measurable in CI):

| elements | family | rvt-rs | Revit |
|---:|---|---|---|
| 218 | Rectangular Mullion | `<Unnamed>` | "Aluminum Clad (Black)" (204), "Aluminum" (14) |
| 94 | Rectangular Column (Off Center) | `<Unnamed>` (81), or the joined walls' layers (13) | one wall material, mostly "Gypsum Wall Board" |
| 21 | Counter Top w Sink Hole | appliance materials | Counter Top (RE-82's nested-appliance case) |
| 17 | Tree - Honey-Locust | its map's two materials | `<Unnamed>` (render type 4) |
| 3 | Refrig_Base_wDrawers, Lobby Chandelier, one wall | | |

- The mullions' material is not reached by RE-166's curtain scan on this file. Running that scan before `<Unnamed>` was tried and changes nothing.
- #580's joined-wall rule gives a column every layer material of its walls. Revit gives one, and it does not reach 81 of these columns.

## 5. What this changes

- `native_instance_materials::instance_parameter_materials`: the material values of an instance's `m_pInstParams`.
- `partition_schema_mvp`: a column or product whose type has an all-unset map takes the materials its instance parameters name, where every one of them is named. Otherwise it takes `<Unnamed>` as before.
- `tests/element_materials.rs`: `r2025_window_trims_take_their_instance_materials` checks the 9 trims and muntins of model 12 against Revit's export. It reads `RVT_PROJECT_CORPUS_DIR/r2025/`, the layout of `tools/ci/fetch-r2025-models.sh`, and skips without it.
