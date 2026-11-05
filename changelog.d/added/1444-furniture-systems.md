- **Furniture Systems are exported as `IfcSystemFurnitureElement` (#1444,
  RE-352).** The element-record sweep reads `OST_FurnitureSystems` under the
  unchanged instance rule on every release the exporter reads, and
  `category_map` writes the class Revit's own IFC4 export writes for it, with
  no `PredefinedType`, where it mapped `FurnitureSystem` to `IfcFurniture`
  before. The evidence is one project and 3 elements: Revit's public IFC4
  export of Autodesk's 2018 `rac_basic` sample holds 3
  `IfcSystemFurnitureElement`, all 3 placed Furniture Systems frames of that
  save are in it and none is written that it lacks (counts of ids, from
  Measure runs 42097940515 and 42108271073; the export has no licence and
  nothing is quoted from it). No other model in Measure places a Furniture
  System, so there is no held-out check. Not written: type objects,
  containment in spaces, Revit's property sets, a body other than the
  record's box. The exports this head moves are not yet measured.
