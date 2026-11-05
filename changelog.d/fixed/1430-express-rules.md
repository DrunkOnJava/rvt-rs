- **Exports hold the EXPRESS rules of the IFC4 schema that
  IfcOpenShell's default validation leaves out (#1430).** Measure's
  `rules:` check (#1106) found three kinds of rule finding in what `rvt-ifc`
  wrote on main, in run 41988011289: `IfcShapeRepresentation.CorrectItemsForType`
  (an `IfcFixedReferenceSweptAreaSolid`, the body of a sloped beam, under
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
  union of its items under `CSG`. A type object whose `PredefinedType` is
  `USERDEFINED` now carries its own name as `ElementType`, and keeps the
  `PredefinedType` and the GlobalId Revit's export has. An `IfcSpace` is
  associated with no material. Revit's own export of the office breaks the
  second and third rules the same way (measured on the as-built file with the
  same check: `USERDEFINED` types with `ElementType` `$`, and a material on
  `IfcSpace`), so those two values now differ from Revit's on purpose: the
  `ElementType` Revit leaves empty is set, and the material Revit associates
  with each `Turning Free Space` instance (42 on each office file, 2 on each
  terrain file) is not. Revit's export of a beam is an `IfcExtrudedAreaSolid`
  under `SweptSolid`; the sloped beam's body is still a fixed-reference
  sweep. No Tag, GlobalId or other value moves. CI's IfcOpenShell job now
  runs the same rules check on every export it writes (the tier-1 projects,
  the synthetic pair, Einhoven, Core Interior with and without saved meshes
  and the four RE1 projects) and fails on any finding, and Measure keeps the
  count of rule findings, with the entity and rule that name each, for the
  held-out models, whose `schema.txt` it deletes. Not yet measured on this
  head: the effect of the change on any model.
