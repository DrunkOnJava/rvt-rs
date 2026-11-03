- **A Revit 2024 Area's `IfcSpace` has a body: the region its area boundary
  lines enclose, from its Level to the next storey.** The Area stores no
  boundary; rvt-rs takes the lines of its area scheme on its Level, splits
  them where they meet or cross, leaves out the ones with a free end, and
  draws the loop that holds the Area's point, with the loops directly inside it
  as holes, as Revit's export draws it. On the BIM4LCA office all 30 Areas, as
  built and at building permit, equal Revit's in area (within 0.1%) and in
  outline (within 0.01 ft), where Measure scored none of them before. An Area
  whose boundary lines are not all straight along the plan axes, or whose
  point lies in no single region they bound, keeps no body, as before.
  `RvtElementRecordGeometry.SpaceOutlineSource` is `area_boundary` on a drawn
  Area, and `none` on one that is not; the room schedule gives it an area. An
  Area does not hold a room's furniture. Revit 2024 only; `Qto_SpaceBaseQuantities.Height`,
  which Revit's export writes on an Area, is not written yet
  ([docs/rooms.md](docs/rooms.md), #763, RE-317).
