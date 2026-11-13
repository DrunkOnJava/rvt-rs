- **The column face is not where Revit stops the beams of a Revit 2027 concrete
  frame, and `tools/re/beam_ends_vs_ifc.py` counts what the ends of Revit's own
  beams lie on (#1839, RE-455).** The reporter ran the beam-end probe and the
  scorer on their model against Revit's export of it, and posted the counts:
  of 915 beams Revit shortens 538, by a median 256 mm; the Revit 2023 rule
  (the face of the column a beam's list names) gives bounds on 361 of them and
  is exact on 2, cutting 310 too far, and would cut 76 beams Revit leaves at the
  box's length, so it stays off for Revit 2024 and later. The scorer now also
  takes each Revit beam's mesh in the world frame and counts its two ends by
  the kind of element whose face (a column's, a beam's, a wall's or a slab's,
  within 1 mm) each end lies near in Revit's export, which asks Revit's own
  geometry what stops the beam without the record box. On the timber office of
  the ten BIM4LCA models, where the rule is exact on 59 of the 60 beams it cuts,
  108 of the 142 ends of the beams Revit leaves at the box's length lie on a
  column's face (105 of 138 at building permit); of the 13 beams Revit shortens
  on the ten models, one (the timber office at permit) has its ends on column
  faces.
