- **The IfcProject description no longer names an internal decoder.** On
  every file not saved by Revit 2023 it read "Revit 2024 export; ArcWall
  standard decoder skipped: Revit 2024 is outside supported versions
  [2023]", a note about one of rvt-rs's readers that told a user of the IFC
  nothing about the model. It is now "Revit 2024 export" and the family's id
  where there is one.
  `rvt-ifc --placeholder`'s description no longer names internal work
  phases; it says the file holds the project only.
