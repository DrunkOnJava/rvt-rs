- **A door's or window's `IfcLocalPlacement` origin stays at its record box's
  centre: Revit's is not the stored origin, and no rule with 0 wrong that
  rvt-rs can check gives it (#1465, RE-357).** Scored on Core Interior, RE1
  Architecture and the BIM4LCA office as built against Revit's own exports
  (locally, on the kept IFCs of Measure run 42066757105; the Measure run of
  this change is not yet in), Revit's origin is more than 0.5 ft from the
  stored one on all 604 walled doors and windows, and its offset is one value
  for every element of an `IfcDoorType` or `IfcWindowType` entity (92 of 92
  type entities) and not for every element of a symbol (25 of 46, in the
  stored axes), which is revit-ifc's per-type `StyleTransform`, taken from the
  instance that creates the type. Two axes follow the element's own opening (x and z on all 462
  walled elements of the office), the third, across the wall, does not (y on
  187 of 193 doors and 99 of 269 windows). Where Revit's export gives a door
  no opening, its origin is the stored one (76 of 76). The scorer
  `door_placements_vs_ifc.py` now prints the offset by type entity, symbol and
  flip state, and per axis against each export's opening.
