- **A curtain mullion drawn from its saved mesh gets Revit's `NetSurfaceArea` (#1046, RE-404).**
  Where its solid's flat ends are one section each, the mullion's
  `Qto_MemberBaseQuantities.NetSurfaceArea` is the area of the faces along its
  axis plus the end faces its solid keeps (RE-268's count, now read for a
  mullion on any host, where the box's own value was written only on a plain
  one); a mullion with a mitred end gets its lateral area plus the mitre face,
  and the flat end where that is kept. A mullion with a partner (a piece of the
  same run that meets its end) at an end that count keeps is given none, since the
  count reads the boxes of one curtain wall and misses a continuation on another.
  Revit's export writes the same on the mullions of the BIM4LCA housing and
  office whose body rvt-rs draws from the mesh Revit saved: 1,634 to 1,658 more
  of each office's mullions read equal to Revit's (the timber office at permit,
  5,292 to 6,950 of 7,872) and the 184 mitred mullions of the housing's large
  files, none different. 13 of Autodesk's samples gain the value on 4,877
  mullions, which no Revit export scores.
