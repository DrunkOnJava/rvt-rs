- **`rvt-ifc --family-extrusions` draws a family instance from its family's
  own extrusions (#156, RE-327).** Opt-in, and implies `--saved-meshes`: an
  instance whose body is the mesh Revit saved for it is written as
  `IfcExtrudedAreaSolid`s over `IfcIndexedPolyCurve` profiles with true arcs
  where its family's extrusions reproduce that mesh (every vertex within
  0.0005 ft of their surfaces, samples of their surfaces within 0.0015 ft of
  the mesh, volumes within 1% and the arcs' chord loss), and keeps the mesh
  otherwise, as it does for a family with a void, a form that is not a
  rebuilt extrusion or an instance placed other than by a rotation about the
  vertical axis. The default export is unchanged. Revit 2023 to 2027 files.
  Not yet measured against Revit's own exports.
