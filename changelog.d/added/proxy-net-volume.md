- **A building element proxy drawn from a saved mesh of flat faces gets the
  `Qto_BuildingElementProxyQuantities.NetVolume` Revit's export writes (#1016,
  RE-387).** The mesh Revit saved for the element is the solid Revit's
  `NetVolumeCalculator` measures, so its volume is Revit's where no face of that
  solid is curved, whose tessellation the mesh would then be; a proxy whose body
  is a tessellated profile, and the proxy's `NetSurfaceArea`, whose faces a join
  may hide from Revit's own area (RE-387), get nothing.
