# RE-154: Revit's common property sets take Reference from the type name

**Date:** 2026-10-03
**Issues:** #35; backlog item B21
**Artefacts:** the four RE1 models (Revit 2025) with Revit's own IFC4 exports, and Core Interior (2024). Measured on GitHub's hosted runners with the Measure workflow, run 37133239742.
**Probe:** `examples/probe_re154_pset_parameters.rs`.
**Status:** mixed. Reference is the type name; FireRating is on the type, under an id one file cannot pick; the other text values are not stored text entries.

## 1. The question

Revit's IFC4 export writes standard property sets on the RE1 models' elements: `Pset_WallCommon`, `Pset_DoorCommon`, `Pset_DistributionFlowElementCommon`, `Pset_QuantityTakeOff` and others. Their text values (`IfcLabel`, `IfcText`, `IfcIdentifier`) come from parameters. Which ones does the file store, and where?

## 2. Method

The probe enumerates every verified data object of every partition in one forward pass (RE-153's object: a 20-byte header with a zero word, a payload ending in its size again, and an Adler-32 that verifies). It then takes every text entry `id · u32 n · UTF-16 × n` whose `i64` id is a BuiltInParameter (-1,000,000 to -2,000,000), with the innermost object holding it.

From Revit's export it takes every element with a numeric `Tag`, its type's `Tag` (`IfcRelDefinesByType`), and every text value of its property sets. Each value is looked for among the element's own entries and among its type's.

## 3. Result

| model | verified objects | objects with text entries | text values Revit writes |
|---|---:|---:|---:|
| RE1 Architecture | 41,714 | 3,998 | 152 |
| RE1 Mechanical | 21,728 | 1,953 | 148 |
| RE1 Plumbing | 22,723 | 1,833 | 247 |
| RE1 Electrical | 113,260 | 15,851 | 143 |
| Core Interior | 77,920 | 4,388 | 0 (the slim export writes no property sets) |

- **`Reference` is the element's type name.** No stored text entry holds it, on any model and in any set. Revit's values are the type names that the element's own `ObjectType` (`Family:Type`) ends in:
  - `84" x 94"` on a door;
  - `300x150` on an air terminal;
  - `Grohe WB265_M` on a basin;
  - `-` and `--` on RE1 Architecture's walls and floors, whose types are named `-` and `--` (`Basic Wall:-`, `Floor:--`);
  - `Single-Flush` on a door whose type is `Single-Flush:Single-Flush`.

  rvt-rs reads type names from the partition name entries (RE-38), not from a parameter.
- **`FireRating`** (`Pset_WallCommon` and `Pset_ConcreteElementGeneral`, 7 walls, all `2`) is found on the wall's type for 7 of 7. It sits under four ids at once: -1001206, -1001405, -1002500 and -1010103. A one-character value meets several unrelated entries by chance, so this file does not say which is Fire Rating. A model with a longer fire rating would settle it.
- **`Finish`** (`Pset_CoveringCommon`, `Moisture Resistant Gypsum Sheathing;`) and **`SerialNumber`** are not stored text entries of the element or its type. `Finish` looks assembled from the covering's layer materials (note the trailing `;`).

## 4. What this changes

`Reference` can be written from the type name rvt-rs already reads: on each element's `Pset_<Entity>Common`, and on `Pset_QuantityTakeOff`, as Revit does. That is the next B21 slice. FireRating waits for an oracle that tells its id apart.
