- **A curtain mullion that keeps a partner end gets its `NetSurfaceArea` from its saved mesh (#1790, RE-439).**
  RE-438 reads a partner end that an oblique mullion or a mullion across inside
  lies at as one Revit's solid keeps, but the pass that draws a mullion's
  `NetSurfaceArea` from its saved mesh refused every mullion with a kept count
  of 2 and a partner end, as it could not tell such a partner from a
  continuation on another curtain wall that the wall's boxes had missed. The
  boxes now say which of a mullion's ends is kept with a partner, the mesh's two
  ends are told by the side of the mullion's axis each lies on, and only a
  partner at another end refuses it. 34 mullions of the BIM4LCA concrete office
  (as built and at permit) and 33 of the timber office gain a `NetSurfaceArea`
  equal to Revit's, none different, and 4 of Snowdon Towers Architectural gain
  one (its Revit export has none). The other 36 and 36 of the 70 and 69
  mullions that keep a partner end lie beside a mullion the reader leaves
  unread (#1794), which the report of RE-438 had taken for the mesh path's
  refusals.
