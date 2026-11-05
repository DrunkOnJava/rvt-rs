- **The saved-graphics reader reads deferred history episodes,
  `FacetedTopology9` under normal binding 2 and a set `m_bHasScale`
  (RE-342, #1158, #1190; STE1200, discussion #112).** STE1200 ran the probe on
  their Revit 2020 family of imported geometry and posted the counts: 109
  deferred episodes read, 325 `FacetedTopology9` objects under binding 2 whose
  47,968 stored normals are all within 0.1 degree of the winding, a set flag on
  26 identity and 10 rotation instances, and 36 owners drawn whole with 48,520
  triangles. That is a measurement reported on a file CI cannot fetch, and the
  only file that exercises the three. Each is read fail closed and narrower
  than their patch: deferred `Episode` pointers when the counted section
  repeats their GUIDs; `FacetedTopology9` under binding 2 only where every
  stored normal follows the winding (`normals_follow_winding`), and a mesh
  that fails is refused, never redrawn from the winding; a set `m_bHasScale`
  only on an `InstanceInfo` of `m_Trf`, `m_symbolId` and `m_cda` with a
  rotation matrix, so a mirror, a scale, or the flag on Revit 2023 to 2027
  stays refused. Saved graphics stay a research reading on Revit 2015 to 2022
  and no export reads them. Measure runs 42039865764, 42039870114,
  42039874650, 42039879159 and 42039914315 show what the changes leave
  unchanged on the files CI fetches; nothing in CI reaches the three paths, so
  nothing in CI will catch a regression in them.
