- **`rvt-elements` and Python's `decoded_elements()` list each element's
  saved parameters (#35, RE-207).** On Revit 2023 to 2027 files, an element's
  `parameters` now also hold the values its native record saves: its typed
  parameter sets, the family parameters of its own `FamilyParams`, and the
  native fields read as built-in parameters, each named by the file's own
  definition (family, shared, project and global parameters) with its
  storage type, spec, group and value. A family instance's type parameters
  follow its own, marked `"level": "type"`; a family's instance parameter is
  taken only from the instance. Lengths, areas, volumes and angles also get
  their SI value, a Yes/No value is a boolean, and a value naming a material
  or a family type also gets that element's name. Built-in parameters keep
  their negative id and no name. A family type parameter (`ParamDefFamType`,
  a window's Muntin Pattern or trim) is now read as the ElementId it holds.
  On the BIM4LCA office (Revit 2024), every parameter named `<set>.<property>`
  holds the value Revit's export writes for that property on the element:
  7,606 of 7,606 as built and 7,592 of 7,592 at building permit; on
  puzzbobb's R2025 models a window's `Pset_WindowCommon.FireRating` is its
  type's `Fire Rating`, 18 of 18. `tools/re/element_parameters_vs_ifc.py`
  scores either output against Revit's export. Nothing new is written to IFC.
  The library gives the same entries through
  `elements::parameters::saved_parameters_lossy`. By puzzbobb (#887).
