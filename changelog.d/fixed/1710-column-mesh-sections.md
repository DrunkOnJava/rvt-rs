- **A column drawn as its type's rectangle is drawn from its saved mesh where
  that mesh is a vertical prism of another section (#1710).** The BIM4LCA
  housing's chamfered concrete columns and the thin-walled struts of Autodesk's
  `rme` samples were solid rectangles; their bodies and base quantities now come
  from the mesh Revit saved. A column whose mesh section is its rectangle keeps
  the rectangle, Revit's own form for a plain section.
