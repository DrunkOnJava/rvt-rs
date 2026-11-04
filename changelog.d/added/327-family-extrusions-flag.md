- **`rvt-ifc --family-extrusions` draws a family instance from its family's
  own extrusions (#156, RE-327).** Off by default, and implies
  `--saved-meshes`: an instance whose body is the mesh Revit saved for it is
  written as `IfcExtrudedAreaSolid`s over `IfcIndexedPolyCurve` profiles with
  true arcs, placed by the instance's transform, where its family's
  extrusions reproduce that mesh (every vertex within 0.0001 ft of their
  surfaces, samples of their surfaces within 0.0011 ft of the mesh, volumes
  within the arcs' chord loss and 0.01%), checked for each instance on its
  own; it keeps the mesh otherwise, as it does for a family with a void, a
  form that is not a rebuilt extrusion, an ellipse or an unclosed sketch, and
  for an instance with no transform that reads. The default export is
  unchanged (29 models, hash for hash, held-out ones included). Revit 2023
  to 2027 files. Against Revit's own exports (Core Interior, RE1, the BIM4LCA
  office files), the first run replaced 335 bodies with wider tolerances and
  250 of the 251 the body scorer matched had Revit's vertices within 0.001
  ft of their surface; the 251st did not, and the tolerances were narrowed
  to the sample projects' own (RE-327 §9 reads the narrowed run). It costs
  up to 20% more export time.
