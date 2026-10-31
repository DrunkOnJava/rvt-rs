- **Type objects' common property sets (W7, RE-286, #1185).** Revit's IFC4
  export gives each type object the common set of its entity, with the
  properties its entries give the type itself. rvt-rs now writes them: the
  type's Fire Rating (RE-77), `LoadBearing` for the structural column and
  framing categories, and a stair type's and flight type's dimensions (a
  stair type's `NosingLength`, `NumberOfRiser` and `NumberOfTreads` at 0, in
  `Pset_StairFlightCommon` too with `--revit-set-placement`). These are equal
  to Revit's export on 34 of the 182 values it writes on the BIM4LCA office's
  type objects as built (29 of 154 at permit), on 34 on the R2025 models and
  on modelo_bim's column and beam types, none different and none that Revit's
  export leaves out. No type object gets `IsExternal`: the office's and the
  R2025 models' exporters write it, Revit's newer ones (the held-out models',
  RE1 Architecture's) do not, and nothing in a file says which exporter will
  read it. `IfcModel` gains `type_entity_sets`, by type and type entity,
  which the writer adds to the type object's `HasPropertySets` (code that
  builds an `IfcModel` with a struct literal adds
  `type_entity_sets: Default::default()`), and the diagnostics'
  `parameter_value_count` counts a type object's `Pset_` values. Core
  Interior's `IFCPROPERTYSET` count goes from 6,508 to 6,510.
