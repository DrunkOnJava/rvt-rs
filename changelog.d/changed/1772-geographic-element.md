- **An element whose "Export to IFC As" names `IfcGeographicElement` is
  written as one (#1772).** On the BIM4LCA housing terrain model measured, 9
  furniture, 9 planting, 3 specialty equipment and 2 floor elements carry the
  value and Revit's export writes them as `IfcGeographicElement`, where rvt-rs
  wrote `IfcFurniture`, `IfcBuildingElementProxy` and `IfcSlab`; each now takes
  its own or its type's "IFC Predefined Type" where that is a `TERRAIN`,
  `USERDEFINED` or `NOTDEFINED` enumerator, and the two floors no longer carry
  the six `Qto_SlabBaseQuantities` values Revit does not write.
