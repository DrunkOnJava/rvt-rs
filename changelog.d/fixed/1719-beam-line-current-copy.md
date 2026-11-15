- **A beam whose copies in several partitions disagree gets its `Length`, `Span`,
  `Slope` and `Roll` from its current partition's location line (#1719).** The
  reader dropped a beam whose element data sat in several partitions with
  different lines, so Revit's four values stayed missing on 9 beams of the
  BIM4LCA models: one in each housing file as built and 7 in the timber office at
  permit. They now take the line of the copy in the beam's current partition, the
  rule sketch lines already follow (RE-216); each beam's record box is already
  that copy's. The beam's body stays its box: no solid along those lines has been
  measured against Revit's. The 36 values are Revit's, with none different; a
  held-out 2021 project's one such beam also gains its `Span` and `Slope`, equal
  (counts only).
