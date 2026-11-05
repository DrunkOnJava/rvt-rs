- **The saved-graphics probe reads deferred history episodes,
  `FacetedTopology9` under normal binding 2 and a set `m_bHasScale`
  (RE-342, #1158, #1190; STE1200, discussion #112).** STE1200 ran the probe on
  their Revit 2020 family of imported geometry and posted the counts: 109
  deferred episodes read, 325 `FacetedTopology9` objects under binding 2 whose
  47,968 stored normals are all within 0.1 degree of the winding, a set flag on
  26 identity and 10 rotation instances, and 36 owners drawn whole with 48,520
  triangles. That is a measurement reported on a file CI cannot fetch, and the
  only file that exercises the three. Each is read fail closed, narrower than
  their patch, and on a route no export takes. Deferred `Episode` pointers are
  read only when `Options::deferred_history_episodes` is set, which only the
  probe does; every other reader, the whole native preamble included, refuses
  such a history as before. `FacetedTopology9` under binding 2 is read only in
  the graphics of a file of Revit 2015 to 2022, and only where every stored
  normal follows the winding (`normals_follow_winding`); a mesh that fails is
  refused, never redrawn from the winding. A set `m_bHasScale` is read only on
  an `InstanceInfo` of `m_Trf`, `m_symbolId` and `m_cda` with a rotation
  matrix, so a mirror, a scale, or the flag on Revit 2023 to 2027 stays
  refused. What the flag means on the older schema is not read: if it marks a
  scale the matrix does not carry, a mesh drawn through it is wrong and nothing
  in the file says so. In one held-out model 57 owners pass the scale check and
  end with no primitive, and what they hold is not known. Saved graphics stay a
  research reading on Revit 2015 to 2022 and no export reads them. Measure runs
  show what the changes leave unchanged on the files CI fetches (42039865764,
  42039870114, 42039874650, 42039879159, 42039914315, and the same five on the
  head with the gating: 42054602432, 42054606416, 42054610905, 42054615619,
  42054620459); nothing in CI reaches the three paths, so nothing in CI will
  catch a regression in them.
