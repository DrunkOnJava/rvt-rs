# RE-286: the common sets of type objects

**Date:** 2026-10-30
**Issues:** [#1185](https://github.com/DrunkOnJava/rvt-rs/issues/1185) and the types' part of [#1186](https://github.com/DrunkOnJava/rvt-rs/issues/1186), W7 of [`docs/release-1.0-criteria.md`](../../docs/release-1.0-criteria.md); follows RE-193, RE-210 and RE-269
**Artefacts:** the BIM4LCA office as built and at building permit (Revit 2024, CC BY-SA 4.0) with Revit's IFC4 exports (exporter 24.0.5.432), puzzbobb's 24 R2025 models (Revit 2025, Apache-2.0) with theirs (exporter 25.0.2.419), RE1 Architecture (Revit 2025, MIT) with its IFC2X3 export (exporter 26.2.0.20), and Core Interior, as Measure fetches them
**Runs:** Measure [41138124029](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41138124029) (`-f base=main -f nordic=true -f r2025=true -f ifc_flags=--revit-set-placement`, every model and scorer, commit 5d30bdb0 against `main` f73908bf)
**Status:** positive, with one gap Revit's own exporters disagree on: its newer exporter writes no `IsExternal` on a type object, and rvt-rs writes it (§3).

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

The BIM4LCA office's exports (24.0.5.432) and the R2025 models' (25.0.2.419) write it on type objects. RE1 Architecture's export (26.2.0.20, IFC2X3) writes other type-level values (`FireRating` and `ThermalTransmittance` on its wall type) and no `IsExternal` on any type, and the source of `IFC_v24.3.40` returns null for a type. Whether RE1's absence is the exporter's version or its IFC2X3 view is not measured. The maintainer decided on 2026-10-30 to write each type's value, as both IFC4 exports do, and rvt-rs writes IFC4. RE1 Architecture's types therefore carry values its export does not write (8 types, §5), and the census keeps them visible as `rvt-rs values Revit does not write`; no set-aside was added.

## 4. What rvt-rs does

`attach_type_is_external` calculates, for each element with a type, the value revit-ifc's calculator gives its type (`calculated_type_is_external`): a Yes/No parameter named `IsExternal` on the type, else the type's Function (external unless interior), else a roof, else not external for the classes whose Function is not required, and nothing for a wall, curtain wall, door, window, floor or building pad whose Function is not read (fail closed). `append_typed_production_elements` collects, for each type and type entity, the common set of the entity with that `IsExternal`, the type's Fire Rating (RE-77) where it has one, `LoadBearing` true for a structural column or framing type, and for a stair type of a stair with data the three stair dimensions at 0 and for a flight type its nosing length. The writer adds the sets to the `HasPropertySets` of each type object of that type and entity (`IfcModel::type_entity_sets`). A type whose elements disagree gets none. With `--revit-set-placement`, a stair type's dimensions go into `Pset_StairFlightCommon` too (IFC4 applies it to flights). Not written: a curtain mullion's or panel's type (RE-269), a railing's (the office's export writes no railing type object, though the R2025 models' write one with `IsExternal` false: 5 values left out so that the office's 7 are not extra), a ramp's, and the entities rvt-rs writes no type object for. The diagnostics' `parameter_value_count` counts a type object's `Pset_` values.

## 5. Measured

`psets_vs_ifc`, head against `main`, run 41138124029 (commit 5d30bdb0). That commit also wrote `IsExternal` on railing types, which the last commit takes out (§4, §6): its figures below include 7 such values on each office file, which Revit's office export has no type object for, and 5 equal ones on the R2025 stair models.

| | office as built | office at permit | 24 R2025 models | RE1 Architecture |
|---|---:|---:|---:|---:|
| Revit's values on objects rvt-rs writes | 45,525 | 45,704 | 1,930 | 260 |
| equal, `main` | 45,088 | 45,270 | 1,347 | 196 |
| equal, this change | 45,249 | 45,405 | 1,459 | 197 |
| different, both | 1 (the mullion whose `Span` differs) | 1 | 0 | 0 |
| missing, `main` / this change | 436 / 275 | 433 / 298 | 583 / 471 | 64 / 63 |
| `rvt-rs values Revit does not write`, `main` / this change | 10 / 17 | 14 / 21 | 0 / 0 | 10 / 19 |

By property, equal values gained on the office as built (the type objects' own values; each was missing before): wall types' `IsExternal` 26 of 26 and `FireRating` 3 of 3, door types' `IsExternal` 39 of 41, slab types' 15, covering types' 19, column types' 11 and `LoadBearing` 10 of 10, the beam type's `IsExternal` and `LoadBearing`, curtain wall types' 5, proxy types' 4, stair types' 4, space types' 3, and the stair types' dimensions in `Pset_StairCommon` (`NosingLength`, `NumberOfRiser`, `NumberOfTreads`: 3 each), with `--revit-set-placement` in `Pset_StairFlightCommon` (3 each) and the two flight types' `NosingLength`. At permit the same, with 38 door types, 25 wall types, 14 slab types, 6 covering types, and 6 column types with 5 `LoadBearing`. On the R2025 models: wall types 23, roof types 21, proxy types 18, door types 6, railing types 5, and the five stair models' stair type (`IsExternal` and the three dimensions in each set) and flight type. No value differs on any model.

RE1 Architecture's wall type gains its `FireRating`. The type values its export does not write, which the census keeps as `rvt-rs values Revit does not write`: 9 in run 41138124029 (a curtain wall type, 4 door types, a railing type, 2 slab types and a wall type), 8 once railing types are out. The reason is §3. Einhoven, R2025 model 00 and RE1 Plumbing are byte-identical; Core Interior and RE1 Electrical and Mechanical gain type sets with no Revit values to score.

## 6. What is left out

- **Window types' `IsExternal`** (6 on the office): Revit writes false on each. A window type holds no Function that rvt-rs reads, and a type whose Function is not read is no proof of one without it, so a window type gets none (fail closed). The same for door type 1107673 of the office.
- **A curtain panel's door type** (12611, written as an `IfcPlateType`): a curtain member's type (RE-269).
- **Doors' and door types' `FireRating`**: 6 door types ('EI60', 'EI30' and '-') and the two doors of one type whose rating is that type's '-'. A door's Fire Rating is a family type parameter in the family type tables RE-185 §3.3 describes, which RE-77's reader of built-in parameters does not read; the wall types' is built-in and is read.
- **`Pset_DoorCommon.ThermalTransmittance` on one door type**, and `Pset_ManufacturerTypeInformation.Manufacturer` on 6 family types: RE-289 (its probe found no entry of RE-77's form for 5 of the 6).
- **`Pset_BuildingElementProxyCommon.IsExternal` on the `IfcBuilding`** (false, in Revit's office export and its R2025 ones): revit-ifc matches the set to the building by the start of the entity's name, and it is another exporter-dependent type of value; not written.
- **A ramp's, railing's and curtain member's type** (§4).
- **A column's `LoadBearing`** on the one office column whose parameter is absent (3515145): an occurrence's value, #1184.
- **The exterior types are checked on the office and R2025 models only**; RE1's export is IFC2X3 by a newer exporter (§3).

## 7. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=af2ebc8e98e63394595ce8521d8d0b65763dc0d0 -f base=main \
  -f nordic=true -f r2025=true -f ifc_flags=--revit-set-placement \
  -f psets_missing="Pset_DoorCommon.IsExternal,Pset_WindowCommon.IsExternal,Pset_DoorCommon.FireRating,Pset_ColumnCommon.LoadBearing"
```
