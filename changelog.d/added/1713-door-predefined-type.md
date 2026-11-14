- **A door takes its own or its type's "IFC Predefined Type" where that is an
  IFC4 enumerator, as Revit's export writes it (#1713).** The wall hatches of
  the BIM4LCA files, 3 on each housing file and 10 on each office file, are
  `TRAPDOOR` in Revit's export; rvt-rs wrote `DOOR` on every door. A door with
  no value keeps `DOOR`.
