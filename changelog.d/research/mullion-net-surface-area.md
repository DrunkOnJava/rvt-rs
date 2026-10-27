- **What a curtain mullion's `NetSurfaceArea` is (RE-248, #1016).** Revit's
  export sums the faces of the mullion's Revit solid: its section's perimeter
  times its length plus 0, 1 or 2 ends, the ends that are not joined to
  another mullion. Listers and a ranking of the mullion records find nothing
  in the model that says which ends those are, so the value stays unwritten
  on all 10,003 of the BIM4LCA office's mullions; the next signal is the
  curtain wall's grid and its type's join condition.
  The curtain wall type's join condition (`m_autoJoinCond`) was read on
  2026-10-27: 0 on every mullion of the office, so it does not say either.
