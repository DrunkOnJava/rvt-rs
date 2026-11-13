- **A beam or brace is in the storey of its Reference Level, which its record's
  second reference list holds (#1842, #1864, RE-454).** The storey was bound from
  the record's first reference list alone, so on each BIM4LCA timber office file
  63 beams were in a storey other than Revit's export puts them in: 47 that name
  two Levels in the first list, where the lower one was taken and Revit's storey
  is the highest, and 16 whose first list names a Level below the one Revit
  uses. A framing record now takes its Level from the second list where that
  names any, else the first, the one Level it names or the highest of several,
  as the `Reference Level` property already did (RE-174, RE-175). The Level is
  the same on the 2024 and the 2025 to 2027 saves of Autodesk's `rac_advanced`
  and `rst_basic`, where the first list's changes for 10 `rst_basic` beams.
  Other classes, and records that name three or more Levels, are as before.
