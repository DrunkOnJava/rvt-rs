# RE-192 — Revit 2024's export writes a wall sweep as a proxy whatever its "Export to IFC As" says

**Date:** 2026-10-18
**Issues:** #792
**Result:**
- On both BIM4LCA office files (Revit 2024), 9 wall sweeps each carry their own "Export to IFC As" (`IFC_EXPORT_ELEMENT_AS`) of `IfcCovering`, with predefined type `ROOFING`. Revit's export of each file writes all 9 as `IfcBuildingElementProxy .NOTDEFINED.`, typed by an `IfcBuildingElementProxyType`. rvt-rs followed the value and wrote them as `IfcCovering`.
- The value is the wall sweep's own: for each of the 9, the data object that holds the entry (RE-153) and the element-data header before it (RE-44) are the same wall sweep's.
- Why Revit's exporter leaves the value unapplied is not established. revit-ifc `IFC_v24.3.40` exports a wall sweep through `WallSweepExporter.Export`, which calls `GenericElementExporter.ExportElement`. That takes the entity from `ExporterUtil.GetProductExportType`, the same lookup that reads `IFC_EXPORT_ELEMENT_AS` for other elements, and for `IfcCovering` hands the element to `CeilingExporter.ExportCovering`. It falls back to a proxy only when the lookup names no entity. The source would follow the value; Revit's export does not.
- A wall sweep in a Revit 2024 file now keeps its category's mapping, `IfcBuildingElementProxy .NOTDEFINED.`, whatever its value says, as RE-188 does for 2024 curtain walls. Other releases keep following the value: no wall sweep of another release that carries a value has been measured.

-----

## 1. Where the value sits

`probe_re45_ifc_export_parameters` prints every "Export to IFC As" entry with the owner each rule takes; `tools/re/export_parameters_vs_ifc.py` pairs each element with Revit's element of the same `Tag` (run [39185300259](https://github.com/DrunkOnJava/rvt-rs/actions/runs/39185300259), both office files):

| Own value | Revit wrote for that Tag | Elements |
|---|---|---:|
| `IfcCovering` | `IfcCovering` | 91 as built, 55 at building permit |
| `IfcCovering` | `IfcBuildingElementProxy` | 9 on each file |

The 9 are the office's wall sweeps (`OST_Cornices`, "Walls: Wall Sweeps"): 3318085, 3318255, 3318316, 3318379, 3320044, 3321175, 3321733, 3322083 and 3322685. Each entry's owner by the data-object rule and by the header rule is the wall sweep itself. At building permit one of the 9 holds a second copy of its entry in another partition, with the same value.

## 2. What Revit's exporter does with a wall sweep

Read in revit-ifc `IFC_v24.3.40` (CLEANROOM.md source 5; read and cited, no code copied):

- `WallSweepExporter.Export` returns at once for a reveal and otherwise calls `GenericElementExporter.ExportElement`.
- `GenericElementExporter.ExportElement` takes the export type from `ExporterUtil.GetProductExportType`, whose third step reads `IFC_EXPORT_ELEMENT_AS` on the element, then `IFC_EXPORT_ELEMENT_TYPE_AS` on its type.
- `FamilyInstanceExporter.ExportGenericToSpecificElement` hands an `IfcCovering` to `CeilingExporter.ExportCovering`; `ExportSimpleGenericElement` writes an `IfcBuildingElementProxy` only when the export type is unknown.

So by the source Revit would write these 9 as coverings, and its export has proxies. The likeliest reading is that `get_Parameter(IFC_EXPORT_ELEMENT_AS)` does not return the stored value on a wall sweep, but that is not shown here.

## 3. Results

Measure A/B against `main` (ccc7e45): office files and the reference and R2025 models in run [39187692379](https://github.com/DrunkOnJava/rvt-rs/actions/runs/39187692379), Autodesk's samples in run [39187694272](https://github.com/DrunkOnJava/rvt-rs/actions/runs/39187694272). Against Revit's export, by `Tag` (`class_sets_vs_ifc.py`):

| | main | RE-192 |
|---|---:|---:|
| As built: Revit's 12 `IfcBuildingElementProxy` written as proxies | 3 | 12 |
| As built: written as `IfcCovering` | 9 | 0 |
| At building permit: Revit's 12 `IfcBuildingElementProxy` written as proxies | 3 | 12 |
| At building permit: written as `IfcCovering` | 9 | 0 |

- **Nothing else changes.** On both office files the property values equal to Revit's, different from Revit's and missing stay as on `main` (as built 12,686, 36 and 44,669). The six reference models and the R2025 models export byte-identically, and so do all 64 hashed sample exports.
- **Not shown.** Revit types each of the 9 by an `IfcBuildingElementProxyType`; rvt-rs writes them untyped, as on `main`.

## 4. Reproduce

```bash
gh workflow run measure.yml -f ref=main -f base=none -f nordic=true -f probe=probe_re45_ifc_export_parameters -f probe_scorer=export_parameters_vs_ifc
```
