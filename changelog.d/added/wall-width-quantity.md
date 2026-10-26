- **Walls carry `Qto_WallBaseQuantities.Width` (#1016).** It is the
  wall type's thickness, the Width parameter revit-ifc reads: 602 of the
  BIM4LCA office's 607 walls as built, and 519 of 589 at building permit,
  each as Revit writes it. A wall's `Length` is not written yet: Revit's is
  its Length parameter, not its location line's length.
