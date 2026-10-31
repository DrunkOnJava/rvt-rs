# RE-286: the common sets of type objects

**Date:** 2026-10-30
**Issues:** [#1185](https://github.com/DrunkOnJava/rvt-rs/issues/1185) and the types' part of [#1186](https://github.com/DrunkOnJava/rvt-rs/issues/1186), W7 of [`docs/release-1.0-criteria.md`](../../docs/release-1.0-criteria.md); follows RE-193, RE-210 and RE-269
**Artefacts:** the BIM4LCA office as built and at building permit (Revit 2024, CC BY-SA 4.0) with Revit's IFC4 exports (exporter 24.0.5.432), puzzbobb's 24 R2025 models (Revit 2025, Apache-2.0) with theirs (exporter 25.0.2.419), RE1 Architecture (Revit 2025, MIT) with its IFC2X3 export (exporter 26.2.0.20), and Core Interior, as Measure fetches them
**Runs:** Measure [41180394315](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41180394315) (`-f base=main -f nordic=true -f r2025=true -f holdout=true -f ifc_flags=--revit-set-placement`, every model and scorer, commit 7bbe15a7 against `main` aa917438), after [41138124029](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41138124029) and [41178463265](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41178463265) (the version that wrote types' `IsExternal`, §3)
**Status:** positive, narrowed: a type object's `FireRating`, `LoadBearing` and a stair or flight type's dimensions are written, equal to Revit's export wherever it writes them. A type object's `IsExternal` is not written, because Revit's exporters disagree on it (§3).

## 1. What Revit writes on a type object

Each type object (`IfcWallType` and so on) of Revit's IFC4 export of the office holds the common set of its entity with properties revit-ifc's entries give the type itself: `CreateElementTypeProperties` runs them on the `ElementType` (CLEANROOM.md source 5; read and cited, no code copied). The census scores them keyed by the type's `Tag` (#1018). On the office as built, the distinct types and what they hold:

| type entity (types) | `IsExternal` true / false | other properties |
|---|---|---|
| IfcWallType (26) | 24 / 2 | `FireRating` on 3 |
| IfcDoorType (41) | 11 / 30 | `FireRating` on 6, `ThermalTransmittance` on 1 |
| IfcSlabType (16) | 9 / 7 | |
| IfcCoveringType (19) | 1 / 18 | |
| IfcColumnType (11) | 0 / 11 | `LoadBearing` true on 10 |
| IfcBeamType (1) | 0 / 1 | `LoadBearing` true |
| IfcWindowType (6) | 0 / 6 | |
| IfcCurtainWallType (5) | 3 / 2 | |
| IfcBuildingElementProxyType (4) | 0 / 4 | |
| IfcStairType (4) | 0 / 4 | `NosingLength`, `NumberOfRiser` and `NumberOfTreads`, all 0, in `Pset_StairCommon` and `Pset_StairFlightCommon` on 3 |
| IfcStairFlightType (2) | | `NosingLength` 0 and 0.01 m in `Pset_StairFlightCommon` |
| IfcSpaceType (3 of 267) | 0 / 3 | |
| IfcMemberType, IfcPlateType | counted apart (RE-269) | |
| IfcRampType (1) | no set | |
| none for railings | | |
| family types (IfcSanitaryTerminalType, IfcLightFixtureType, IfcFurnitureType) | | `Pset_ManufacturerTypeInformation.Manufacturer` on 6 |

## 2. Revit's rules, read in revit-ifc

At `IFC_v24.2.0` and `IFC_v24.3.40`:

- **`IsExternal`**: the entry's calculator, `CategoryUtil.IsElementExternal`, is run on the type. At `IFC_v24.1.0` and `IFC_v24.2.0` it takes a Yes/No parameter named `IsExternal` on the type; else the type's `FUNCTION_PARAM`, external unless interior; else true for a roof; else false, which it caches under the type's id (the cache RE-269 describes). At `IFC_v24.2.20` and `IFC_v24.3.40` it returns null for an `ElementType`.
- **`LoadBearing`**: `LoadBearingCalculator` calls load bearing every element of the structural columns, structural framing, framing systems and trusses categories before it reads any parameter; the office's 10 structural column types and its framing type are such, the eleventh column type is not and holds none.
- **`FireRating`**: a type's Fire Rating parameter, where set.
- **Stairs**: `NumberOfRiserCalculator`, `NumberOfTreadsCalculator` and `NosingLengthCalculator` give a `Stairs` and a `StairsRun` their values and are invalid for anything else, yet Revit writes 0 on each stair type of a stair with data; where that comes from is not read. The rule is measured: 3 of 3 stair types on the office, one in each R2025 stair model.

## 3. Which exporters write a type's `IsExternal`

The BIM4LCA office's exports (24.0.5.432) and the R2025 models' (25.0.2.419) write it on type objects, and the source of `IFC_v24.3.40` returns null for a type. The newer exports measured write none: RE1 Architecture's (26.2.0.20, IFC2X3), which writes other type-level values (`FireRating` and `ThermalTransmittance` on its wall type), and the held-out models'. A first version of this change wrote each type's value as the two older exports do (the maintainer, 2026-10-30). The held-out run [41178463265](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41178463265) then counted values Revit's export does not write: 8 on the AA house (Revit 26.4's export), 3 on teste_export_2025, and 8 on RE1 Architecture. Nothing in a file tells which exporter will read it. So the maintainer decided on 2026-10-31 to write no `IsExternal` on a type object. On the office and R2025 exports those values are missing (127 as built, 106 at permit, 73 on R2025), not different.

## 4. What rvt-rs does

`append_typed_production_elements` collects, for each type and type entity, the common set of the entity with the type's Fire Rating (RE-77) where it has one and IFC4's set defines `FireRating`, `LoadBearing` true for a structural column or framing type, and for a stair type of a stair with data the three stair dimensions at 0 and for a flight type its nosing length. The writer adds the sets to the `HasPropertySets` of each type object of that type and entity (`IfcModel::type_entity_sets`). A type whose elements disagree gets none, and a set with no property is not written. With `--revit-set-placement`, a stair type's dimensions go into `Pset_StairFlightCommon` too (IFC4 applies it to flights). Not written: a type object's `IsExternal` (§3), a curtain mullion's or panel's type (RE-269), a railing's (the office's export writes no railing type object), a ramp's, and the entities rvt-rs writes no type object for. The diagnostics' `parameter_value_count` counts a type object's `Pset_` values.

## 5. Measured

`psets_vs_ifc`, head against `main`, run 41180394315 (commit 7bbe15a7, with `--revit-set-placement`):

| | office as built | office at permit | 24 R2025 models | RE1 Architecture | 3 held-out models |
|---|---:|---:|---:|---:|---:|
| equal, `main` / this change | 45,200 / 45,234 | 45,382 / 45,411 | 1,386 / 1,420 | 208 / 209 | 512 / 514 |
| different, both | 1 (the mullion whose `Span` differs) | 1 | 0 | 0 | 3 |
| missing, `main` / this change | 324 / 290 | 321 / 292 | 544 / 510 | 52 / 51 | 367 / 365 |
| `rvt-rs values Revit does not write`, both | 10 | 14 | 0 | 10 | 13 |

Equal values gained, each missing before. On the office as built: column and beam types' `LoadBearing` 11, wall types' `FireRating` 3, and the stair types' dimensions in `Pset_StairCommon` (`NosingLength`, `NumberOfRiser`, `NumberOfTreads`: 3 each), with `--revit-set-placement` in `Pset_StairFlightCommon` (3 each), plus the two flight types' `NosingLength`. At permit the same, with 6 `LoadBearing`. On the R2025 models: the five stair models' stair type dimensions and flight type. On the held-out models: modelo_bim's column and beam types' `LoadBearing` (Revit 26.2's export writes them). RE1 Architecture's wall type gains its `FireRating`. No value differs and no new value Revit does not write appears on any model. Core Interior's `IFCPROPERTYSET` goes from 6,508 to 6,510.

The stair types' zeros are measured on the 24.0.5 and 25.0.2 exports only: no held-out model has a stair type.

## 6. What is left out

- **Every type object's `IsExternal`** (§3).
- **A curtain panel's door type** (12611, written as an `IfcPlateType`): a curtain member's type (RE-269).
- **Doors' and door types' `FireRating`**: 6 door types ('EI60', 'EI30' and '-') and the two doors of one type whose rating is that type's '-'. A door's Fire Rating is a family type parameter in the family type tables RE-185 §3.3 describes, which RE-77's reader of built-in parameters does not read; the wall types' is built-in and is read.
- **`Pset_DoorCommon.ThermalTransmittance` on one door type**, and `Pset_ManufacturerTypeInformation.Manufacturer` on 6 family types: RE-289 (its probe found no entry of RE-77's form for 5 of the 6).
- **`Pset_BuildingElementProxyCommon.IsExternal` on the `IfcBuilding`** (false, in Revit's office export and its R2025 ones): revit-ifc matches the set to the building by the start of the entity's name, and it is another exporter-dependent type of value; not written.
- **A ramp's, railing's and curtain member's type** (§4).
- **A column's `LoadBearing`** on the one office column whose parameter is absent (3515145): an occurrence's value, #1184.

## 7. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=7bbe15a73246a37a011c987364ce070e3058d8d4 -f base=aa917438a1318654fc7f876dc03030cccd926b0a \
  -f nordic=true -f r2025=true -f holdout=true -f ifc_flags=--revit-set-placement
```
