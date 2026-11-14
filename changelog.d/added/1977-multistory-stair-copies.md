- **A multistory stair is written again on each storey its group repeats it
  on, as Revit's export writes it, on Revit 2024 (#1977, RE-463).** Where no set
  of a `MultistoryStairs` record covers the span between two consecutive Levels
  of its stairs, and the span is as high as the stair that ends where it
  begins, that stair is written once more with its flights, landing, stringers
  and railings, shifted up by the Level difference, in the storey of the span.
  The copy of the stair and of each part but a railing is named `<name>:2` and
  keeps its original's `Tag`; the stair, its flights and its landing have no type
  object and the flights are `NOTDEFINED`, as in Revit's export. On Autodesk's
  Snowdon Towers architectural the three multistory stairs gain the 53 entities
  Revit's export holds beyond rvt-rs's, with Revit's names, storeys and place.
  A copy's `GlobalId` is rvt-rs's own, derived from its original's, and its
  record set carries `MultistoryCopyOf`; the rule is inferred from three groups
  of one building, since the file does not say how far a stair repeats, and a
  span it does not reach is counted in the export notes, not copied.
