- **Exports hold the EXPRESS rules of the IFC4 schema that
  IfcOpenShell's default validation leaves out; Revit's way of breaking one
  is behind `--revit-set-placement` (#1430).** Measure's `rules:` check
  (#1106) found three kinds of rule finding in what `rvt-ifc` wrote on main,
  in run 41988011289:
  `IfcShapeRepresentation.CorrectItemsForType` (an
  `IfcFixedReferenceSweptAreaSolid`, the body of a sloped beam, under
  `SweptSolid`, which IFC4 reserves for extruded and revolved solids: 14
  findings on `rst_basic` 2024, 2 on `rac_advanced` 2024, 10 on Snowdon
  structural 2024), `CorrectPredefinedType` (an `IfcAirTerminalType`,
  `IfcElectricApplianceType` or `IfcSpaceType` with `PredefinedType`
  `USERDEFINED` and no `ElementType`, 1 each on the BIM4LCA office as built
  and at permit) and `IfcRelAssociatesMaterial.AllowedElements` (a material
  associated with an `IfcSpace`, which that rule does not take: 1 on each
  office file and each terrain file). A sweep along a directrix is now
  written under `AdvancedSweptSolid`. A body of several solids takes the one
  type IFC4 accepts for all its items (`AdvancedSweptSolid`, `SolidModel` or
  `SurfaceOrSolidModel`), where it used to take `CSG` for any mix, which
  holds boolean results alone; a mix with a boolean result is written as the
  union of its items under `CSG`. No current producer builds such a mix. A type
  object whose `PredefinedType` is `USERDEFINED` now carries its own name as
  `ElementType`, and keeps the `PredefinedType` and the GlobalId Revit's
  export has. An `IfcSpace` is associated with no material, and
  `rvt-ifc --revit-set-placement` (the MCP server's `revit_set_placement`)
  keeps the one Revit's export associates with each. Revit's own export of
  the office breaks the second and third rules the same way (measured on the
  as-built file with the same check, IfcOpenShell 0.9.0.post1: `USERDEFINED`
  types with `ElementType` `$`, and a material on `IfcSpace`), so what to
  write was Griffin's call, decided on 2026-11-05: valid IFC4 by default,
  Revit's behind the flag. The output of `--revit-set-placement` therefore
  breaks `IfcRelAssociatesMaterial.AllowedElements` on the 42 `Turning Free
  Space` instances of each office file and the 2 of each terrain file, as
  Revit's export does; the `ElementType` is set either way, which Revit's
  export leaves empty. Revit's export of a beam is an `IfcExtrudedAreaSolid`
  under `SweptSolid`; the sloped beam's body is still a fixed-reference
  sweep. CI's IfcOpenShell job now runs the same rules check, on the default
  output, on every export it writes (the tier-1 projects, the synthetic pair,
  Einhoven, Core Interior with and without saved meshes and the four RE1
  projects) and fails on any finding, and Measure keeps the count of rule
  findings, with the entity and rule that name each, for the held-out models,
  whose `schema.txt` it deletes. Measured in Measure run 41994889680 (code
  head c4b6e861, default output, `base=main`, `nordic`, `r2025`, `holdout`
  and `samples` on, no `ifc_flags`; the base side reproduces the findings
  above): the `rules:` section reads `findings 0` on the 90 models whose
  export it checks (Core Interior, Einhoven, the four RE1 projects, the four
  office files, 24 Revit 2025 models, one spent pair's model and 55 of
  Autodesk's 58 sample projects) and the 18 held-out models report `schema rules: findings
  0` as counts. The other 3 samples (the 2024 `rme_advanced`, `rme_basic` and
  Snowdon architectural) are over the 30 MB the schema step reads, so their
  rules are not measured, as before. The export's `ifc.sha256` moves on 7
  models only, the 7 that had a finding (the four office files and the 2024
  `rac_advanced`, `rst_basic` and Snowdon structural), and no other output of
  any model differs between the two sides apart from `schema.txt`, the
  timings, logs, diagnostics and MCP dump not compared. That includes the
  type-object, GlobalId, tagged-space and property-set scorers, so no Tag,
  GlobalId, `PredefinedType` or property value moved, and Core Interior's
  export is the same bytes. `element_materials_vs_ifc` pairs no `IfcSpace`,
  so it reads the same on both sides: the spaces' materials leaving the
  default output shows only as the finding going. Not yet measured: the 2026
  and 2027 sample exports that #1426 will give their findings (it is not in
  this base), the output of `--revit-set-placement`, the CI gate's time on
  Core Interior and its first check of the tier-1 and RE1 Electrical,
  Mechanical and Plumbing exports.
