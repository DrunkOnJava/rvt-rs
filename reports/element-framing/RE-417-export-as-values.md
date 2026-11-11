# RE-417: the "Export to IFC As" values the BIM4LCA housing proves: IfcSpaceType, IfcFurniture, IfcSolarDevice and IfcTransportElement

**Date:** 2026-11-10
**Issues:** #1636 (the repairs RE-402 listed), #792 (the entity an "Export to IFC As" value names); follows #763 (`IfcSpace`), RE-45, RE-188, RE-190 and RE-402 section 5
**Artefacts:** the BIM4LCA housing, the four large files (concrete and timber, as built and at building permit) and its two terrain files (Revit 2024; Revit 24.0.5's IFC4 export of each, CC BY-SA 4.0, as Measure fetches it)
**Runs:** Measure [42935556262](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42935556262) (`probe_re45_ifc_export_parameters` and its scorer `export_parameters_vs_ifc` on the six files, `main` at `56821a14aec98f9fc8c4f1bba2d35c2f91738ad2`); the unfiltered run of the change follows
**Status:** In progress. The class sets of RE-402 section 5 are the evidence; the proof run and the change's unfiltered Measure are pending.

## 1. What RE-402 left open

On each of the four large housing files Revit's export writes 47 elements as another class than their category's, through their "Export to IFC As" value, and rvt-rs writes them as proxies or as a roof (`class_sets_vs_ifc`, paired by `Tag`; spaces by `GlobalId`):

| Revit writes | elements per large file | rvt-rs writes | Revit's `PredefinedType` | rvt-rs's value read ("Export to IFC As", class) |
|---|---:|---|---|---|
| `IfcSpace` | 42 (`Turning Free Space`) | `IfcBuildingElementProxy` | | `IfcSpaceType` on `GenericModel` |
| `IfcFurniture` | 3 | `IfcBuildingElementProxy` | `USERDEFINED` (type `IfcFurnitureType USERDEFINED`) | `IfcFurniture` on `GenericModel` |
| `IfcSolarDevice` | 1 | `IfcRoof` | `SOLARPANEL` (type `IfcSolarDeviceType SOLARPANEL`) | `IfcSolarDevice` on `Roof` |
| `IfcTransportElement` | 1 | `IfcBuildingElementProxy` | `ELEVATOR` (type `IfcTransportElementType ELEVATOR`) | `IfcTransportElement` on `SpecialtyEquipment` |

The diagnostics of each export count the same values as `Export to IFC As` values the export does not apply (#792): `IfcSpaceType` on `GenericModel` 42, `IfcFurniture` on `GenericModel` 3, `IfcSolarDevice` on `Roof` 1, `IfcTransportElement` on `SpecialtyEquipment` 1, and noise for values that name the type object of the entity the element is written as already (`IfcFurnitureType` on `Furniture` 15, `IfcWallType` on `Wall` 3, `IfcRoofType` on `Roof` 1). The 42 proxies carry `Pset_BuildingElementProxyCommon.IsExternal` and `.Reference`, which Revit's export omits (RE-402 section 4: 46 of each on every large file).

## 2. The change

Four values are acted on, each limited to the class it was seen on (`category_map::lookup_export_override_for`): `IfcSpaceType` (a family instance written as `IfcSpace`, as #763 does for `IfcSpace`), `IfcFurniture`, `IfcSolarDevice` and `IfcTransportElement`. Revit's exporter takes the entity from the value on any product (`ExporterUtil.GetExportType` restricts it to `IfcProduct`, revit-ifc, CLEANROOM.md source 5), so the class limit is what has been measured. The three entities that only a value reaches take their own predefined type where it is an enumerator of the entity, and an element whose category already maps to the entity keeps its default. A value that names the type object of the entity an element is written as (`IfcFurnitureType`) no longer counts as an unapplied value.

## 3. Measured

Pending.
