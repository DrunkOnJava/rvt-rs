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
  arc in a profile is written as the arc where its radius is up to 0.12 ft,
  and as arcs whose chords lie within 0.00025 ft of it above that (#1346). The
  one body that failed the saved-mesh bar when arcs were drawn as arcs, a
  toilet-paper dispenser in RE1 Architecture, was a measured count mismatch:
  Revit's export draws its 0.2292 ft circle as an `IfcCircleProfileDef`,
  which IfcOpenShell cuts into 27 chords, and two half arcs of the same
  circle come out as 28, so no vertex coincides and Revit's lay 0.001439 ft
  from the 28-gon's chords. IfcOpenShell's steps are at most 0.2417 rad and
  hold the sagitta near 0.0016 ft above a radius of about 0.14 ft, so an arc
  over 0.12 ft could miss Revit's vertices by more than 0.001 ft whatever the
  form Revit wrote, and a short arc cannot (RE-327 section 11). The default
  export is unchanged (29 models, hash for hash, held-out ones included).
  Revit 2024 to 2027 files, measured on 2024 and 2025: a file before 2024 has
  no instance transform that reads, so nothing is replaced.
