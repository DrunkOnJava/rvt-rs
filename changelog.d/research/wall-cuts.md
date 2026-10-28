- **Where Revit cuts a wall's top and base (RE-258, #1067).** On the BIM4LCA
  office, rvt-rs's walls differ from Revit's at one end only: 65 tops and 20
  bases as built. Revit cuts a top at the underside of a floor the wall's
  top join names, which gives 63 of the 65, but that rule also cuts 5
  walls Revit leaves whole, and nothing read tells them apart, so no wall
  is cut yet. `tools/re/wall_heights_vs_ifc.py` measures every wall's base
  and top against Revit's export: Core Interior's 360 and puzzbobb's 117
  R2025 walls all match.
