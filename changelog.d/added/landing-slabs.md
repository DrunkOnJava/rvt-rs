- **A monolithic stair landing is drawn as its slab (#824).** Each landing
  was its record box, which spans its stair's storey, 12.5 to 14.2 ft tall.
  On Revit 2024 a monolithic landing is now its slab: the outline the lines
  at its top in its own data make, from its top down its landing type's
  thickness. On the BIM4LCA office all 21 monolithic landings have Revit's
  top and outline, and 60 of its 63 slabs now have Revit's outline, against
  40. Where a flight meets a landing, Revit's underside follows the flight's
  soffit and rvt-rs draws it flat, so the slabs are 100.4% to 104.7% of
  Revit's volume. The body says so: `BodySource` `partition_landing_slab`.
