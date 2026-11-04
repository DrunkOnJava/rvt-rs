- **`rvt-ifc --family-extrusions` draws a family instance from its family's
  own extrusions (#156, RE-327).** Off by default, and implies
  `--saved-meshes`: an instance whose body is the mesh Revit saved for it is
  written as `IfcExtrudedAreaSolid`s over `IfcIndexedPolyCurve` profiles,
  placed by the instance's transform, where its family's extrusions reproduce
  that mesh (every vertex within 0.0001 ft of their surfaces, samples of their
  surfaces within 0.0011 ft of the mesh, volumes within the arcs' chord loss
  and 0.01%), checked for each instance on its own; it keeps the mesh
  otherwise, as it does for a family with a void or with an extrusion that
  does not rebuild (an ellipse, an unclosed sketch) and for an instance with
  no transform that reads (other forms a family holds are not read, and the
  check against the instance's own mesh refuses what they make differ). An
  arc in a profile is written as the arc, the form Revit's IFC4 export of
  Revit 2024 gives it (#1346): a circle is two half arcs, and IfcOpenShell
  cuts them into the chords Revit's export is cut into (the office's 78
  waste terminals, 1.9e-6 ft). Revit 26.2's IFC2X3 exports of RE1 write a
  circle as an `IfcCircleProfileDef` instead, one chord fewer than the half
  arcs at some radii (27 against 28 for a toilet-paper dispenser of 0.2292
  ft, 0.001439 ft off), and no drawing meets both forms there, so in a Revit
  2025 or later file an instance with a whole circle over 0.12 ft in radius
  keeps its mesh (RE-327 section 11). The default export is unchanged (29
  models, hash for hash, held-out ones included). Revit 2024 to 2027 files,
  measured on 2024 and 2025: a file before 2024 has no instance transform
  that reads, so nothing is replaced.
