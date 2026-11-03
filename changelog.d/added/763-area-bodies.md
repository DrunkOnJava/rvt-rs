- **A Revit 2024 Area's `IfcSpace` has a body: the region its area boundary
  lines enclose, from its Level up to the Level above it.** The Area stores no
  boundary; rvt-rs takes the lines of its area scheme on its Level, splits
  them where they meet or cross, leaves out the ones with a free end, and
  draws the loop that holds the Area's point, with the loops directly inside it
  as holes, as Revit's export draws it. On the BIM4LCA office all 30 Areas, as
  built and at building permit, equal Revit's in area (within 0.1%) and in
  outline (within 0.01 ft), where Measure scored none of them before. An Area
  keeps no body, as before, and says why in
  `RvtElementRecordGeometry.AreaBodyRefusal` and `exported.area_body_refusals`,
  wherever its boundary is not proven: a boundary line that is not straight
  along the plan axes, or that was not read, a point in no single region, two
  Areas with one region, or a Level that is not a building story. Not
  measured: the top storey (no Level above), Levels that are not building
  stories, a Level whose "Story Above" is not the next Level up (never read),
  Areas bounded partly by walls, and releases other than 2024.
  `SpaceOutlineSource` is `area_boundary` on a drawn Area and `none` on one that
  is not; the room schedule gives it an area. An Area does not hold a room's
  furniture. `Qto_SpaceBaseQuantities.Height`, which Revit's export writes on an
  Area, is not written yet ([docs/rooms.md](docs/rooms.md), #763, RE-317).
