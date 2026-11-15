- **A ceiling in pieces takes the area of the piece Revit's export names
  (#1209, RE-486).** Revit's export gives a ceiling sketched in several pieces
  the area of the piece its body exporter creates last, and rvt-rs wrote none
  where RE-465's sketch rule cannot tell which piece that is. The saved mesh
  stores that piece first: the part of its top face that holds the face's
  first triangle, on all 78 ceilings in pieces of the BIM4LCA office. A ceiling
  in pieces with no `GrossArea` now takes that part's area, on its own
  covering: 16 values on the office, 5 cut ceilings among them, all equal to
  Revit's, with nothing else moving. The rule rests on one building
  (`docs/one-project-rules.md`).
