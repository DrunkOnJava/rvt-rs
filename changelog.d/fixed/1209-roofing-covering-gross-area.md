- **A family instance Revit writes as a covering takes its saved mesh's face
  area where the mesh is one part (#1209, RE-484).** Revit's export writes
  roof equipment such as ventilation units, snow guards and downpipe casings as
  `IfcCovering` `ROOFING` and gives each the area of the extrusion its body was
  exported as; rvt-rs wrote none. Where the saved mesh is one connected part
  and its largest upper and lower horizontal faces are each one piece of the
  same area, that area is Revit's `GrossArea`: 72 values on the BIM4LCA office
  and housing, equal to Revit's on all 72, with nothing else moving. A body of
  several parts (a vent's plate and duct) and one whose faces differ (a
  downpipe, which Revit gives none) keep none.
