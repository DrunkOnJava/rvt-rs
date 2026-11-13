- **Revit's export stops a beam where its record box ends on the BIM4LCA
  models, so the column face cannot be tested there (RE-455, #1839).** On the
  ten BIM4LCA models that have Revit's export (300 beams), Revit's beam is its
  record box's length within 1 mm on 273, shorter on 25 (13 beams) and longer
  on 2. The only beams that name a column are the timber office's, 63 on each
  file, and their box already ends at the column's near face: RE-122's rule
  gives the box's own ends on 116 of the 118 it reaches that Revit leaves at
  the box, cuts beam 3360608 230 mm short on both files, and is exact on the
  one beam Revit shortens and it reaches. The 13 beams Revit does shorten name
  a wall (7), a column (1) or nothing (5), and a wall's near face is not
  Revit's stop for any of the ten beams it holds an end of. Nothing ships. The
  test is a model whose boxes run to the lines' ends, such as the reporter's:
  the probe and `tools/re/beam_ends_vs_ifc.py`, which prints counts only
  without `--list`, are for it.
