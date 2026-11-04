- **`rvt-ifc --family-extrusions` draws a family instance from its family's
  own extrusions (#156, RE-327).** Off by default, and implies
  `--saved-meshes`: an instance whose body is the mesh Revit saved for it is
  written as `IfcExtrudedAreaSolid`s over `IfcIndexedPolyCurve` profiles of
  straight segments, placed by the instance's transform, where its family's
  extrusions reproduce that mesh (every vertex within 0.0001 ft of their
  surfaces, samples of their surfaces within 0.0011 ft of the mesh, volumes
  within the arcs' chord loss and 0.01%), checked for each instance on its
  own; it keeps the mesh otherwise, as it does for a family with a void, a
  form that is not a rebuilt extrusion, an ellipse or an unclosed sketch, for
  an instance with no transform that reads, and for any instance with an arc
  in a profile. The default export is unchanged (29 models, hash for hash,
  held-out ones included). Revit 2023 to 2027 files. Against Revit's own
  exports (Core Interior, RE1, the BIM4LCA office files) the first run, which
  drew arcs as arcs, replaced 335 bodies and 250 of the 251 the body scorer
  matched had Revit's vertices within 0.001 ft of their surface; the 251st, a
  toilet-paper dispenser, did not, because Revit's own tessellation of an arc
  starts where the file does not say. With arcs left out it replaces 64
  bodies, all 64 within 0.001 ft, and every scorer row equals the saved-mesh
  baseline's: lines are as exact as the mesh they replace, and the arcs'
  gains (the BIM4LCA office files' bodies within 0.01% of Revit's volume,
  1,547 to 1,625 of 1,650) are not taken (RE-327 section 9).
