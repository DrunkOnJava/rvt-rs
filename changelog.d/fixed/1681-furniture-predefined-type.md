- **A furniture element's `PredefinedType` is Revit's (#1681, RE-419).** rvt-rs
  wrote none on 104 of the 107 furniture of each concrete BIM4LCA housing file
  and 103 of the 106 of each timber one, where Revit's export writes `NOTDEFINED`
  on 89 or 90, `CHAIR` on 9 and `BED` on 6 or 4. An element now takes its own or
  its type's "IFC Predefined Type" where that is an `IfcFurnitureTypeEnum`
  enumerator and, on Revit 2024, 2025 and 2026 files, `NOTDEFINED` where it
  has none: the held-out pairs of those releases write it on every furniture, and
  the two Revit 2027 pairs write no `PredefinedType` at all, so a 2027 file keeps
  none, and a 2022 file keeps none too, since its export parameters are not read.
  A value that is no enumerator stays unset, since revit-ifc's releases differ on
  it.
