- **A flagged `GInstance` with a rotation matrix draws none of the 32 showers it
  was meant for (#1645, RE-414).** The traversal refuses a `GInstance` whose
  `m_bHasScale` is set, which kept the BIM4LCA housing's 22 showers and the
  office's 10 as boxes. Walking one whose matrix is a rotation (111 of the 112 in
  those files are the identity, one a quarter turn; no other model of the Measure
  set holds any) reaches the showers' nested symbols, which hold faces of three
  kinds the mesher refuses (multiple cylinder trim loops, nonrectangular parametric
  trims, HermiteSurf), so every shower stays a box and no body moves. The flag was
  not what blocked them; the rule is not shipped.
