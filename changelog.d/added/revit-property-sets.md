- **`rvt-ifc --revit-property-sets` writes Revit's own property sets (#35,
  RE-210).** With the option, an export writes each element's named saved
  parameters (RE-207) as Revit's exporter does with "Export Revit property
  sets": one set per parameter group, named for the group (`Dimensions`,
  `Identity Data`, `Materials and Finishes`, ..., `Other` for none), and the
  type's parameters in such sets on its type object, or on its elements with
  `(Type)` added where it has none. Following revit-ifc's `Release_25.x.x`
  branch, the IFC group is left out, a Yes/No is a Boolean, an integer with
  a spec an integer, a non-empty text a text, a length, pipe or duct size,
  area, volume, angle or number its measure, an air flow, HVAC or piping
  pressure, apparent or electrical power, electrical potential or current
  its measure in SI units (whose units the export then declares), a
  material its name and a family type `<family> : <type>`. Built-in
  parameters, enumerations and other specs are left out and counted.
  Against Revit 2025's export of Autodesk's Snowdon Towers HVAC with
  "Export Revit property sets" (measured locally, not committed), all
  28,906 properties written (counted per element) are in the set of the
  same name with Revit's value (`tools/re/revit_property_sets_vs_ifc.py`);
  Revit writes 79,907 more, nearly all built-in parameters. On Snowdon
  Architectural, 58,346 of 58,733 are; Revit names the sets of 13 of its
  type objects with `(Type)`, which follows from the order of its export. On the BIM4LCA office
  as built, `FireExit`, written in `Identity Data`, equals Revit's
  `Pset_DoorCommon` and `Pset_WindowCommon.FireExit` on 498 of 498 doors and
  windows. Without the option the export is unchanged.

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
