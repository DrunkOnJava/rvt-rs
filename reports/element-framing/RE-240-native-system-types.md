# RE-240 — The type a pipe, wall sweep or ramp names in its own record

**Date:** 2026-10-26
**Issues:** #35 (parameters), W7 of [`docs/release-1.0-criteria.md`](../../docs/release-1.0-criteria.md); follows RE-69 (wall sweeps), RE-130 (pipe types), RE-230 (curtain wall type names)
**Artefacts:** the BIM4LCA office as built and at building permit (Revit 2024, Revit's IFC4 exports), RE1 Architecture, Mechanical, Plumbing and Electrical (Revit 2025, Revit's IFC2X3 exports), Core Interior (Revit 2024) and puzzbobb's R2025 models (Revit 2025, Revit's IFC4 exports), as Measure fetches them
**Runs:** Measure [40278868408](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40278868408) (`examples/probe_re240_untyped_types.rs` with `tools/re/untyped_types_vs_ifc.py`), [40281443280](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40281443280) (this change against `main` at c305931e) and [40283284163](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40283284163) (`tools/re/ramp_elements_list.py`)
**Result:** positive: on both office files every pipe, wall sweep and ramp rvt-rs wrote untyped takes Revit's type.

## 1. Before

On the office rvt-rs wrote 12 pipe segments, 9 building element proxies and 5 ramps with neither Revit's Name nor its ObjectType (`names_vs_ifc`), and so with no type object and no type `Reference` (`type_objects_vs_ifc`, `psets_vs_ifc`). Revit names them `Pipe Types:JSV-3:<id>`, `Wall Sweep:JSV-4:<id>` or `Wall Sweep:JSV-5:<id>`, and `Ramp:AP-ulkoluiska:<id>`. RE-130 reads a pipe type's name from a layout of its own measured on Revit 2025 files, and RE-69 types a wall sweep from its record's reference list on Snowdon Towers; neither types these.

## 2. Each record names its type

The probe lists every ElementId each such element's native record names, and the scorer finds which is the Tag of the type object Revit's export relates the element to:

| element | record | member naming the type | type record | as built | at permit |
|---|---|---|---|---:|---:|
| pipe | `RbsPipeCurve` | `m_idType` | `RbsPipeType` | 12 of 12 | 12 of 12 |
| wall sweep (written as a proxy) | `WallSweep` | `m_typeId` | `CorniceAttr` | 9 of 9 | 9 of 9 |
| ramp | `Ramp` | `m_rampAttrId` | `RampAttributes` | 4 of 4 | 4 of 4 |

The type record's `SymbolInfo.m_name` is the type's name in every case (`JSV-3`, `JSV-4`, `JSV-5`, `AP-ulkoluiska`). Revit's export relates 4 of the 5 ramps to a type object. The fifth, 1440596, it writes as an `IfcRamp` named `Ramp:AP-ulkoluiska:1440596` with ObjectType `Ramp:AP-ulkoluiska`, aggregating an `IfcRampFlight` of the same Tag named `Ramp:AP-ulkoluiska:1440596:1`, and relates neither to a type object; its record names the same type as the other four's (`ramp_elements_list`). revit-ifc (CLEANROOM.md source 5) writes a ramp as such a container where its geometry splits into more than one flight or landing (`RampExporter.IdentifyRampFlightAndLanding`), which rvt-rs does not read.

## 3. What rvt-rs does

`native_element_types::system_type_names` reads, in one pass over the file, the records of the pipes, sweeps and ramps that have no type yet and every record of the three type classes, and gives each element the type its member names where the type's record is of the matching class and has a name; an element or type whose copies disagree gets none. A pipe's and a ramp's family is then their system family (Pipe Types, Ramp), and a sweep's Wall Sweep, as RE-69 names it. The fifth ramp is related to its type object too: the type is the one Revit names it for, and only Revit's container export leaves the relation out.

## 4. Measured

This change against `main` (run 40281443280); only the two office exports move:

| | office as built, `main` | this change | at permit, `main` | this change |
|---|---:|---:|---:|---:|
| pipes, wall sweeps, ramps with Revit's Name and ObjectType (`names_vs_ifc`) | 0 of 26 | 26 of 26 | 0 of 26 | 26 of 26 |
| elements typed (`type_objects_vs_ifc`) | 12,578 of 12,624 | 12,603 | 12,623 of 12,669 | 12,648 |
| type name carried | 12,599 | 12,624 of 12,624 | 12,644 | 12,669 of 12,669 |
| typed in rvt-rs but not in Revit's export | 41 | 42 | 42 | 43 |
| Revit's property values equal (`psets_vs_ifc`) | 44,016 | 44,068 | 44,198 | 44,250 |
| different / rvt-rs's that Revit does not write | 56 / 504 | 56 / 504 | 56 / 504 | 56 / 504 |

`names_vs_ifc` counts 25 of the 26 names equal: it compares an element with the last of Revit's elements of its Tag, for 1440596 the flight; Revit's `IfcRamp` is named as rvt-rs names it. The 52 values are the types' `Reference` in `Pset_PipeSegmentTypeCommon` (12), `Pset_BuildingElementProxyCommon` (9), `Pset_RampCommon` (5) and `Pset_EnvironmentalImpactIndicators` (26). The one more element typed in rvt-rs than in Revit's export is ramp 1440596 (§2). Core Interior, Einhoven, the RE1 models and the R2025 models export byte for byte as before.

## 5. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=main -f base=none -f nordic=true -f r2025=true \
  -f probe=probe_re240_untyped_types -f probe_scorer=untyped_types_vs_ifc
```
