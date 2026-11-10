- **The mesh body scorer reads a round section's volume and the faces Revit's
  export writes twice (#1658, RE-397).** `mesh_bodies_vs_ifc.py` read a volume
  ratio outside 1% as a difference where none is: Revit's export draws a round
  section as a polygon inscribed in the circle a saved mesh draws with more
  segments (a ratio of 1.1989 for a hexagon on the BIM4LCA housing's 30 mm rails,
  1.60 for the quadrilateral on the office's `KAI-3` posts, 0.9643 where rvt-rs's
  circle has the fewer segments), and it writes some bodies with every face
  twice in the same winding, so that their volume is twice the body's (the roofs
  and ramp flights that read exactly 0.5000). A body outside 1% is now read
  shell by shell (`volume_readings.py`): a shell is `explained` only where
  Revit's end faces are polygons inscribed in rvt-rs's circle (to 0.001 ft) and
  its volume ratio is their area ratio, and `doubled` only where its volume is
  rvt-rs's once Revit's repeated triangles count once and every vertex of each
  lies within 0.01 ft of the other's surface; every other difference stays
  `other`. The class table is unchanged; a `shell readings:` line and `reading`
  rows follow it. On the BIM4LCA files, all 12 and 16 outlying railings of the
  housing, the office's `KAI-3` and `KAI-5` railings and every roof and ramp
  flight at 0.5000 or 0.5010 are explained; the housing's ramp flight, the
  doors with a shell Revit's export lacks and the walls stay `other`.
