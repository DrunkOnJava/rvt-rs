- **A wall end whose join step names an omitted target stops flat at its
  partner's near face (#1814, RE-488).** At a layered butt join, rvt-rs drew
  the end of a wall that stops to the layer that reaches furthest into its
  partner, as Revit cleans such a join layer by layer. Where the end's
  `JoinEndGStep` names an omitted target, Revit's export instead ends the whole
  wall flat at the partner's near face. Such an end now stops there with every
  layer, where its record joins no roof or floor to the wall: 8 ends of the
  BIM4LCA concrete housing at permit, 435 or 390 mm long before, are now
  Revit's, with nothing else moving. A roof-joined wall is left as it was,
  since Revit ran one such end on over the height its roof join shapes. The
  rule rests on one building (`docs/one-project-rules.md`).
