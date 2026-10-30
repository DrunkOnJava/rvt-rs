# RE-275: a storey's Reference is its Level type's name, and its EntranceLevel is a Level parameter

**Date:** 2026-10-30
**Issues:** W7 ([release 1.0 criteria](../../docs/release-1.0-criteria.md))
**Artefacts:** the BIM4LCA office as built and at building permit (Revit 2024, CC BY-SA 4.0) and the 23 R2025 models (Revit 2025, Apache-2.0), each with Revit's own IFC4 export. Read on GitHub's hosted runners with the Measure workflow, run 41113408569.
**Probe:** `probes/examples/probe_w7_office_gaps.rs`.
**Status:** positive.

## 1. The question

Revit's IFC4 export writes `Pset_BuildingStoreyCommon` on each storey with `AboveGround` (unknown), which rvt-rs writes, and two properties it does not:

- `Reference`: `Korkomerkintä` on the office's 10 storeys as built and at permit, `Level 1` on each of the R2025 models' storeys (23 models, 110 storeys);
- `EntranceLevel`: on the office's 9 storeys, true on Level_01 and false on the 8 others, none on Sea Level.

Where does the file hold them?

## 2. Method

The probe lists the parameter definitions the file declares, and for every Level its native record, its saved parameters (`native_element_parameters::read`), and the record of the type the Level's `m_attrId` names.

## 3. Result

- **`Reference` is the name of the Level's type.** A `Level` record names its type in `m_attrId`, an ElementId: 505060 on all 10 office Levels in both files, 1945 on every R2025 Level. That id is a `LevelAttributes` record, whose `SymbolInfo.m_name` is `Korkomerkintä` and `Level 1` there. It is revit-ifc's `ReferenceCalculator` (CLEANROOM.md source 5), which takes the name of the element's type. All 110 R2025 Levels and the office's 20 give the name Revit writes. The same read gives a pipe's, wall sweep's and ramp's type (RE-240).
- **`EntranceLevel` is a yes/no parameter on the Level.** revit-ifc's definition of `Pset_BuildingStoreyCommon` gives `EntranceLevel` no calculator, so Revit's exporter reads a parameter named `Pset_BuildingStoreyCommon.EntranceLevel`, then `EntranceLevel`. The office declares the first as a shared parameter (id 3364458), and its Levels hold it: Level_01 true, Foundation, Basement, Level_02a_Parking, Level_02, Level_03, Level_04, Level_05 and Roof false, Sea Level none. That is Revit's 9 values, as built and at permit. No R2025 Level declares one, and Revit's R2025 exports write none.

## 4. What this changes

rvt-rs writes a storey's `Reference` from its Level's type, and, after it, its `EntranceLevel` from the Level's parameter where the Level holds one.

## 5. Left out

The building's `Pset_BuildingStoreyCommon.Reference` (`Project Information` in Revit's export, on the office and on each R2025 model) is revit-ifc matching property sets by the start of an entity's name: `IfcBuildingStorey` begins with `IfcBuilding`, so the set is applied to the building. IFC4 applies it to storeys only, so it belongs to `rvt-ifc --revit-set-placement` (RE-235), which writes building elements' sets only. The building's standard `Pset_BuildingCommon.Reference` is not written either.

## 6. Reproduce

```text
gh workflow run measure.yml --ref main -f ref=8193128ea56b7fb098c1472eacb0536b9fe09c32 -f base=none -f nordic=true -f r2025=true -f models="nordic-office-* r2025-*" -f scorers=none -f probe=probe_w7_office_gaps -f probe_scorer="-"
```
