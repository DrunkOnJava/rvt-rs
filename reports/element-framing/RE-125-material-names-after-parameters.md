# RE-125 — Material names kept after a run of parameter entries

**Date:** 2026-09-28
**Issues:** #355 (materials), #421 (Revit 2026); follows RE-58 (material names) and RE-124
**Artefacts:** the flowbim.ee house (Revit 2026, sha256 `eeff78f3d07dc23397d276f870f42240cb453cfdd39d965531a9d0ccc7d0a654`) with Revit 26.4's IFC4X3 export; `2024_Core_Interior.rvt` (MIT) with its slim IFC4 export; Snowdon Towers Architectural (Revit 2024, local only) with its IFC4 export; RE1 Architecture (Revit 2025, MIT).
**Result:** positive. A material whose name field holds shared-parameter entries (a manufacturer's product, supplier, fire class, URL, classification code, unit) keeps its name after them, as a bare `u32 n · UTF-16` string closed by eight zero bytes, a `u64` ElementId and `0xff`×8. None of RE-58's four rules reads it. Read as a last resort, it names every material object on all four files.

## 1. The observation

The 2026 house's materials carry the Estonian CCI classification as shared parameters. Material 699393's object holds, in order:
- `Fibo 5`, `Saint-Gobain Weber Ehitustooted AS`, `A1 (EN 13501-1)`, a product URL and `Weber Fibo 5`;
- the class `Wall plate from block units`, `ULM70`, `ULM70_01` and `Concrete`;
- `m2` (keyed by an ElementId);
- then `%ULM70 - Wall plate from block units`, followed by `00`×8, `u64` 699392 and `ff`×8.

The last is its name in Revit's export. The `ff ff ff ff` + `PhysicalParamSet` terminator of RE-58's second rule never follows it. 32 of the house's 148 material objects are named only this way. Core Interior's 8 unnamed materials (`Terrazzo`, `Chair Fabric`, `Material 2`, `4`, `4 (2)`, `6`, `8`, `10`) and Snowdon's 1 (`Sheathing Roof Board`) are too.

## 2. The rule and its risk

`partition_materials::name_at` tries it last. On all four files, the first string closed that way agrees with the earlier rules on 183 objects and disagrees on 9. In those 9 it lands on an earlier string of the same shape, such as an appearance's colour name (`NCS S 0502-Y`). So it never overrides a name another rule reads.

| file | material objects | named before | named now | new names in Revit's export's material list |
|---|---:|---:|---:|---|
| Revit 2026 house | 148 | 116 | **148** | 32 of 32 |
| Snowdon Towers (2024) | 220 | 219 | **220** | 1 of 1 |
| Core Interior (2024) | 86 | 78 | **86** | 0 of 8; the slim export lists only the 9 materials its elements use |
| RE1 Architecture (2025) | 69 | 69 | 69 | – |

`scan_material_names` still drops a name read for two materials, so a misread cannot duplicate one.

## 3. End to end

- **The 2026 house:** layer names equal Revit's, in Revit's order, on 183 layers (13 before). 65 of the 67 elements given materials have Revit's set (5 of 19 before).
- **Core Interior and Snowdon:** element material sets are unchanged (580 and 2,628 Revit's). Only their material lists grow, by 8 and 1. Core's project-count baseline moves from 78 to 86, and both Core witness observations are regenerated (PASS, three replay matches each).
- **Every other file:** byte-identical (RE1 Architecture and Electrical, Einhoven, and the four 255ribeiro projects).
