# RE-429: LoadBearing from the Structural check box (floors and stairs) and a wall's Structural Usage

**Date:** 2026-11-11
**Issues:** #1193 (W7: the BIM4LCA office's remaining property values), #1636 (the rule rests on the BIM4LCA models); follows RE-285 (structural beams' and columns' `LoadBearing`), RE-194 (parameters named for a property) and RE-158 (a type's Function)
**Artefacts:** the BIM4LCA housing and office, concrete and timber, as built and at building permit (Revit 2024), and Revit's IFC4 exports of them; RE1 Architecture; the 18 held-out models as counts only
**Probe and scorer:** `probes/examples/probe_re285_flag_candidates.rs`, `tools/re/native_flags_vs_ifc.py`
**Runs:** Measure [43025706580](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43025706580) (the probe on `main` at `a0dd7813`: saved parameters, floors and family instances), [43028230598](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43028230598) (the same, with the 2024 walls' class named) and [43030521879](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43030521879) (the integer members of a wall's root record)
**Status:** In progress. Floors and stairs: positive on every model Measure holds, and written by the change on this branch. Walls: the saved parameters of a 2024 wall hold no integer built-in parameter, so the wall's Structural Usage is looked for in its root record (section 4); not written yet.

## 1. The question

In the property census of the BIM4LCA models `Pset_WallCommon.LoadBearing` is missing on all 1,464 walls of the six housing files and on 25 walls of the offices, `Pset_SlabCommon.LoadBearing` on 131 slabs and `Pset_StairCommon.LoadBearing` on 116 stairs (Measure run 43018018669). rvt-rs writes the property only for the structural column and framing categories (RE-285) and where a parameter named for it is read (RE-194). Where does Revit's export take the other values from?

## 2. What Revit's source says

Autodesk's exporter (CLEANROOM.md source 5, read and cited, not quoted: `Source/Revit.IFC.Export/Exporter/PropertySet/Calculators/LoadBearingCalculator.cs`, class `LoadBearingCalculator`, the same logic at the tags `IFC_v24.2.20` and `IFC_v27.4.0.30`) calculates `LoadBearing` in this order: for a wall, whether its `StructuralUsage` is `Bearing` (not `Shear` or `Combined`); for the structural column, framing, framing system and truss categories, true; for any element that holds the built-in parameter `FLOOR_PARAM_IS_STRUCTURAL` as an integer, whether it is not 0; otherwise a parameter named for the property on the element or its type. A parameter named for the property in the file replaces the calculation, as it does every standard property (RE-194).

## 3. Floors and stairs

`FLOOR_PARAM_IS_STRUCTURAL` is the built-in parameter -1001954. The probe lists it as a saved integer parameter of the `Floor` and `FamilyInstance` records, and `native_flags_vs_ifc.py` holds its value against Revit's `LoadBearing` per IFC class. On the elements Revit's IFC4 export writes `LoadBearing` for:

| model | slabs | hold the parameter | value 1 | value 0 |
|---|---:|---:|---|---|
| housing, timber, as built and at permit | 34 | 30 | true on 19 | false on 11 |
| housing, concrete, as built | 35 | 30 | true on 23 | false on 7 |
| housing, concrete, at permit | 31 | 27 | true on 20 | false on 7 |
| the two terrain housing files | 2 each | 2 | true on 2 | |
| RE1 Architecture | 2 | 2 | | false on 2 |
| the 18 held-out models (counts only) | 110 | 110 | true on 12 | false on 98 |

Every slab that holds the parameter and no parameter named for `LoadBearing` has Revit's value in it: the parameter's value 1 goes with true and 0 with false, with no exception on any model. The slabs that do not hold it (4 or 5 per large housing file) already read equal through a parameter named for the property. The offices are the one place the parameter and Revit's value part: their 31 slabs that hold it (value 1 on 18 slabs Revit writes false for) also hold a parameter named `Pset_SlabCommon.LoadBearing`, which holds Revit's value on all 31 (true 8, false 23), and the named parameter wins in Revit's exporter as it does in rvt-rs.

The stairs of the four large housing files (29 each) hold the parameter with the value 0 and Revit writes false on all 116. No stair holds a checked box on any model, so a stair with the value 1 gets none: the rule writes what Revit's exports show and nothing the source alone would give. The offices' stairs hold no such parameter and are not changed.

## 4. Walls

The 2024 walls are class `SWall` with 455 elements holding saved parameters in the timber housing, and none of them holds an integer built-in parameter: the first two probe runs printed no wall line. The probe now also prints the integer and Yes/No members of a wall's root record (run 43030521879) and the scorer holds each against Revit's `LoadBearing`, true on 111 of the timber housing's 367 walls.

## 5. The rule written

`attach_structural_load_bearing` (`src/research/partition_schema_mvp/parameters.rs`) reads `FLOOR_PARAM_IS_STRUCTURAL` from the element's own data object on Revit 2024 and later (`scan_integer_parameter`, as a type's Function is read, RE-158) for the slabs and products. 0 is false for any element; 1 is true for a floor alone. An element that has a `LoadBearing` from its category keeps it, and a parameter named for the property replaces the value when the sets are written.

## 6. What is not measured yet

The Measure of the change (what moves, element by element on the kept pairs, the census before and after), the wall rule, and Revit 2015 to 2023, whose saved parameters the probe reads but whose element data this rule does not.

## 7. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=a0dd7813e1635284a95150eb7e2c2faf7d4621b6 -f base=none -f nordic=true -f r2025=true -f holdout=true -f models="nordic-* r2025-* re1-* core-interior holdout-*" -f scorers=none -f probe=probe_re285_flag_candidates -f probe_scorer=native_flags_vs_ifc
```
