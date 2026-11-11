- **A curtain mullion drawn from its saved mesh gets Revit's `NetSurfaceArea` (#1046, RE-404).**
  Where its solid's flat ends are one section each, the mullion's
  `Qto_MemberBaseQuantities.NetSurfaceArea` is the area of the faces along its
  axis plus the end faces its solid keeps (RE-268's count, now read for a
  mullion on any host, where the box's own value was written only on a plain
  one); a mullion with a mitred end gets its lateral area plus the mitre face,
  and the flat end where that is kept. Revit's export writes the same on the
  mullions of the BIM4LCA housing and office whose body rvt-rs draws from the
  mesh Revit saved.
