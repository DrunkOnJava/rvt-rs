- **rvt-rs writes the default local axes (X along +X, Z up) on every door and
  window, and they differ from Revit's on most of those bound to a plain wall
  (#1447, RE-347).** `tools/re/door_placements_vs_ifc.py` compares the axes of
  each element's `IfcLocalPlacement` in rvt-rs's export with Revit's, by Tag,
  in Revit's internal frame (Measure run 42066757105, head `eb6de281`). On
  Core Interior, RE1 Architecture and the BIM4LCA office files (as built and
  at permit), the same elements as RE-343's Rule 1, 406 of 521 doors and 314
  of 540 windows bound to a plain wall differ (115 and 226 are the same,
  where Revit's X is the model's +X): 189 and 202 by a half turn, 217 and 112
  by a quarter turn. In the 2024 to 2027 held-out models 19 of 41 doors and
  83 of 105 windows differ. The mixed flip state of RE-343 is only part of
  it, since walls along -X and along Y differ with equal flags. RE-343's
  rule (the stored axes, reversed where the flips differ) reproduces Revit's
  on all 521 and 540, and is not written: it would move most door and window
  placements, and needs an unfiltered Measure first. The origin (the record
  box's centre) is a separate difference and is not claimed. Snowdon Towers
  and the R2025 models are not measured.
