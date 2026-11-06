- **Core Interior's 20 shading devices were never saved-mesh bodies on
  `main`: RE-78's 158 was the count before RE-95 (#1492).** RE-78 counted 158
  saved-mesh bodies on Core Interior (132 doors, 6 windows, 20 shading
  devices), and `mesh_bodies_vs_ifc` and `rvt-ifc --saved-meshes` count 138.
  Nothing changed on `main`. The 158 was measured on 2026-09-27, when the 20
  plates and the 116 rooms were record boxes; RE-95 (#467) closed the plates'
  sketch outlines and RE-101 (#473) gave the rooms their stored-solid
  outlines on 2026-09-28, both before the saved-mesh option landed on
  2026-09-30, and `saved_meshes::attach` replaces only a body whose profile is
  not resolved, as it did when it landed (73861770). No body that measured
  exact was lost, and RE-78 now says so under its first table. The new
  `tools/re/shading_bodies_vs_ifc.py` scores the plates' bodies by shape
  against Revit's, which neither `mesh_bodies_vs_ifc` (saved meshes only) nor
  `plan_profiles_vs_ifc` (plan outline only) did: on Core Interior all 20
  are within 0.01% of Revit's volume and every vertex within 0.001 ft of its
  body's surface (the largest distance 1.5e-06 ft), Measure run 42133202790.
