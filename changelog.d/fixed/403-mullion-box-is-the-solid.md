- **A curtain mullion's box gives its `NetVolume` and `NetSurfaceArea` only
  where Revit's solid is that box (RE-403, #1046).** A mullion whose box volume
  differs from its saved mesh's, as a mullion with a 45 degree end does, takes
  the mesh's volume, which is Revit's, and loses its `NetSurfaceArea`; an
  overlapped mullion is read only where what lies over it is of its type. RE-400
  had written 24 wrong values on the BIM4LCA timber office at building permit.
