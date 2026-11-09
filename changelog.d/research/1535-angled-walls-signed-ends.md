- **Revit 2015 to 2020 angled walls keep their record box: Revit trims seven of
  the eight that a line body makes worse about 0.28 ft inside their lines'
  ends, as a join does (#1535, RE-364).** On the spent Vitoria `ALD26 Before`
  (Revit 2018, IFC2X3), drawing an angled wall's faces from its line takes 33
  of 48 to Revit's faces within 0.001 ft, from 1. Of those walls, 8 are worse
  than their box at their ends alone, and a scorer that signs each end's
  difference shows why. Seven run 0.275 to 0.279 ft past Revit's end with the
  line and 0.163 to 0.186 ft with the box, at one or both ends. No angled
  wall has a join reach on these releases, so they stay boxes until the join
  trims are read. No export changes.
