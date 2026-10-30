- **`psets_vs_ifc` scores each opening against the opening it corresponds to
  (W7).** Revit's export writes several openings with one `Tag`, the
  ElementId of the element they belong to: a door drawn as an opening, filled
  by its door, and, in each further wall or door that element cuts, an
  opening with no filling that holds `Pset_OpeningElementCommon.Reference`
  and `FireExit`. Keyed by `Tag` alone, the census scored those values
  against rvt-rs's filled opening, or against a curtain wall that shares the
  Tag, and listed them as missing: on the BIM4LCA office as built, 140
  `Reference` values and 31 `FireExit` values that sit on openings rvt-rs
  does not write (a curtain wall's 69 cuts, 39 fire hose cabinets' and 31
  door openings' cuts in further walls). An opening is now keyed by whether
  something fills it, and the values on openings rvt-rs does not write are
  counted apart, not dropped.
