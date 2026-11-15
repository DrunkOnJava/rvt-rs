- **A wall turned off the model's axes can get its side areas, gross volume and
  height when its mesh is a prism less its openings (#1046).** The rule that
  reads those walls compared each face with the openings' world boxes. A world
  box is the opening's own extent only when the wall lies along a model axis or
  across one, so a turned wall was refused. Each opening is now taken as the
  corners of its rectangle, read in the wall's own frame, which is exact at any
  turn of a wall whose openings turn with it.
