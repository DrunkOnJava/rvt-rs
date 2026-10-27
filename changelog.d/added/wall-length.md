- **Walls' `Length` (#1046, RE-252).** A wall carries
  `Qto_WallBaseQuantities.Length` where its curve, cut back where its body
  meets the walls joined at its ends, is as long as the curve. Revit's
  export writes either length, by how it draws the wall, so only there is
  the value never in doubt: 88 of the BIM4LCA office's walls as built and 81
  at permit, each Revit's value, none different. The cut-back ends are read
  twice, from the wall's native record and from its join reaches, and taken
  only where the two agree.
