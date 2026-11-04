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
  arc in a profile is written as the arc, whatever its radius (#1366), in the
  form Revit 24.0.5's IFC4 export gives it. Revit 26.2's IFC2X3 export writes
  a circle as an `IfcCircleProfileDef` and IfcOpenShell's default
  tessellation of the two forms has a chord more or fewer at some radii (27
  against 28 for a toilet-paper dispenser of 0.2292 ft, 0.001439 ft off); the
  scorers no longer count that as a miss, since a body both files write as
  true curves is judged in a fine tessellation, where the forms coincide
  (RE-327 section 11, RE-331). On Core Interior, RE1 and the BIM4LCA office
  files, measured at `34750454` (runs 41884078892 and 41884639625), 335 bodies
  are replaced; the body scorer scores 251 of them and all 251 hold the bar,
  187 judged in the fine tessellation and 64 in the default one, and 6 of 53
  exports move against `main`'s, none of them held-out. The extrusions are
  smaller than the meshes they replace: the BIM4LCA office's IFC is 77.9 MB
  instead of 82.2 MB, and its terrain files' 0.17 MB instead of 0.92 MB. The
  default export is unchanged (34 of 34 models, run 41884073888). Revit 2024
  to 2027 files, measured on 2024 and 2025: a file
  before 2024 has no instance transform that reads, so nothing is replaced.
