- **Which columns Revit's export gives a `Slope` (RE-233, #35).** A probe of
  columns' native records, and `tools/re/record_scalars_vs_ifc.py` now ranks
  record fields against whether Revit writes a property at all. No field
  tells the 73 of the BIM4LCA office's 279 columns Revit writes
  `Pset_ColumnCommon.Slope` on: revit-ifc takes it from the last object of
  each column's exported geometry.
