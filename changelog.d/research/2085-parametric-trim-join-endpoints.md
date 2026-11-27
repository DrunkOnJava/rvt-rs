- **A trim loop the coordinated join closes meets at one point (#2085).** Where
  the coordinated join takes a stored gap of a parametric face's trim loop, the
  two ends' parameters are reconciled to one point before the face is cut into
  strips. The point is checked against both ends in model space and added to
  the U knots of the repaired loop.
