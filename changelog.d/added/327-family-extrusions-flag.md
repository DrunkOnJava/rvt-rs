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
  arc of 0.12 ft in radius or less is written as the arc (#1346). An instance
  with an arc over 0.12 ft in radius keeps its mesh in every release: Revit
  24.0.5's IFC4 export writes a circle as arcs and Revit 26.2's IFC2X3 export
  as an `IfcCircleProfileDef`, IfcOpenShell's default tessellation of the two
  forms has one chord more or fewer at some radii (27 against 28 for a
  toilet-paper dispenser of 0.2292 ft, 0.001439 ft off), and the chord passes
  the 0.001 ft bar over 0.137 ft, so no one form meets both exporters and
  nothing in the `.rvt` says which will read it (RE-327 section 11; true arcs
  for those return if the scorers' tessellation changes the bar, #1355). Of
  the 335 bodies the first run replaced, 66 are replaced and 269 kept for an
  arc, every scorer row equal to the saved-mesh baseline's; no partial arc and
  no arc of the IFC4 dialect is among the 66. The default export is unchanged
  (111 of 111 models of an unfiltered byte-identity run, held-out ones
  included). Revit 2024 to 2027 files, measured on 2024 and 2025: a file
  before 2024 has no instance transform that reads, so nothing is replaced.
