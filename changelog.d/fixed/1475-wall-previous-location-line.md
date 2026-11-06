- **A wall whose record holds its previous location line beside its
  location curve is read by the curve `VWallDriver.m_pCrv` names (RE-360,
  #1475).** Revit 2027's save of `rac_advanced` stores a second `GLine`,
  owned by `WallRunTimeData.m_prevLocLine`, in 18 of its 180 `SWall`
  records, and `native_walls` left out any wall with a second curve, so the 8
  host walls of 14 doors returned no native line and the doors kept the
  default axes. A record with exactly two curves is now read where one is the
  level `GLine` `m_pCrv` names and the other is a `GLine` only
  `m_prevLocLine` names; every wall read before is read as before, and a
  `CurveLoop.m_curves` profile wall stays unread. Measured on Autodesk's
  sample projects of 2024 to 2027 (no Revit export), Measure run
  42134051521 at `0a0ba7ce` against the same code without it: 26 walls are
  newly read over every `SWall` record (`rac_advanced` 2027 18, `rac_basic`
  6, and 5 of those on the earlier saves, Snowdon Towers Architectural 2), of
  which 20 can reach an export (the 18, and `rac_basic` 709245 and 234869;
  none on Snowdon). `rac_advanced` and `rac_basic` then read the same walls
  on all four saves, the 14 doors are placed by the RE-347 axes rule and
  equal the 2025 save's in axes, origin, vertices and volume, and two
  curtain walls (`rac_basic` 709245 on every save, `rac_advanced` 149877 on
  2027) gain `Qto_CurtainWallQuantities.Length`, a value no Revit export or
  gate checks on these files. No other wall, placement, body or property of
  the five exports that move (`rac_basic` 2024 to 2027, `rac_advanced` 2027)
  changes. The unfiltered run (Measure 42141380072 at `5754c7f3`, 111 models
  and samples) moves six exports: those five and the BIM4LCA office as built,
  where one door gains the rule's axes and they are Revit's (doors placed by
  the rule 193 to 194, none differing from Revit's; its two newly read plain
  walls' lengths are Revit's, and no body, class or quantity of it changes).
  The other 105, Core Interior and the office at permit among them (281 and
  76 walls newly read), are byte for byte main's. No held-out model holds a
  previous location line. The 14 openings the doors cut have their doors'
  axes; their size is still a record box on 2027, as the cut loop of a door's
  symbol is read on 2024 and 2025 only.
