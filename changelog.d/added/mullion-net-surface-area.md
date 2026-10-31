- **Curtain mullions' `NetSurfaceArea` (#1046, RE-268).** Revit's export
  sums the faces of a mullion's Revit solid, so an end face is not counted
  where the mullion meets its own continuation end to end, nor where it stops
  inside a crossing mullion, and is counted where it butts against a crossing
  mullion's side face. rvt-rs's boxes of a curtain wall's mullions are those
  solids, so it reads which ends are kept from where they meet and writes
  `Qto_MemberBaseQuantities.NetSurfaceArea` as the section's perimeter times
  the length plus that many of the section's areas, on the mullions drawn as
  an extrusion of their section (RE-243). On the BIM4LCA office that is 6,606
  of 10,003 mullions as built and 6,636 of 10,031 at building permit, each
  Revit's value. Left out are the mullions with an end that meets a mullion
  lying over another, an oblique one, a crossing one that holds the end off
  its axis and faces, one shorter than its section or one left out for
  these reasons, and every mullion on a curved or edited curtain wall.
