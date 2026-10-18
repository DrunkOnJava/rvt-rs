# RE-188 — Revit 2024's exporter writes a curtain wall as IfcCurtainWall whatever its "Export to IFC As" says

**Date:** 2026-10-18
**Issues:** #792
**Result:**
- On both BIM4LCA office files (Revit 2024), 124 curtain walls carry their own "Export to IFC As" (`IFC_EXPORT_ELEMENT_AS`) of `IfcShadingDevice`, read by RE-45 on a `Walls` record. Revit's export of each file writes all 124 as `IfcCurtainWall`; rvt-rs followed the value and wrote shading devices.
- Revit 2024's exporter never reads the value on a curtain wall. In revit-ifc `IFC_v24.1.0` to `IFC_v24.3.40`, `CurtainSystemExporter.ExportBase` creates every curtain wall, curtain system and legacy curtain element it exports as an `IfcCurtainWall`, whatever the element's parameters say. From `IFC_v25.2.0` (Revit 2025) on, `ExportBase` takes the entity from `ExporterUtil.GetProductExportType`, the lookup every other element goes through, which reads the instance's value and then its type's.
- A curtain wall in a 2024 file now stays `IfcCurtainWall`; in a 2025 or 2026 file it keeps following the value, as the newer exporters' source does. The 2025 and 2026 half is read from source only: no Revit 2025 or 2026 export of a curtain wall carrying a value has been measured (the RE1 models carry none). The maintainer chose to follow the setting there (2026-10-18).
- The export diagnostics now count every value the export does not apply, by value and class, in a warning.

| Against Revit's export, by `Tag` (`class_sets_vs_ifc.py`) | main | RE-188 |
|---|---:|---:|
| Office as built: Revit's 220 `IfcCurtainWall` written as `IfcCurtainWall` | 96 | 220 |
| Office as built: written as `IfcShadingDevice` | 124 | 0 |
| Office at building permit: Revit's 220 `IfcCurtainWall` written as `IfcCurtainWall` | 96 | 220 |
| Office at building permit: written as `IfcShadingDevice` | 124 | 0 |
| Core Interior: Revit's 20 `IfcShadingDevice` (floors) written as `IfcShadingDevice` | 20 | 20 |

-----

## 1. Where the value sits

`probe_re45_ifc_export_parameters` prints every element carrying an export parameter, and every "Export to IFC As" entry with the element-data header the owner rule takes. `tools/re/export_parameters_vs_ifc.py` pairs each with the element of the same `Tag` in Revit's export (Measure run [39153127128](https://github.com/DrunkOnJava/rvt-rs/actions/runs/39153127128), `-f nordic=true`, as built):

| Own value | Revit wrote for that Tag | Elements |
|---|---|---:|
| `IfcWall` | `IfcWall` | 605 |
| `IfcColumn` | `IfcColumn` | 278 |
| `IfcWindow` | `IfcWindow` | 269 |
| `IfcDoor` | `IfcDoor` | 262 |
| `IfcShadingDevice` | `IfcCurtainWall` | 124 |
| `IfcCurtainWall` | `IfcCurtainWall` | 75 |
| `IfcSpace` | `IfcSpace` (Name ends in the id) | 44 |
| `IfcFireSuppressionTerminal` | `IfcFireSuppressionTerminal` | 40 |
| `IfcWall` | `IfcCurtainWall` | 21 |
| `IfcCovering` | `IfcBuildingElementProxy` | 9 |

The 124 entries' owners are records of category `Walls`: the value is each curtain wall's own, not a neighbour's. The 21 curtain walls whose own value is `IfcWall` go through the same exporter path; rvt-rs already wrote them as curtain walls, since `IfcWall` is not a value the writer acts on.

On Core Interior (Revit 2024) the 20 floors whose value is `IfcShadingDevice` are `IfcShadingDevice` in Revit's export (RE-22, RE-45): the 2024 exporter does act on the value on floors. The rule is the curtain-wall path's, not the value's.

## 2. What the exporter does, by release

Read from revit-ifc (CLEANROOM source 5), at each release tag:

| Tag | `CurtainSystemExporter` |
|---|---|
| `IFC_v24.1.0` to `IFC_v24.3.40` | `ExportBase` creates the element as a curtain wall and does not consult the export type |
| `IFC_v25.2.0` | `ExportBase` gets the export type from `GetProductExportType` and creates that entity |
| `IFC_v25.4.40`, `IFC_v26.1.0`, `IFC_v26.4.1` | the same, with `ExportWall` passing the export type in |

The rule keys on the release that saved the file, which stands for the exporter: exporting from a later Revit upgrades the file first.

## 3. Left open

- **Wall sweeps.** The 9 wall sweeps whose own value is `IfcCovering` are proxies in Revit's export. `WallSweepExporter` is the same at `IFC_v24.3.40`, `IFC_v25.2.0` and `IFC_v26.4.1`: it exports through `GenericElementExporter`, which reads the value. So this is not the curtain walls' version split, and is not explained here; rvt-rs keeps writing them as coverings.
- **Entries without an owner.** 198 `IfcSpace` entries on the as-built file have no element-data header within the owner window. That is a limit of RE-45's owner rule on this file, not part of this finding.
- **The other mismatches of #792** (fire suppression terminals, family instances exported as spaces, type-level values) are values the writer does not act on yet. They are now listed in the diagnostics warning, not changed.

## 4. End-to-end measurement

Measure run [39156744563](https://github.com/DrunkOnJava/rvt-rs/actions/runs/39156744563), `-f nordic=true`, `main` (bc4c4eb) against this change (edb585b), on the CI reference models and the two BIM4LCA office files:

| file | sha256 |
|---|---|
| `ARK_NordicLCA_Office_Concrete_As-Built.rvt` (VTT BIM4LCA, CC BY-SA 4.0) | `74712000…` (`research/bim4lca-measure.tsv`) |
| `ARK_NordicLCA_Office_Concrete_BuildingPermit.rvt` | `1c12c205…` |
| `ARK_NordicLCA_Office_Concrete_As-Built_Revit.ifc` (Revit 2024's export) | `b2f530eb…` |
| `ARK_NordicLCA_Office_Concrete_BuildingPermit_Revit.ifc` | `bc3bd336…` |

```bash
gh workflow run measure.yml -f ref=<branch> -f base=main -f nordic=true
gh workflow run measure.yml -f ref=<branch> -f base=none -f nordic=true \
  -f probe=probe_re45_ifc_export_parameters -f probe_scorer=export_parameters_vs_ifc
```

- **The two office files:** the 124 shading devices become `IfcCurtainWall` (IFC `IFCCURTAINWALL` 96 to 220, `IFCSHADINGDEVICE` 124 to 0). `class_sets`, `aggregates` and `names` move by exactly those elements. Their names still differ from Revit's, as all 220 curtain walls' did before (a separate gap).
- **Core Interior, Einhoven and the four RE1 models:** every output is byte-identical to main's apart from the timing lines. No exported element on them carries a value the export does not apply, so their diagnostics gain no warning.
- **The diagnostics warning** on the as-built file reads: 381 elements carry a value the export does not apply. It lists them as `IfcShadingDevice` on `CurtainWall` 124, `IfcWasteTerminal` on `PlumbingFixture` 78, `IfcElectricAppliance` on `ElectricalEquipment` 51, `IfcSpace` on `GenericModel` 42, `IfcFireSuppressionTerminal` on `SpecialtyEquipment` 40, `IfcWall` on `CurtainWall` 21, `IfcSanitaryTerminal` on `Casework` 15, and five smaller groups (10 elements). Those values account for #792's other mismatches, which a later change can act on, one value at a time, against these two exports.
