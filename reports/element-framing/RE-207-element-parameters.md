# RE-207 — An element's saved parameters, named by the file's definitions

**Date:** 2026-10-22
**Issues:** #35 (M4-07, the first slice); follows RE-194 (parameters named for a property set's property), RE-156 (project and shared parameter definitions), RE-185 (a window's Fire Rating) and the native record path of #548.
**Artefacts:** the BIM4LCA office, as built and at building permit, and the housing on terrain, each with Revit 2024's own IFC export (VTT, CC BY-SA 4.0; `research/bim4lca-measure.tsv`, `research/bim4lca-models.tsv`); puzzbobb's R2025 models (`R2025_Models.zip`, Apache-2.0, #356); Core Interior and the RE1 models (CI's project corpus).
**Probe and scorer:** `examples/probe_re207_element_parameters.rs` and `tools/re/element_parameters_vs_ifc.py`; the same values are what `rvt-elements` and Python's `decoded_elements()` list.
**Status:** positive for every value Revit's exports show: the parameters named for a property set's property, and a window type's Fire Rating. The other parameters are read and named, but the exports measured do not carry them (§5).

## 1. Before

`rvt-elements` and `decoded_elements()` gave each element a `parameters` list holding only AProperty* value carriers, empty for every building element ("until host↔parameter joins recover", #35). The native record path (`native_document::extract`) already projected each record's saved values (`native_metadata::SavedMetadata`) and the file's parameter definitions, but nothing joined them to the element, named them, or reached the CLI or Python.

## 2. Where an element's values are

Each value is in the element's own native record:

- **Typed parameter sets.** `Element`'s first four members are `m_pParamValueSetDouble`, `…Int`, `…AString` and `…ElementId`, each a set of `(m_paramId, m_value)` pairs. The set gives the storage type.
- **Family parameter slots.** A family instance's `m_pInstParams` and a family type's `m_pParams` point to a `FamilyParams` whose `m_params` hold one slot per family parameter, with a double, an integer, a string and an ElementId field each. The parameter's definition picks the slot (`native_parameter_definitions::Registry::resolve_slot`).
- **Native fields read as built-in parameters** (`native_metadata`'s `raw_field_parameters`): an instance's type, Level, host, phases and category.

A positive parameter id is named by its definition's `m_caption`. The definition owner gives its kind: `ParamElemFamily` a family parameter, `ParamElemExternal` a shared one, `ParamElemGlobal` a global one, the others project parameters. A built-in parameter's negative id has no definition in the file, and the built-in catalog is not shipped (#255), so it keeps its id and no name.

One definition class was not read: `ParamDefFamType`, a family type parameter, had no storage type, so its slots were not resolved. Its value is the slot's `m_elemId`, the ElementId of a family type of the definition's `m_categoryId`. On R2025 model 12 the window type's Muntin Pattern, Exterior Trim and Interior Trim hold 12362, 12372 and 12382, the Generic Model types `Muntin Pattern_2x2`, `with Sill` and `Picture Frame` of the families `Muntin Pattern_2x2`, `Trim-Window-Exterior-Flat` and `Trim-Window-Interior-Flat`. It is now read as an ElementId.

## 3. Instance and type

A family instance's type is its `m_masterSymbolId`. The type's `FamilyParams` also holds a slot for each of the family's instance parameters; its value there is the family's default, not the instance's. On R2025 model 12 the window type 12499 holds 32 slots: 9 of instance parameters (definition `m_instanceParam` true), 11 of type parameters, 12 of built-ins. Window 12538 holds 6 of those 9 instance parameters itself.

So an instance's list is its own values, then its type's values for the parameters it does not hold, leaving out the family's instance parameters. Each value carries `level` (`instance` or `type`) and the record it came from. The three instance parameters the window holds no value for (Wall Thickness, Extension Jamb, Extension Jamb Visibility) are counted, not listed.

## 4. Units

Values are as saved: lengths in feet, areas and volumes in square and cubic feet, angles in radians, which are Revit's internal units. A Double whose spec is a length, area, volume or angle also gets its SI value. A Yes/No (`autodesk.spec:spec.bool`) is a boolean. A value naming a material gets the material's `Material.m_name`; one naming a family type gets the type's `SymbolInfo.m_name` and its family's `Family.m_name`.

## 5. Measured

`rvt-elements MODEL.rvt`, then `tools/re/element_parameters_vs_ifc.py elements.json REVIT.ifc` (IfcOpenShell 0.9.0; CI pins 0.9.0.post1). The scorer pairs each element with Revit's of the same Tag (an opening sharing its door's or window's Tag left out). It takes every property Revit writes on the element, its own sets first, then its type object's. Each named parameter is matched to the property of the same name, `<set>.<property>` or plain. A Yes/No or integer is compared with a Boolean or integer, a text with a string, and a length with its SI value after the export's own length unit. As RE-194 found, a parameter named for a property is the value Revit writes there.

| model (licence) | paired | equal | differ |
|---|---:|---:|---:|
| BIM4LCA office, as built (CC BY-SA 4.0) | 7,606 | 7,606 | 0 |
| BIM4LCA office, building permit | 7,592 | 7,592 | 0 |
| BIM4LCA housing on terrain, building permit | 27 | 27 | 0 |
| R2025 12 to 17 (Apache-2.0), each | 3 | 3 | 0 |

The office's pairs are 42 parameters named `Pset_*` and one named `FireExit`, 13 properties in all, on walls, curtain walls, windows, doors, columns, slabs, stairs, ramps, railings, beams and coverings: Booleans (`IsExternal`, `LoadBearing`, `IsLaminated`, `IsTempered`, `FireExit`, `HandicapAccessible`), texts (`FireRating`, `AcousticRating`), counts (`NumberOfRiser`, `NumberOfTreads`), lengths (`RiserHeight`, `TreadLength`) and thermal transmittance. All are on the element itself.

On the R2025 models each of the three windows' `Pset_WindowCommon.FireRating` (`0.5h`) is the family type parameter `Fire Rating` of its type. That is a value the window reaches only through its type (`level: type`, from 12499 on model 12).

Revit takes a parameter named for the property first. Four office windows hold `Pset_WindowCommon.FireRating` = `EI30` and a type `Fire Rating` that is empty. Revit writes `EI30`, so `Fire Rating` is paired only where the element has no parameter named for `FireRating`.

The office as built also gives 22,787 named values whose name Revit's export does not write on that element. Those are family, shared and project parameters other than the ones above, and parameters whose property the export leaves out (RE-194 §4). Core Interior (against `IFC Exports/2024_Core_Interior_slim.ifc`), the RE1 models and R2025 models 12 to 17 give 1,312, 303 to 1,304 and 134 such values each, and nothing to pair. Their exports were made without Revit's own property sets ("Export Revit property sets"), which would write every parameter under its own name. Those values are read and named but not measured. An export of the R2025 models with Revit's property sets would measure them.

`tests/element_parameters.rs` asserts the office's and the housing's values (Booleans, integers, texts and unitless numbers: 7,086 and 27) and the R2025 windows' type Fire Rating. Without the type merge the R2025 test fails, 0 of 3.

## 6. What is listed

| model | elements with values | values | named | built-in, unnamed | family slots not resolved |
|---|---:|---:|---:|---:|---:|
| R2025 model 12 | 75 | 860 | 137 | 723 | 61 |
| RE1 Architecture | 154 | 2,009 | 314 | 1,695 | 175 |
| RE1 Electrical | 93 | 2,319 | 1,304 | 1,015 | 390 |
| RE1 Mechanical | 114 | 1,959 | 971 | 988 | 157 |
| RE1 Plumbing | 126 | 3,011 | 787 | 2,224 | 232 |
| Core Interior | 1,056 | 11,800 | 1,312 | 10,488 | 1,092 |
| BIM4LCA housing on terrain | 696 | 7,775 | 367 | 7,408 | 172 |
| BIM4LCA office, as built | 13,776 | 176,827 | 31,925 | 144,902 | 11,799 |
| BIM4LCA office, building permit | 13,190 | 169,783 | 31,507 | 138,276 | 11,033 |

These counts are from the probe's first line, over the elements `rvt-elements` lists. A type's unresolved slots count once for each of its instances listed, and almost all of them are built-ins.

`rvt-elements`' other fields are unchanged on every model compared with `main` (`b6086d4`). Its `parameters` keep their AProperty* entries first, and the saved parameters follow.

## 7. Not read

- **Built-in parameters' names.** The file holds no definition for a negative id, and the catalog that would name them is not shipped (#255). They keep their id.
- **A family's built-in slots.** A window type's Width (−1001301) and Height (−1001300) sit in its `FamilyParams` with every slot field filled. Nothing in the file says which field holds the value, and none is guessed.
- **A system family's type parameters.** A wall's or floor's type (`BasicWallType`, `FloorAttributes`) is not joined to the element yet.
- **Evaluated values.** A formula's or a reporting parameter's value is the one saved, not recomputed.
- **Releases before 2023.** The native record path reads Revit 2023 to 2027 only.
- **The IFC export.** Nothing new is written to IFC. The values Revit writes from parameters named for a property set are already written (RE-194).

## 8. Cost

Reading the saved parameters decodes each listed element's native record and its type's, so `rvt-elements` takes longer. With hyperfine (`-N --warmup 1 --runs 3`) on a Raspberry Pi 5, `rvt-elements MODEL.rvt --no-typed`, `main` (`b6086d4`) against this change:

| model | `main` (s) | this change (s) | ratio |
|---|---:|---:|---:|
| RE1 Architecture | 1.385 ± 0.011 | 1.691 ± 0.002 | 1.22 |
| Core Interior | 6.143 ± 0.012 | 7.338 ± 0.026 | 1.19 |
| Snowdon Towers Architectural (2025 copy, local only) | 18.922 ± 0.052 | 27.230 ± 0.160 | 1.44 |
| BIM4LCA office, as built | 21.570 ± 0.145 | 32.310 ± 0.023 | 1.50 |

On the office, `native_document::extract`'s fixed cost (the definition pass and the partition walk) is 1.3 s. Decoding the 13,776 listed elements' records takes 8.9 s, most of it the 11,605 family instances. Adding the types', materials' and families' records costs under 1 s, so it is the instance records themselves.

`rvt-ifc` does not read them. Its output is unchanged apart from the export timestamp on Core Interior, RE1 Architecture and Electrical, R2025 model 12, the office as built and the housing on terrain.
