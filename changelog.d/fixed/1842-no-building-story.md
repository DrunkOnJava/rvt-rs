- **A file whose Levels include no building story is written with no
  storey, as Revit's export writes it, instead of storeys made up from its
  elements' heights (#1842).** Every one of the 9 Levels of Autodesk's
  `rst_basic` sample, saved by Revit 2016 to 2027, has its Building Story
  setting off. rvt-rs then fell back to storeys named from record-box
  elevations ("Elevation -1.949 ft"), which no Revit export has. Revit's
  exporter makes no storey from such a Level and contains the file's elements
  in the building, and rvt-rs now does the same.
