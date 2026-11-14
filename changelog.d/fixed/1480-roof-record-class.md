- **The wall top and base passes tell a roof by its Revit record, not by the
  IFC entity it is exported as (#1480).** A wall is cut at a roof only where
  the roof is a level plate (RE-351); a floor exported as `IfcRoof` now keeps
  its cut, and a roof exported as `IfcSlab` is held to the plate rule.
