- **A railing's `PredefinedType` is its own or its type's (#1713).** rvt-rs
  wrote none on all 41 railings of the BIM4LCA office, where Revit's export
  writes `HANDRAIL` on 32, `GUARDRAIL` on 8 and `NOTDEFINED` on 1, one value per
  railing type. A railing now takes its own or its type's "IFC Predefined Type"
  where that is an `IfcRailingTypeEnum` enumerator, and stays unset where it has
  none, since what Revit writes for a railing with no value is not measured. A
  Revit 2027 save still writes none (#1692).
