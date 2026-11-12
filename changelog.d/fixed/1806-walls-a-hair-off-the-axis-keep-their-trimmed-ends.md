- **A wall whose line drifts a hair off a model axis keeps the ends its joins
  trimmed (#1806, RE-447).** A wall's line was taken as axis-parallel only
  when its direction was within 1e-9 of an axis, so a vertical or horizontal
  line whose end drifts 5e-8 ft across the axis over 16 to 27 ft was drawn as
  a wall at an angle: from one end of its line to the other, dropping the
  ends its record box's joins trimmed. A line is now axis-parallel when its
  ends lie within 1e-6 ft of each other across the axis, and a tapered or
  layered wall takes its run from the same test. Two wall ends of the BIM4LCA
  concrete housing at permit, exact before #1807 gave their walls their
  type's thickness, were 0.935 and 0.886 ft off since.
