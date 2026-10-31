- **Type objects' common property sets (W7, RE-286, #1185).** Revit's IFC4
  export gives each type object the common set of its entity with the
  properties its entries give the type itself. rvt-rs now writes them:
  `IsExternal` from the type's own parameter, its Function (external unless
  interior), a roof's, else not external where the class needs no Function;
  the type's Fire Rating (RE-77); `LoadBearing` for the structural column and
  framing categories; and a stair type's and flight type's dimensions (a stair
  type's `NosingLength`, `NumberOfRiser` and `NumberOfTreads` at 0, in
  `Pset_StairFlightCommon` too with `--revit-set-placement`). On the BIM4LCA
  office, 161 of the 182 values Revit's export writes on the type objects
  rvt-rs writes as built (135 of 154 at permit) and 107 on the R2025 models
  are equal to its, none different. A window type, or a type whose Function
  is not read, gets no `IsExternal`. RE1 Architecture's IFC2X3 export by
  Revit 26.2 writes no `IsExternal` on a type, so the 8 of its types rvt-rs
  writes it on count as values Revit's export does not write (whether that is
  the exporter's version or its view is not measured). `IfcModel` gains
  `type_entity_sets`, by type and type entity, which the writer adds to the
  type object's `HasPropertySets` (code that builds an `IfcModel` with a
  struct literal adds `type_entity_sets: Default::default()`), and the diagnostics' `parameter_value_count`
  counts a type object's `Pset_` values. Core Interior's `IFCPROPERTYSET`
  count and its witness observations move with them.
