# RE-158: a door's and a floor's IsExternal is its type's Function

**Date:** 2026-10-03
**Issues:** #35; backlog items B28 and B32
**Artefacts:** the four RE1 models (Revit 2025) with Revit's own IFC4 exports. Measured on GitHub's hosted runners with the Measure workflow, run 37144392154.
**Probe:** `examples/probe_re158_value_entries.rs`.
**Status:** positive for door and floor `IsExternal`; inconclusive for the walls' `IsExternal`, `LoadBearing` and `ExtendToStructure`, and for `NumberOfPoles`.

## 1. The question

RE-153, RE-154 and RE-156 found where the text values of Revit's property sets are: an entry `id · u32 n · UTF-16` in the element's own data object. Revit's export also writes booleans (`IsExternal`, `LoadBearing`, `ExtendToStructure`, 44 values on RE1 Architecture) and integers (`NumberOfPoles`, 2 on RE1 Electrical). Where are they?

## 2. Method

For each element of the reference export with a boolean or integer property, and for that element's type, the probe lists every BuiltInParameter id (-1,000,000 to -2,000,000) in their verified data objects whose first or second `u32` after it is 0 to 1,000. For each `(set, property)` it prints the `(place, id, word)` whose value equals Revit's on every element that has one. For each project or shared parameter definition (RE-156) it also prints the 16 bytes after each occurrence of its id.

## 3. Result

| set and property | elements | Revit's values | candidates in every element |
|---|---:|---|---|
| `Pset_DoorCommon.IsExternal` | 5 | false and true | the type's -1001006, first word: 5 of 5 |
| `Pset_SlabCommon.IsExternal` | 2 | false | the type's -1001006, first word: 2 of 2, among 4 |
| `Pset_WallCommon.IsExternal`, `LoadBearing`, `ExtendToStructure` | 7 | false | 7 candidates, -1001006 not among them |
| `Pset_MemberCommon.IsExternal` | 10 | false | 1 |
| `Pset_CurtainWallCommon`, `Pset_RailingCommon`, `Pset_PlateCommon` `IsExternal` | 1, 1, 2 | false | 6, 8 and 0 |
| `Pset_ElectricalDeviceCommon.NumberOfPoles` | 2 | 1 | none |

- **-1001006 is `FUNCTION_PARAM`, "Function"** (Revit API, `BuiltInParameter`): a type's Interior (0) or Exterior (1). It is stored as `i64 id · u32 value` in the type's own data object. It is the only candidate that separates the doors' two values, so a door's `IsExternal` is its type's Function. The floors agree with it, though their values are all false.
- **Every other property is false on every RE1 element** (or 1 on both `NumberOfPoles`). A value that never changes agrees with any zero, or any 1, near it, so these candidates do not identify a parameter. The wall types carry no -1001006 entry with a small value; where a wall type's Function is kept is not found here.
- `Number of Poles` (987675, RE-156) is not among the definitions the name lookup returns: two definitions carry that name with different ids, so the lookup drops it. Its value is not found.

## 4. What this changes

A door's and a floor's `IsExternal` can be written from its type's Function (B28). The walls' booleans, `LoadBearing`, `ExtendToStructure` and `NumberOfPoles` need a model where they take both values.

## 5. Reproduce

```text
gh workflow run measure.yml --ref re/re158-value-entries -f base=none -f probe=probe_re158_value_entries
```
