- **A wall's curve length in rvt-rs's own set (#1046, RE-252).** Each wall
  drawn from its centreline carries `RvtElementRecordGeometry.CurveLength`,
  its native curve's length. `tools/re/qtos_vs_ifc.py` compares it with the
  cut-back curve Revit's export draws as the wall's `Axis`, and counts a
  missing `Length` apart where the two differ, since Revit's export writes
  either: 461 of the BIM4LCA office's walls as built and 388 at permit. It
  does so only on a model where Revit's `Length` is one of the two on every
  wall, so no curve was read wrong.
