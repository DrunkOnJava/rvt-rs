- **Furniture Systems are exported as `IfcSystemFurnitureElement` (#1444,
  RE-352).** The element-record sweep reads `OST_FurnitureSystems` under the
  unchanged instance rule on every release the exporter reads, and
  `category_map` writes the class Revit's own IFC4 export writes for it, with
  no `PredefinedType`, where it mapped `FurnitureSystem` to `IfcFurniture`
  before. The evidence is one project and 3 elements: Revit's public IFC4
  export of Autodesk's 2018 `rac_basic` sample holds 3
  `IfcSystemFurnitureElement`; the 3 written for that save are its 3 by `Tag`,
  none extra and none missing, with record boxes equal to Revit's bodies to
  0.0000 ft (counts and distances from Measure run 42135914864; the export has
  no licence and nothing is quoted from it). It writes no property set on them,
  so no `Reference` is written, and it contains all 3 in a space: rvt-rs
  contains 2 of the 3 in a space on Revit 2024 and later only, as for every
  furniture class, and one of the 3 has no Level and stays in the building.
  Revit's export types all 3; rvt-rs types none. In that run 13 of 111 exports
  move, every `rac_basic` save of 2015 to 2027, each by 3
  `IfcSystemFurnitureElement` and nothing else; 98 are byte for byte main's,
  and the same-project gates pass. No other model in Measure places a
  Furniture System, so there is no held-out check. Not exercised: nested
  components of the class's own category and the design-option and phase
  filters.
