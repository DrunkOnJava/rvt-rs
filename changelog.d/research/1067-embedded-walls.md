- **A wall that embeds a curtain wall is cut by it; the body is measured, not
  drawn (#1067, RE-335).** After RE-267 and RE-320 the BIM4LCA office's walls
  whose lowest point is not Revit's are 20 as built and 24 at building permit
  (Measure run 41906631624, `main` at 5026d863). Fourteen of the 20 as built
  stand on footings Revit draws sloped. The other six are raised exactly
  8.2021 ft, and each embeds a curtain wall (`WallEmbedWallGStep`) that is
  8.2021 ft tall from the wall's base and spans 0.90 to 1.0 of its length.
  - All 72 walls embedded in the 57 walls that embed one are curtain walls, so
    the step alone does not separate the six. What does in Revit's body is the
    end stubs: 2103576 as built has the same curtain wall as 2098938 and keeps
    its stubs (Revit's volume 32.08 ft³ against 23.42), where 2098938's are
    gone. No byte signal that tells them apart is read.
  - Nothing is drawn differently, and the diagnostics do not count these walls:
    a rule that raised the base wherever the curtain wall spans the wall would
    be wrong on 2103576 and 2177158.
