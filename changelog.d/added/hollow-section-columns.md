- **A column whose type is a square hollow section is drawn as it (W6,
  RE-221).** Such a column was a solid box. Its section's outside, wall
  thickness and corner radius are now read from its family's own
  parameters, and it is drawn as Revit's export draws it: an
  `IfcArbitraryProfileDefWithVoids` of two `IfcIndexedPolyCurve`s, each four
  lines and four quarter arcs, extruded through its height (`BodySource`
  `family_type_hollow_section`). On the BIM4LCA office all 79 such columns
  as built and 55 at building permit are Revit's body, every vertex within
  0.001 ft. The IFC model gains `ProfileDef::IndexedPolyCurveWithVoids` and
  `IndexedCurve` for it.
