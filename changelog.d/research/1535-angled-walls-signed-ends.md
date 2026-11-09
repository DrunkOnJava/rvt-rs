- **Revit 2015 to 2020 angled walls keep their record box: Revit stops seven
  of the eight walls that a line body makes worse about 0.28 ft inside their
  lines' ends (#1535, RE-364).** On the spent Vitoria `ALD26 Before` (Revit
  2018, IFC2X3), drawing an angled wall's faces from its line takes 33 of 48 to
  Revit's faces within 0.001 ft, from 1. Of those walls, 8 are worse than
  their box at their ends alone. A scorer that signs each end's difference
  shows that seven run 0.275 to 0.279 ft past Revit's end with the line, and
  0.163 to 0.186 ft with the box, at one or both ends. That fits a join trim
  or a rule for their one wall type, and neither is tested. No angled wall
  has a join reach on these releases, so they stay boxes. No export changes.
