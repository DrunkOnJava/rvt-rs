- **Furniture Systems are exported as `IfcSystemFurnitureElement` (#1444,
  RE-352).** The element-record sweep reads `OST_FurnitureSystems` under the
  unchanged instance rule on every release the exporter reads, and
  `category_map` writes the class Revit's own IFC4 export writes for it, with
  no `PredefinedType`, where it mapped `FurnitureSystem` to `IfcFurniture`
  before. The evidence is one project and 3 elements: Revit's public IFC4
  export of Autodesk's 2018 `rac_basic` sample holds 3
  `IfcSystemFurnitureElement`, all 3 placed Furniture Systems frames of that
  save are in it and none is written that it lacks, it writes no property
  set on them (so none is written) and it contains all 3 in a space (so an
  element of the class on a storey is contained in the room that holds it;
  one of the 3 has no Level and stays in the building). These are counts of
  ids from Measure runs 42097940515 and 42108271073; the export has no
  licence and nothing is quoted from it. No other model in Measure places a
  Furniture System, so there is no held-out check. Not written: type
  objects, Revit's property sets, a body other than the record's box. Not
  exercised: nested components of the class's own category and the
  design-option and phase filters. Measure run 42116433579 (`8bf4e3f2`)
  moves 13 of Autodesk's 58 samples' exports, every `rac_basic` save of 2015
  to 2027, each by 3 `IfcSystemFurnitureElement`; the other 45 are byte for
  byte main's. The models, the held-out files, the written ids, the bodies
  and the same-project check are not yet measured.
