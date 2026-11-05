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
  holds boolean results alone. A mix with a boolean result is written as the
  union of its items under `CSG`, a triangulated item of it as the faceted
  brep of the same triangles: IFC4 takes a tessellated boolean operand only
  where it is closed (`IfcBooleanResult.FirstOperandClosed` and
  `SecondOperandClosed`), and the face set records `Closed` as `$`. On
  hand-written STEP files, IfcOpenShell 0.9.0.post1 finds that rule on the
  union with the face set as either operand and none on the same union with
  the face set marked closed or written as the faceted brep, and finds
  `CorrectItemsForType` on extruded and tessellated items under `CSG` and
  none under `SurfaceOrSolidModel`. No current producer builds such a mix, so
  no current output changes. A type object whose `PredefinedType` is
  `USERDEFINED` now carries its own name as `ElementType` (an empty label
  where the type has no name), and keeps the `PredefinedType` and the GlobalId
  Revit's export has. An occurrence with `PredefinedType` `USERDEFINED` and no
  `ObjectType` read is written without a `PredefinedType`, which would break
  the same rule, and no measured model has one. An `IfcSpace` is associated
  with no material, and `rvt-ifc --revit-set-placement` (the MCP server's
  `revit_set_placement`) keeps the one Revit's export associates with each.
  Revit's own export of the office breaks the second and third rules the
  same way (measured on the as-built file with the same check: `USERDEFINED`
  types with `ElementType` `$`, and a material on `IfcSpace`), so what to
  write was Griffin's call, decided on 2026-11-05: valid IFC4 by default,
  Revit's behind the flag. The output of `--revit-set-placement` therefore
  breaks `IfcRelAssociatesMaterial.AllowedElements` on the 42 `Turning Free
  Space` instances of each office file and the 2 of each terrain file, as
  Revit's export does; the `ElementType` is set either way, which Revit's
  export leaves empty. The flag is the writer's entry points
  `write_step_with_space_materials` and `write_step_to_with_space_materials`,
  which `rvt-ifc` and the MCP server pick by the option; library, Python and
  wasm callers of `write_step` get the default. Everything that lists an
  element's material follows the default export, as the schedules follow its
  storeys (#1417, #1422): the elements CSV (`rvt-schedule`, Python's
  `schedule_csv()` and the viewer's download) and the viewer's info panel,
  schedule and material highlight list none for an `IfcSpace`, and the
  material's element count leaves spaces out. No schedule has the flag, so
  none lists a space's material. The glTF colours are left alone: a colour on
  a space's mesh is a drawing choice, not an association. Revit's export of a
  beam is an `IfcExtrudedAreaSolid` under `SweptSolid`; the sloped beam's body
  is still a fixed-reference sweep. CI's IfcOpenShell job now runs the same
  rules check, on the default output, on every export it writes (the tier-1
  projects, the synthetic pair, Einhoven, Core Interior with and without
  saved meshes and as the two witness steps write it, and the four RE1
  projects), after the witness steps so that a finding never drops the
  observations a replay fix needs, and fails on any finding. Its check no
  longer ends at the first message holding a `%` (an `IfcSpaceType` named `50
  % Space` made IfcOpenShell's logger raise and hid that finding and the
  ones after it), and Measure keeps the count of rule findings, with the
  entity and rule that name each, for the held-out models, whose `schema.txt`
  it deletes, saying `not read` where the rules check left no count. Measured in Measure run
  41994889680 (code head c4b6e861, default output, `base=main`, `nordic`,
  `r2025`, `holdout` and `samples` on, no `ifc_flags`; the base side
  reproduces the findings above): the `rules:` section reads `findings 0` on
  the 90 models whose export it checks (Core Interior, Einhoven, the four RE1
  projects, the four office files, 24 Revit 2025 models, one spent pair's
  model and 55 of Autodesk's 58 sample projects) and the 18 held-out models
  report `schema rules: findings 0` as counts. The other 3 samples (the 2024
  `rme_advanced`, `rme_basic` and Snowdon architectural) are over the 30 MB
  the schema step reads, so their rules are not measured, as before. Run
  42002548557 (head f3c87e48, the same flags) read by the main session: 104
  of 111 exports are byte-identical and 7 move, the 4 office files and the
  2024 `rac_advanced`, `rst_basic` and Snowdon structural, the 7 that had a
  finding, with 0 rule findings on every model checked, the held-out counts
  at 0 on all 18 and the MCP server's answers equal on 1,258 lines. No other
  output of any model differs between the two sides apart from `schema.txt`
  (run 41994889680, compared file by file with the timings, logs, diagnostics
  and MCP dump apart), which covers the type-object, GlobalId, tagged-space
  and property-set scorers. `element_materials_vs_ifc` pairs no `IfcSpace`,
  so it reads the same on both sides: the spaces' materials leaving the
  default output shows only as the finding going. With `--revit-set-placement`
  (run 42002553349, head f3c87e48, the two office files, `base=main`, the
  `schema`, `psets_vs_ifc` and `element_materials_vs_ifc` steps) the rules
  check reads `findings 1` on each, `AllowedElements` for the 42 spaces, where
  `main` reads `findings 4`, and the property set and material outputs are the
  same at `main` and at the head. CI's gate printed `findings 0` on 13 files
  in 48 s on the pull request's CI run at f3c87e48 (read by the main
  session). The three fixed kinds occur on none of Core Interior, Einhoven
  and the four RE1 projects, which read 0 on `main` in run 41994889680, so
  the gate would have passed those on `main`; the tier-1 and synthetic
  exports were not checked on `main`. That the gate fails on each kind was
  shown on Revit's terrain export (52 findings) and on a hand-written file
  with all three kinds (exit 1). Not yet measured on the head that carries the union,
  consumer, `USERDEFINED` occurrence and gate-position changes: the
  exports' bytes against run 42002548557's head side, the elements CSV, and
  the 2026 and 2027 sample exports that #1426 will give their findings.
