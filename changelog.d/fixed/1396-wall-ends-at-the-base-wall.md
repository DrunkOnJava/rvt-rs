- **A wall whose end candidates stand above its base ends at the face of the
  wall that holds the base (#1396, RE-337).** The join trim of a wall drawn from
  its record box (RE-26) cut an end half the thickness of the walls with their
  centreline on it that overlap it in elevation by any amount. Where those
  walls all lie above the wall's base and exactly one other wall its record
  names holds the base and has the end inside its thickness, the end is now cut
  at that wall's face. Revit's body steps the end up the height and rvt-rs
  draws one prism, so the end is Revit's at the base and below the wall that
  holds it. The measured effect is in
  `reports/element-framing/RE-337-wall-ends-at-the-base-wall.md`.
