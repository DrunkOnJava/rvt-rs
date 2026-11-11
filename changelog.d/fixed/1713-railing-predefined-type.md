- **A railing's `PredefinedType` is its own or its type's (#1713).** rvt-rs
  wrote none on any railing. A railing now takes its own or its type's "IFC
  Predefined Type" where that is one of IFC4's `IfcRailingTypeEnum` enumerators:
  on the BIM4LCA office and housing files 360 railings read as Revit's export
  writes them and none differs. A railing with no such value stays unset:
  Revit's 2024 export writes `NOTDEFINED` on the 8 of those files, and its 2027
  exporter leaves the attribute off, so nothing is written there without a
  measurement per release. A Revit 2027 save still writes none (#1692).
