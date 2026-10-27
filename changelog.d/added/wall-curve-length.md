- **A wall's curve length in the export diagnostics (#1046, RE-252).**
  `rvt-ifc --diagnostics` lists each exported wall's native curve length
  (`decoded.wall_curve_lengths_feet`, by ElementId); the IFC file does not
  change. `tools/re/qtos_vs_ifc.py` compares it with the cut-back curve
  Revit's export draws as the wall's `Axis`, and counts a missing `Length`
  apart where the two differ, since Revit's export writes either: 461 of the
  BIM4LCA office's walls as built and 388 at permit. It does so only on a
  model where Revit's `Length` is one of the two on every wall, so no curve
  was read wrong.
