- **A wall's end at a corner of three or more walls follows the order its
  record joins them in (RE-322, #1194).** A join's walls are listed in the
  order Revit joined them, and an earlier wall wins: counting only the walls
  that overlap in elevation (the list also names the walls on other storeys
  that end at the same point), a wall stops at the near face of the
  perpendicular walls listed before it, meets flush a wall that continues it
  when that is listed before the next perpendicular wall, and otherwise runs
  through the perpendicular walls listed after it. The faces are those of each
  wall where its own line ends, since walls of different thickness are
  aligned on a face, not a centreline. It is read only where it is right on
  every measured end: a butt join, every wall straight and meeting at a right
  angle or straight on, no element but a wall cutting the wall, the faces that
  decide agreeing, and the wall single-layer or stopping against single-layer
  walls. Against Revit's own export of the BIM4LCA office, 510 of 607 walls
  as built have both ends within 0.001 ft (482 before) and 430 of 589 at
  building permit (422); 30 and 8 ends go from wrong to right and none from
  right to wrong. Two single-layer walls that meet at a mitre (R2025
  `18_WallJoins`, the only one measured) end on the line through the two
  corners: 9 of its 9 walls, from 7. Core Interior and RE1 export byte for
  byte as before. Layered corners of two walls, a layered wall at layered
  walls and every corner a column cuts were measured and are not read
  (`reports/element-framing/RE-322-wall-corner-order.md`).
