# RE-263 — The storey count Revit's exporter writes

**Date:** 2026-10-29
**Issues:** #1097, W7 of [`docs/release-1.0-criteria.md`](../../docs/release-1.0-criteria.md); found while measuring #1086 (a project with one Level exports its storey)
**Artefacts:** puzzbobb's 22 Revit 2025 exports (`research/r2025-models.tsv`, Apache-2.0), RE1's four Revit 2025 models (MIT), the BIMcollab Revit 2015 structural example, and, local only, Snowdon Towers Architectural (2024) and the held-out `teste_export_2025` and `modelo_bim` (`research/holdout-models.tsv`; only the counts below are reported); Autodesk's open-source exporter revit-ifc (source 5 of [`CLEANROOM.md`](../../CLEANROOM.md)), read and cited, not copied
**Runs:** Measure [40942112911](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40942112911) (#1086 against main af500685, every model and scorer) and [40948455973](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40948455973) (main with this branch's census)
**Result:** positive, a fact about Revit's exporter rather than the file: its `Pset_BuildingCommon.NumberOfStoreys` counts the storeys that contain an element up to IFC 26.0 and every storey from IFC 26.1. The value rvt-rs writes, the number of storeys, is IFC4's definition and what each exporter since 26.1 writes. The census counts the older value apart. No export changes.

## 1. What differs

rvt-rs writes `NumberOfStoreys` as the number of `IfcBuildingStorey` it exports. The W7 property census (`tools/re/psets_vs_ifc.py`) counted that value different on 20 of puzzbobb's 22 exported Revit 2025 models at main af500685, and #1086 makes it 21 by giving 00_Blank its storey (Measure 40942112911). On RE1 and the BIM4LCA office it is equal.

## 2. Revit's exporter

In revit-ifc, `NumberOfStoreysCalculator` (`Source/Revit.IFC.Export/Exporter/PropertySet/Calculators/NumberOfStoreysCalculator.cs`) gives the property its value. Until the commit "IFC 26.1 for Revit 2026" (ef15008e2e, 2025-04-24) it took the value from `ExporterIFCUtils.GetNumBuildingStoreys`, a call into Revit itself whose rule the source does not show. Since that commit it takes the number of entries in the exporter's level cache. Each export names its exporter in its `FILE_NAME` header ("Autodesk Revit 25.0.2.419 (ENU) - IFC 25.0.2.419").

## 3. Measured

For every reference export that writes the property: the exporter its header names, its storeys, the storeys some element is contained in (`IfcRelContainedInSpatialStructure`) and Revit's value.

| Export | Schema | Exporter | Storeys | Containing an element | NumberOfStoreys |
|---|---|---|---|---|---|
| R2025 00_Blank | IFC4 | IFC 25.0.2.419 | 1 | 0 | 0 |
| R2025 01_Walls | IFC4 | IFC 25.0.2.419 | 1 | 1 | 1 |
| R2025 02 to 17 (19 models) | IFC4 | IFC 25.0.2.419 | 4 | 2 | 2 |
| R2025 18_WallJoins | IFC4 | IFC 25.0.2.419 | 4 | 1 | 1 |
| `teste_export_2025` (held out) | IFC4 | IFC 25.4.3.0 | 2 | 1 | 1 |
| Snowdon Towers Architectural | IFC4 | IFC 24.0.20.20 | 18 | 18 | 18 |
| BIMcollab STR (2015) | IFC2X3 | not named (Solibri IFC Optimizer) | 5 | 5 | 5 |
| RE1 Architecture, Electrical, Mechanical, Plumbing | IFC2X3 | IFC 26.2.0.20 | 2 | 1 | 2 |
| `modelo_bim` (held out) | IFC4 | IFC 26.1.0.23 | 4 | 3 | 4 |

On all 24 exports by an exporter before 26.1 that name it, and on the BIMcollab one, the value is the number of storeys containing an element. On all five by 26.1 or later it is the number of storeys. Where the two counts are equal (Snowdon, BIMcollab) the exports cannot tell the rules apart.

## 4. What follows

A `.rvt` does not say which exporter will export it: a Revit 2025 file exported from Revit 2026 gets the newer value (RE1, and `modelo_bim`, saved in Revit 2023). rvt-rs keeps writing the number of storeys, IFC4's "number of storeys within the building" and what the current exporters write. The census reads the exporter from each reference export's header and counts `NumberOfStoreys` apart, on a line of its own, where the exporter is older than IFC 26.1, its storeys do not all contain an element, and its value is the number that do. An export whose header names no exporter is scored as before. On Measure 40948455973 that line counts the property on the 20 R2025 models that differ, 00_Blank among them, and nothing else changes; RE1's and the office's stay equal.
