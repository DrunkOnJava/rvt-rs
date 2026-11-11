- **A wall's, ramp's, light fixture's and pipe segment's `PredefinedType` is
  its own or its type's (#1713).** rvt-rs wrote none on every ramp, light
  fixture and pipe segment, and `NOTDEFINED` on 3,842 walls where Revit's
  export writes `PARTITIONING`, `SOLIDWALL`, `SHEAR`, `PARAPET`,
  `PLUMBINGWALL`, `STRAIGHT_RUN_RAMP`, `SECURITYLIGHTING` or `GUTTER`. Each
  now takes its own or its type's "IFC Predefined Type" where that is an
  enumerator of its IFC4 entity. A wall with no value keeps `NOTDEFINED`,
  which is what Revit writes there, and a ramp, light fixture or pipe segment
  with none stays unset, since Revit writes `NOTDEFINED` on some of them and
  no 2027 export of the three is measured. A Revit 2027 save still writes
  none on a wall (#1692).
