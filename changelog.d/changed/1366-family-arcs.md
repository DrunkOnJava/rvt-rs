- **`rvt-ifc --family-extrusions` writes every arc as an arc (#1366, RE-331
  section 9).** The rule that kept an instance's saved mesh for an arc over
  0.12 ft in radius (#1346) is gone: every arc and circle of a family's
  extrusions is written as `IfcIndexedPolyCurve` arcs, whatever its radius, in
  the form Revit 24.0.5's IFC4 export gives it, now that the scorers judge a
  body both files write as true curves in the fine tessellation (#1365), where
  Revit's two forms of a circle coincide. `FamilyExtrusionReport::arcs` and the
  `arcs` field of `ExportNote::FamilyExtrusions`, the count of the instances
  the rule kept, are removed with it (the module is not in a release). The
  default export is unchanged.
