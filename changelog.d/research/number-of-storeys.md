- **The storey count Revit's exporter writes (RE-263, #1097).** Up to IFC
  26.0 Revit's exporter writes `Pset_BuildingCommon.NumberOfStoreys` as the
  number of storeys that contain an element, and from IFC 26.1 as the number
  of storeys (24 of 24 and 5 of 5 reference exports). rvt-rs writes the
  number of storeys, IFC4's definition, only where every storey contains
  an element, where the two rules agree.
