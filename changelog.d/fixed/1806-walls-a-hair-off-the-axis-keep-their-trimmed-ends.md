- **A wall whose line drifts a hair off a model axis keeps its record box's
  end where its join names a wall with no centreline (#1806, RE-447).** A
  vertical or horizontal line whose end drifts 5e-8 ft across the axis over
  16 to 27 ft is a few billionths off it in direction, so its box closure
  divided by that drift and failed and the wall ran from one end of its line
  to the other. At an end whose join (the wall's native `m_controlJoinsSet`)
  names a wall the reader gives no centreline, the line cannot be trimmed to
  that wall; such an end now takes the record box's end, which the join
  trimmed, and every other end keeps its line's, at the line's own angle, so
  a door or window set in the wall keeps its flip. On the BIM4LCA concrete
  housing at permit this puts one end of each of two walls back on Revit's
  (0.935 and 0.886 ft off since #1807) and moves no other end. The box's
  join trim is wrong at ends whose join decides no reach (#1814).
