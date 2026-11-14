- **On Revit 2027, a horizontal bracing's type object is an `IfcMemberType` with
  the `PredefinedType` `BRACE` (#1841),** as Revit's export writes it, where it
  was `NOTDEFINED`. The member's occurrence keeps no `PredefinedType`, as Revit
  2027's export leaves it off members.
