- **Four of Autodesk's Snowdon Towers Architectural's six block roofs are drawn
  from their saved meshes (Revit 2024; #1683, RE-422).** Each stayed the level
  block of its sketch outline, 33% to 133% over Revit's volume, because a
  ruled-surface component was refused whenever one of its edges named fewer than
  two faces. The roofs' ruled faces are closed; the open edges are on the planes
  and the cylinder beside them, which cannot join two ruled faces, so a component
  is refused now only where a ruled face of it borders such an edge. The four are
  within 0.11% of Revit's volume with every vertex of Revit's body within
  0.000358 ft of their surface, in 198,762 triangles. A component the old guard
  refused takes a grid per face, and a face between two straight profiles a
  quarter of the cells the shared grid asked (a bilinear patch leaves its two
  triangles by one quarter of its twist over the cell count; the first version
  drew the roofs in 1,510,643 triangles where Revit's own bodies are 1,259, 138,
  240 and 192). A component the old guard accepted keeps its grid, so 124 of 126
  exports of Measure's models are byte-identical; the other two are Snowdon
  Towers Architectural and Snowdon Towers Structural, whose 43 footings
  take a saved mesh in place of their record box (Structural has no Revit IFC4
  export; the VIM export of the model scores them: the height of all 43 equals
  Revit's to 0.00001 m, the volume reads within 0.56%, the VIM's coarser
  tessellation). Two roofs stay blocks, each with its decoder diagnostic
  (#1706).
