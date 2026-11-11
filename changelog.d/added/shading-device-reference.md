- **A shading device carries its type's name as `Pset_ShadingDeviceCommon.Reference`,
  the set Revit's IFC4 export writes it in.** Revit's export of the BIM4LCA timber
  office as built writes that property on all 44 of its `IfcShadingDevice` and on no
  other element, and rvt-rs writes the same 44 shading devices, so the set joins the
  ones a Revit export confirms and the value is the type name rvt-rs already writes
  as `Reference` in every other confirmed set. `IsExternal`, which Revit's export
  also writes on those 44 and on their type, stays out: no rule read gives it.
