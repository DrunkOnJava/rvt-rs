- **A door's or window's `IfcLocalPlacement` origin stays at its record box's
  centre: Revit's is not the stored origin, and no rule with 0 wrong that
  rvt-rs can check gives it (#1465, RE-357).** Measure run 42127619521 (Core
  Interior, RE1 Architecture and the BIM4LCA office as built and at permit,
  against Revit's own exports, every job succeeded) finds Revit's origin more
  than 0.5 ft from the stored one on all 1,061 walled doors and windows
  (604 counted once, without the permit file), and its offset one value for
  every element of an `IfcDoorType` or `IfcWindowType` entity (160 of 160 type
  entities) and not for every element of a symbol (46 of 82, in the stored
  axes), which is revit-ifc's per-type `StyleTransform`, taken from the
  instance that creates the type and not recorded in the file. Two axes
  follow the element's own opening (x and z on all 919 walled elements of the
  two office files), the third, across the wall, does not (y on 375 of 385
  doors and 198 of 534 windows). The elements with both flips set agree on all
  three axes (391 of 391), and that is not written as a rule: the flip state
  has no mechanism behind it. Where Revit's export gives a door no opening,
  its origin is the stored one (153 of 153; #1494). The scorer
  `door_placements_vs_ifc.py` now prints the offset by type entity, symbol and
  flip state, and per axis against each export's opening.
