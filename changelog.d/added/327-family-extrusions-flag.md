- **`rvt-ifc --family-extrusions` draws a family instance from its family's
  own extrusions (#156, RE-327).** Off by default, and implies
  `--saved-meshes`: an instance whose body is the mesh Revit saved for it is
  written as `IfcExtrudedAreaSolid`s over `IfcIndexedPolyCurve` profiles with
  true arcs, placed by the instance's transform, where its family's
  extrusions reproduce that mesh (every vertex within 0.0005 ft of their
  surfaces, samples of their surfaces within 0.0015 ft of the mesh, volumes
  within 1% and the arcs' chord loss), checked for each instance on its own;
  it keeps the mesh otherwise, as it does for a family with a void, a form
  that is not a rebuilt extrusion, an ellipse or an unclosed sketch, and for
  an instance with no transform that reads. The default export is unchanged.
  Revit 2023 to 2027 files. Not yet measured against Revit's own exports.
