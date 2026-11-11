- **A curtain wall whose record holds another curve besides its location curve
  gets its `Qto_CurtainWallQuantities.Length` (#1016, RE-252).** rvt-rs read a
  curtain wall's location curve only where its native record held that curve
  alone, so 7 of the BIM4LCA office's 220 curtain walls carried no `Length`
  where Revit's export writes one on all 220. It now takes the curve the
  record's `VWallDriver.m_pCrv` names, whatever other curves the record holds,
  and still none where that curve is an arc.
