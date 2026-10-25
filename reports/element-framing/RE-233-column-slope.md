# RE-233 — Which columns Revit's export gives `Pset_ColumnCommon.Slope`

**Date:** 2026-10-25
**Issues:** #35 (parameters), W7 of [`docs/release-1.0-criteria.md`](../../docs/release-1.0-criteria.md)
**Artefacts:** the BIM4LCA office as built and at building permit (Revit 2024, Revit's IFC4 exports), as Measure fetches them
**Runs:** Measure [40243309751](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40243309751) (`tools/re/psets_vs_ifc.py` listing the columns) and [40244629929](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40244629929) (`examples/probe_re233_column_record_scalars.rs` with `tools/re/record_scalars_vs_ifc.py`)
**Result:** negative: no field of a column's native record tells the columns Revit writes `Slope` on.

## 1. What is missing

Revit's export writes `Pset_ColumnCommon.Slope`, 0 on every one, on 73 of the office's 279 columns as built and 101 of 345 at building permit, and rvt-rs writes none. The 73 are not a family's: 13 of the 168 `Concrete-Square-Column`, 7 of the 31 `Concrete-Rectangular-Column` and 53 of the 79 hollow section columns.

revit-ifc's `SlopeCalculator` (CLEANROOM.md source 5) reaches a vertical column's slope through its last fallback: the slope of the largest upward face of the last object of the column's exported geometry, and nothing where that object has none. So whether Revit writes it depends on the order of the objects Revit's geometry gives, not on a parameter.

## 2. The record

The probe prints every scalar of the first 400 objects of each column's native record, and the scorer, which now also takes whether Revit writes a property at all (`present:<set>.<property>`) as a target, ranks every field by how well one of its values tells the 73 from the others.

| best fields | right, as built | at permit |
|---|---:|---:|
| `FamilyLevelRef#2.m_offset` = one value | 237 of 279 | 275 of 345 |
| `CoverSettingsCell.m_active` = false | 233 of 279 | |
| `FamilyLevelRef.m_offset` | 233 of 279 | 271 of 345 |

None is close. Every column's record holds a `CopyGeomToInstGStep`, so every one has geometry of its own rather than its symbol's; the counts of cut and clean steps after it (`CutPairGStep`, `CleanPairGStep`) do not separate them either: the four most common combinations, 261 of the 279 columns, each occur with `Slope` and without.

## 3. Conclusion

`Slope` is not written. What decides it is the order of the solids and curves in each column's exported geometry, which the record does not state; Revit's saved display graphics (RE-78) are the next place to look.

## 4. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=main -f base=none -f nordic=true \
  -f probe=probe_re233_column_record_scalars -f probe_scorer=record_scalars_vs_ifc
```
