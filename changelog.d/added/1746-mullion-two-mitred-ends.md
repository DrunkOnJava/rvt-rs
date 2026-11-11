- **A curtain mullion with two mitred ends and both kept gets the whole surface as its `NetSurfaceArea`
  (#1746, RE-433).** A bent joint mitres both ends of a mullion that has a bent partner at each, and the rule
  that reads a prism's end faces returned none for two mitred ends, so the 131 mullions of the BIM4LCA concrete
  office as built (and 131 at permit, 134 and 132 on the timber office) got no value although their ends are
  counted right since RE-432. The faces along the mullion plus both end faces are Revit's value on all of
  them: 528 more values equal to Revit's across the four office files, none different; 122 of 126 exports are
  byte-identical. A prism with two mitred ends and an end not kept still gets none.
