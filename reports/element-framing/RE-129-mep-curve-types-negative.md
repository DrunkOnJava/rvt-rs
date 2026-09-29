# RE-129: pipe and duct types, negative

**Date:** 2026-09-28
**Issue:** #96 (MEP); follows RE-35 (name entries) and RE-79 (MEP categories)
**Artefacts:** RE1 Mechanical and RE1 Plumbing (Revit 2025, MIT), each with its IFC2X3 CoordinationView export. Measured on GitHub's hosted runners with the Measure workflow; the last run is 36497990733.
**Probe:** `examples/probe_re129_mep_curve_types.rs`
**Status:** negative, superseded by RE-130. The type is in the reference list, where this report did not find it: it has no name entry, and its name is stored in a layout of its own.

## Why it matters

Revit's export names a pipe `Pipe Types:<type>:<ElementId>` and a duct `Rectangular Duct:<type>:<ElementId>`. rvt-rs names them `Pipe-<id>` and `Duct-<id>`, with no ObjectType: 69 pipes and 25 ducts on RE1 (the `Name differs` rows of `names_vs_ifc.py`). The type rule `partition_names::resolve_type` needs one id in the record's reference list that carries a name entry of the element's category.

## What was measured

1. **Name entries.** RE1 has 154 (Plumbing) and 155 (Mechanical) name entries, none in the pipe or duct category.
2. **Reference lists.** No id in any pipe's or duct's reference list (8 to 10 ids each) has a name entry. The ids every list shares are `3`, `1819`, and `50321` (Mechanical pipes) or `53292` (ducts).
3. **Where the type names are.** `221116-WTR`, the type of all 63 Plumbing pipes, occurs once, 2,328 bytes into the data of element 179830. `233113-DUCT-Tees` occurs once, 8,190 bytes into the data of element 434017. Neither is a name entry: no declared ElementId stands in the 64 bytes before either.
4. **The pipes' and ducts' own data.** Across all 63 Plumbing pipes, 6 Mechanical pipes and 25 ducts, the data never names 179830 or 434017. Other than small constants, the declared ids in at least 90% of them are `1819`, `1893`, `50321` (every Mechanical pipe) and `53292` (every duct). None of their data holds a type name.
5. **50321 is not a type.** Mechanical's six pipes use three types in Revit's export (SANWST-PVC ×4, WTR-Copper B, WTR-Copper D), yet all six name 50321.

## What was tried and withdrawn

- `system_family("Pipe") = "Pipe Types"`.
- Keeping a type's name where no family element names it.

Measured before and after (run 36496111432), neither changed any output on the six MIT models, because no pipe resolves a type in the first place. Both were withdrawn rather than shipped unmeasured.

## Next

The type is likely reached through an object the name entries do not cover: the element whose data holds the name (179830) may own the type as a sub-object, or the link is a `u32` or tagged reference rather than a `u64`. A file with one pipe per type, or a before/after pair where only a pipe's type changes, would isolate it.
