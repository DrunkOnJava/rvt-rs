# RE-156: SerialNumber is a shared parameter's text entry in the element's own data object

**Date:** 2026-10-03
**Issues:** #35; backlog item B21
**Artefacts:** the four RE1 models (Revit 2025) with Revit's own IFC4 exports, and Core Interior (2024). Measured on GitHub's hosted runners with the Measure workflow, runs 37139488894, 37140018345 and 37140355491.
**Probe:** `examples/probe_re156_value_sources.rs`.
**Status:** positive for `SerialNumber`; negative for `Finish`.

## 1. The question

RE-154 looked for each text value of Revit's property sets among the entries `i64 id · u32 n · UTF-16 × n` whose id is a BuiltInParameter (-1,000,000 to -2,000,000), and found none for `Pset_ManufacturerOccurrence.SerialNumber`, `Pset_PrecastConcreteElementGeneral.SerialNumber` or `Pset_CoveringCommon.Finish`. On RE1 Electrical that leaves 50 values missing, the largest gap on that model. Where are they?

## 2. Method

For every `(set, property)` Revit writes, the probe takes up to three distinct values of three or more characters and looks for each in every stream of the file, inflated where it inflates and raw otherwise, as UTF-16LE and as UTF-8. For each hit it prints the `u32` before the text, the `i64` before that when the `u32` is the text's length, and the innermost Adler-32-verified data object holding it (RE-153). For each positive id found before a length-prefixed value, it prints the length-prefixed strings in that id's own data objects. It also looks for each property's name.

## 3. Result

- **`SerialNumber` is a text entry whose id is a shared parameter's ElementId**, inside the element's own data object, in the same layout as a BuiltInParameter entry:
  - RE1 Electrical: `G2342` is found 10 times in `Partitions/114` (and 10 copies in `Partitions/119`), one per light fixture of type 439260 that Revit gives it, each inside that fixture's object (441149, 441150, ...). `G2343`, `H1116`, `H2274` and `H3955` are found the same way. Every hit is preceded by the id 490488 and the length.
  - RE1 Plumbing: `AO9487503-2` on element 446093, preceded by the id 447886.
- **The id is the parameter's definition**, a data object of its own whose strings are the parameter group, the name and, for a shared parameter, its GUID:

  | model | id | strings |
  |---|---:|---|
  | RE1 Electrical | 490488 | `autodesk.parameter.group:identityData-1.0.0`, `Serial Number`, `revit.local.shared:0d1007705af740f5990de496876dfe28-1.0.0` |
  | RE1 Plumbing | 447886 | the same three, with the same GUID |
  | RE1 Electrical | 987675 | `autodesk.parameter.group:electrical-1.0.0`, `Number of Poles` (no GUID) |

  The property's own name, `SerialNumber`, is in neither file in either encoding: Revit's export matches the parameter named `Serial Number` to the property `SerialNumber`. The two definitions with a GUID have class word `0xc12` and the one without `0xc13`; that these are `SharedParameterElement` and `ParameterElement` is inferred, not read from the schema.
- **`Finish`** (`Moisture Resistant Gypsum Sheathing;`) is not stored as text anywhere in the file. RE-154's reading stands: the export assembles it from the covering's layer materials.
- `NumberOfPoles` is an `IfcInteger`, so it is not a text entry; where its value is stored is not measured here.

## 4. What this changes

`SerialNumber` can be read: find the definition named `Serial Number` (by its strings), then the text entries with its id in each element's own data object. Revit writes the value in `Pset_ManufacturerOccurrence` on every element that has one: on RE1 Electrical 12 `IfcFlowTerminal`s and 19 `IfcBuildingElementProxy`s, on RE1 Plumbing 1 `IfcFlowTerminal`. The same 19 proxies also carry it in `Pset_PrecastConcreteElementGeneral`, a set IFC4 defines for building elements, which a proxy is.

## 5. Reproduce

```text
gh workflow run measure.yml --ref re/re156-serial-numbers -f base=none -f probe=probe_re156_value_sources
```
