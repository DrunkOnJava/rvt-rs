- **A room outline drops a point where two straight edges turn by less than
  a hundred-millionth (#1825).** The test for a straight continuation took
  only turns under 1e-12 of the edges' lengths, so two rooms of the BIM4LCA
  timber office, `TSTO5` and `TSTO9`, kept a vertex Revit's own outline does
  not have, on both of its saves. They now lie within 0.01 ft of Revit's.
