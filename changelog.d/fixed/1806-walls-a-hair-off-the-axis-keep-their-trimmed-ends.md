- **A wall whose line drifts a hair off a model axis keeps its record box's
  extent along it (#1806, RE-447).** A vertical or horizontal line whose end
  drifts 5e-8 ft across the axis over 16 to 27 ft is a few billionths off it
  in direction, so it missed the axis-parallel case, its box closure divided
  by that drift and failed, and the wall ran from one end of its line to the
  other. Where the box does not close and the line's ends lie within 1e-6 ft
  of each other across an axis, the wall now keeps the box's extent along the
  axis, as an axis-parallel wall does, at the line's own angle, so a door or
  window set in it keeps the flip Revit's exporter gives it. On the BIM4LCA
  concrete housing at permit this puts one end of each of two walls back on
  Revit's (0.935 and 0.886 ft off since #1807), and the other end of each
  goes back to the box's 1.378 and 1.280 ft short, from 0.443 and 0.394 ft:
  the box's join trim at a wall whose join decides no reach, which leaves
  other walls of the file as far short (#1814).
