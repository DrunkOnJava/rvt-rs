- **A continuation of another section no longer hides a curtain mullion's end face (#1745, RE-436).** The 2
  `Qto_MemberBaseQuantities.NetSurfaceArea` values one end face short on the BIM4LCA timber office as built are
  two pieces of a row where a 50 x 50 mm mullion meets a 150 x 50 mm one end to end, and Revit's value counts
  the end where the sections differ whole and hides the end where they match, as it hides an end only where it
  coincides with its continuation's end face. A partner now hides an end only where its section is the
  mullion's. The 2 read equal to Revit's as built and at permit (different 2 to 0 and 6 to 4); 4 values change
  and nothing else, and 124 of 126 exports are byte-identical.
