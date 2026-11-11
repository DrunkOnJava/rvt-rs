- **A continuation bent off a curtain mullion's axis no longer hides its end face (#1735, RE-432).** The
  8 `Qto_MemberBaseQuantities.NetSurfaceArea` values one end face short on each BIM4LCA office file (RE-404's
  rule on the mullions RE-421 newly meshes, which landed together unmeasured) all have a partner bent off the
  axis by 1.8 to 2.2 degrees, which RE-268 and RE-404 counted as hiding the end, and Revit's solid counts the
  end faces of all 159 of the office's mullions that have one. A partner now hides an end only where it
  coincides with it (a cosine above 0.99999). The 8 read equal to Revit's on the concrete office as built and
  at permit (9 different quantities to 1 each), 7 of the timber office's 9 as built and 5 of its 11 at permit;
  30 mullions' values change and nothing else, and 122 of 126 exports are byte-identical.
