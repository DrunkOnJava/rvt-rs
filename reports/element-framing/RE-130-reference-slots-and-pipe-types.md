# RE-130: reference-list slots, and pipe types

**Date:** 2026-09-28
**Issues:** #228 (reference-list slots), #96 (MEP); resolves RE-129's negative
**Artefacts:** Core Interior (Revit 2024, MIT) and the four RE1 models (Revit 2025, MIT), each against Revit's own export. Every run is on GitHub's hosted runners with the Measure workflow; the last is 36505507226.
**Probe:** `examples/probe_re130_reference_slots.rs`; scored by `tools/re/reference_slots_vs_ifc.py`.
**Status:** positive for types. The storey slot is not measured.

## 1. The type is in the reference list

For every record whose element Revit exports with a type (IfcRelDefinesByType, whose type object's Tag is the type's ElementId), the scorer looks the type up in the record's reference list at `+0x88`:

| file | exported with a type | type in the list | not in it |
|---|---:|---:|---:|
| Core Interior | 854 | 669 | 185 (173 walls, 12 floors) |
| RE1 Architecture | 73 | 73 | 0 |
| RE1 Mechanical | 74 | 74 | 0 |
| RE1 Plumbing | 123 | 123 | 0 |
| RE1 Electrical | 49 | 46 | 3 |

The slot is not fixed. It depends on the category and on how many references the element has (for example `+1` on Core Interior's walls and doors, `+3` on its columns, `+3` or `+4` on RE1's pipes). The rule rvt-rs uses, the one listed id that carries a name entry of the element's category (`partition_names::resolve_type`), fits this.

## 2. Pipes' and ducts' types have no name entry

RE-129 found that pipes and ducts resolve no type, and tried to link them to the stored type names through their own data. Their types are in their reference lists after all: 191045 (`221116-WTR`) on 57 of Plumbing's 63 pipes, and 53292 (`233113-DUCT-Tees`) on all 25 ducts. None of them has a name entry.

The name sits in a layout of its own. On both files, and for all five pipe and duct types Revit exports, the type's ElementId occurs, then again 56 bytes later, and 306 bytes after the first a `u32 n` and `n` UTF-16 units: the type name. `partition_names::find_mep_curve_type_names` reads it. The list's first slot, the constant 3 (#228), also matches the pattern by chance (`"6"`), so it is skipped.

## 3. What changes

A pipe without a type takes the one id in its reference list with such a name, and its family is Pipe Types, Revit's one pipe system family. Run 36505507226, against `main`:

| | `main` | RE-130 |
|---|---:|---:|
| RE1 Mechanical pipes with Revit's Name and ObjectType | 0 of 6 | **6 of 6** |
| RE1 Plumbing pipes with Revit's Name and ObjectType | 0 of 63 | **62 of 63** |
| RE1 Plumbing elements typed as Revit types them | 60 of 123 | **122 of 123** |
| RE1 Mechanical elements typed as Revit types them | 43 of 74 | **49 of 74** |

Every name and Tag of the added type objects agrees with Revit's. No other scorer's output changes, and the Core Interior, Einhoven, RE1 Architecture and RE1 Electrical IFCs are identical to `main`'s. Ducts keep no type: their family (Rectangular, Round or Oval Duct) follows the type's shape, which is not read. The one rectangular duct type has `0x85` where the pipe types have `0x82` in the bytes before its name. That is one sample, not a rule.

## 4. Not measured

Storeys: neither RE1 nor Core Interior has name entries for its Levels, so the scorer's map from storey name to Level ElementId is empty and the storey slot is untested.
