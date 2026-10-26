- **Whether a wall keeps Revit's wall quantities (RE-251, #1016).** A probe of
  every number in each wall's native record and saved parameters finds none
  of Revit's wall areas or volumes on the BIM4LCA office: Revit computes them
  when it exports. A wall's `Height` is its saved parameter -1001101 on 496
  of 579 walls, but nothing read tells the other 83 apart, so it stays
  unwritten.
