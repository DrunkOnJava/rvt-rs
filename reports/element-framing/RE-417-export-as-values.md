# RE-417: the "Export to IFC As" values the BIM4LCA housing proves: IfcSpaceType, IfcFurniture, IfcSolarDevice and IfcTransportElement

**Date:** 2026-11-10
**Issues:** #1636 (the repairs RE-402 listed), #792 (the entity an "Export to IFC As" value names); follows #763 (`IfcSpace`), RE-45, RE-188, RE-190 and RE-402 section 5
**Artefacts:** the BIM4LCA housing, the four large files (concrete and timber, as built and at building permit) and its two terrain files (Revit 2024; Revit 24.0.5's IFC4 export of each, CC BY-SA 4.0, as Measure fetches it)
**Runs:** Measure [42935556262](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42935556262) (`probe_re45_ifc_export_parameters` and its scorer `export_parameters_vs_ifc` on the six files, `main` at `56821a14aec98f9fc8c4f1bba2d35c2f91738ad2`) for section 2, and [42936466431](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42936466431) (unfiltered, head `c74d3887bde363aa383dbfe2a316e6b92aabfe19` against `main` at `56821a14`, every flag, 126 models each side) for section 4
**Status:** Positive, as registered on #1636 (comment 6783894703): the four registered rows hold on the four large housing files, and 122 of 126 exports are byte-identical, every model with a Revit export but the housing among them. The 47 elements of each file are written in Revit's class with Revit's `PredefinedType`, the 42 spaces draw their bodies from their saved meshes within Revit's area and height, and the values rvt-rs wrote that Revit omits fall from 163 to 168 to 27 to 32 on each file. Three things the new elements expose are listed in section 5.

## 1. What RE-402 left open

On each of the four large housing files Revit's export writes 47 elements as another class than their category's, through their "Export to IFC As" value, and rvt-rs writes them as proxies or as a roof (`class_sets_vs_ifc`, paired by `Tag`; spaces by `GlobalId`):

| Revit writes | elements per large file | rvt-rs writes | Revit's `PredefinedType` | rvt-rs's value read ("Export to IFC As", class) |
|---|---:|---|---|---|
| `IfcSpace` | 42 (`Turning Free Space`) | `IfcBuildingElementProxy` | | `IfcSpaceType` on `GenericModel` |
| `IfcFurniture` | 3 | `IfcBuildingElementProxy` | `USERDEFINED` (type `IfcFurnitureType USERDEFINED`) | `IfcFurniture` on `GenericModel` |
| `IfcSolarDevice` | 1 | `IfcRoof` | `SOLARPANEL` (type `IfcSolarDeviceType SOLARPANEL`) | `IfcSolarDevice` on `Roof` |
| `IfcTransportElement` | 1 | `IfcBuildingElementProxy` | `ELEVATOR` (type `IfcTransportElementType ELEVATOR`) | `IfcTransportElement` on `SpecialtyEquipment` |

The diagnostics of the concrete as built count the same values as `Export to IFC As` values the export does not apply (#792): `IfcSpaceType` on `GenericModel` 42, `IfcFurniture` on `GenericModel` 3, `IfcSolarDevice` on `Roof` 1, `IfcTransportElement` on `SpecialtyEquipment` 1, and noise for values that name the type object of the entity the element is written as already (`IfcFurnitureType` on `Furniture` 15, `IfcWallType` on `Wall` 3, `IfcRoofType` on `Roof` 1). The 42 proxies carry `Pset_BuildingElementProxyCommon.IsExternal` and `.Reference`, which Revit's export omits (RE-402 section 4: 46 of each on every large file).

## 2. What Revit's export says for each value

`probe_re45_ifc_export_parameters` lists each element's own "Export to IFC As" and each type's "Export Type to IFC As", and `export_parameters_vs_ifc` finds what Revit wrote for the same `Tag` (run 42935556262, the six housing files). An element exports as its own value, else its type's with the trailing `Type` dropped (`ExportParameters::effective_export_as`, RE-45), which is how the three furniture and the roof reach their values:

| value | read from | class of the element | Revit writes | elements on each large file | files |
|---|---|---|---|---:|---|
| `IfcSpaceType` | the element's own value | `GenericModel` | `IfcSpace` | 42 of 42 | 4 of 4 |
| `IfcFurnitureType` | the type's value (a type Revit writes as `IfcFurnitureType`, typing 3 `IfcFurniture`) | `GenericModel` | `IfcFurniture` | 3 of 3 | 4 of 4 |
| `IfcSolarDeviceType` | the type's value (a type Revit writes as `IfcSolarDeviceType`, typing 1 `IfcSolarDevice`) | `Roof` | `IfcSolarDevice` | 1 of 1 | 4 of 4 |
| `IfcTransportElement` | the element's own value | `SpecialtyEquipment` | `IfcTransportElement` | 1 of 1 | 4 of 4 |

The same files hold a value Revit does not act on, which the class limits keep out: on the concrete at permit one floor type's value is `IfcFurnitureType`, and Revit writes the 4 floors it types as `IfcSlab` (an `IfcSlabType`), so `IfcFurniture` is acted on for generic models and for no floor. Two elements whose own value is `IfcCovering` are `IfcFlowSegment` in Revit's export on every large file (the gutters, `IfcPipeSegmentType GUTTER`): a value Revit ignores, not covered here.

## 3. The change

Four values are acted on, each limited to the class it was seen on (`category_map::lookup_export_override_for`): `IfcSpaceType` (a family instance written as `IfcSpace`, as #763 does for `IfcSpace`), `IfcFurniture`, `IfcSolarDevice` and `IfcTransportElement`. Revit's exporter takes the entity from the value on any product (`ExporterUtil.GetExportType` restricts it to `IfcProduct`, revit-ifc, CLEANROOM.md source 5), so the class limit is what has been measured. The three entities that only a value reaches take their own predefined type where it is an enumerator of the entity, and an element whose category already maps to the entity keeps its default. A value that names the type object of the entity an element is written as (`IfcFurnitureType`) no longer counts as an unapplied value.

## 4. Measured

Unfiltered Measure 42936466431, 126 models on each side: 122 exports are byte-identical and 4 move, the four large housing files. No Autodesk sample, no office file, no terrain file, no Revit 2025 model and no held-out or IFC2X3 model moves, so none carries one of the four values on the class it is limited to.

The registered row, on the concrete and timber files as built and at permit:

1. **Classes** (`class_sets_vs_ifc`): Revit's `IfcSpace` absent from rvt-rs's 42 to 0, rvt-rs's `IfcBuildingElementProxy` that Revit's export lacks 42 to 0, Revit's `IfcFurniture` written as another class 3 to 0, `IfcSolarDevice` 1 to 0 and `IfcTransportElement` 1 to 0, on all four files. No class that was right before is written as another class after.
2. **PredefinedType** (`predefined_types_vs_ifc`, extended in this change): `same` for all 47 on every file, `differs` 0, unset in rvt-rs 0: the 42 spaces `INTERNAL`, the 3 furniture `USERDEFINED`, the solar device `SOLARPANEL`, the transport element `ELEVATOR`.
3. **Census:** `different` does not rise on any line, before and after, the concrete as built reading properties 1 different (`Pset_StairCommon.IsExternal`, RE-412) and quantities 2, the other three files 1 and 6, 1 and 2, and 1 and 2. The values rvt-rs writes that Revit omits fall from 167, 163, 168 and 163 to 31, 27, 32 and 27: `Pset_BuildingElementProxyCommon.IsExternal` and `.Reference` on the 46 proxies these elements were (46 of each to none), the solar device's `Pset_RoofCommon` pair, and 42 of the 54 `Pset_EnvironmentalImpactIndicators.Reference` (to 12). Equal values rise by 87 on each file (5,766 to 5,853 on the concrete as built), among them `Pset_FurnitureTypeCommon.Reference` 104 to 107 of 107 and the 42 spaces' `Pset_SpaceCommon.Reference` and `IsExternal`. The quantities Revit holds and rvt-rs does not write rise by 84 on each file (1,638 to 1,722 on the concrete as built), the `Qto_SpaceBaseQuantities.GrossVolume` and `NetVolume` of the 42 spaces; equal quantities do not move.
4. **Everything else byte-identical:** 122 of 126.

Other scorers move the same way: `space_profiles_vs_ifc` pairs 145 of 145 spaces (103 and 42 only in Revit's before), and the family-instance spaces drawn from saved meshes read 43 (1 before), area within 0.1% on 43, height within 0.01 ft on 43 and outline within 0.01 ft on 42; `global_ids_vs_ifc` reads spaces 145 of 145 equal one of Revit's (103 of 103), the elements rvt-rs writes that Revit's export lacks 53 to 11; `tagged_spaces_vs_ifc` reads name, long name, object type, `PredefinedType`, type object, `Pset_SpaceCommon.Reference` and `IsExternal` equal on 43 of 43; `type_objects_vs_ifc` reads the entity agreeing on 1,326 of 1,329 types (1,321) and the elements typed in rvt-rs's export and not in Revit's 54 to 12. The diagnostics' "Export to IFC As value the export does not apply" warning (66 elements on the concrete as built) is gone on the housing, and its space kinds read `family_instance` 43 (1).

## 5. What the new elements expose

- **Furniture's `PredefinedType`.** The scorer now reads `IfcFurniture`'s, which `predefined_types_vs_ifc` did not: on every file rvt-rs writes none on the 103 or 104 furniture that were already right in class where Revit writes `NOTDEFINED` on 89 or 90, `CHAIR` on 9 and `BED` on 6 or 4. The elements with an `IfcFurnitureType` value (15 on the concrete files, 13 on the timber files) are as many as the `CHAIR` and `BED` (15 and 13), and the `NOTDEFINED` is Revit's default. It is outside this change and is not repaired here.
- **Space quantities.** Revit writes `GrossVolume` and `NetVolume` on the 42 spaces; rvt-rs writes no `GrossVolume` on any space and a `NetVolume` only where a room's prism gives one (RE-296, #1207): 84 more missing quantities on each file.
- **The relation.** Revit's export puts none of the 42 spaces in a storey (`in nothing`); rvt-rs aggregates each in the storey it reads, as it does the one other family-instance space of the housing and the office's 44 (in a storey, or in the building). `tagged_spaces_vs_ifc` and `room_names_vs_ifc` count it: 43 of 43.

## 6. Not shown

- **The other classes Revit writes through a value on the housing.** The terrain files' 23 `IfcGeographicElement` written as furniture, proxies and slabs and their 42 absent (toposolids and bicycle posts), 3 `IfcFlowTerminal` written as proxies; the large files' 3 `IfcFlowSegment` (two gutters Revit writes as `IfcFlowSegment` although their own value says `IfcCovering`, one pipe) and 1 `IfcCovering` Revit writes for a wall; and the 11 sanitary terminals of the `Suihku 2:SU-2` type that Revit's export lacks.
- **Another project.** The four values are proven on the housing alone; the offices carry `IfcSpace` and not `IfcSpaceType` on these discs, and no Autodesk sample carries a value on the classes the limits name. The limits are what has been measured, not what Revit does.
- **A Revit 2025 or later value on these classes.** The housing is Revit 2024.

## 7. Reproduce

```bash
gh workflow run measure.yml --ref fix/1636-export-as-values -f ref=c74d3887bde363aa383dbfe2a316e6b92aabfe19 -f base=56821a14aec98f9fc8c4f1bba2d35c2f91738ad2 \
  -f nordic=true -f samples=true -f r2025=true -f holdout=true -f bimcollab=true -f racadv=true
gh workflow run measure.yml --ref main -f ref=56821a14aec98f9fc8c4f1bba2d35c2f91738ad2 -f base=none -f nordic=true \
  -f models="nordic-housing-concrete-as-built nordic-housing-concrete-permit nordic-housing-timber-as-built nordic-housing-timber-permit nordic-housing-terrain-concrete nordic-housing-terrain-timber" \
  -f probe=probe_re45_ifc_export_parameters -f probe_scorer=export_parameters_vs_ifc -f scorers=none
```

The head of the first is kept under `refs/archive/2026-11-10/cited/c74d3887bde3`.

## 8. Sources

RE-45, RE-188, RE-190, RE-402, #763, #792; revit-ifc's `ExporterUtil.GetExportType` and `GetIFCExportElementParameterInfo` (CLEANROOM.md source 5), read for the entity group an override is restricted to; no code was copied. No Autodesk source was read.
