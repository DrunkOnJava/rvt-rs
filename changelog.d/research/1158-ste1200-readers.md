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
  refused. This also narrows #1436: an `InstanceInfo` with no `m_GRepId`,
  which it let every route read as the default representation, is now read on
  the pre-2023 route alone and refused on every other, as before. What the flag means on the older schema is not read: if it marks a
  scale the matrix does not carry, a mesh drawn through it is wrong and nothing
  in the file says so. In one held-out model 57 owners pass the scale check and
  end with no primitive, and what they hold is not known. Saved graphics stay a
  research reading on Revit 2015 to 2022 and no export reads them. At
  `997d5da1`, before later changes that the head has not been measured with
  (the route gates, the allocation and the strict winding count), the changes
  altered no export and drew no body through a set flag on the files CI
  fetches: 92 of 92 export
  hashes were identical to `main`'s, with and without `--saved-meshes` (Measure
  runs 42054602432 and 42054606416), and the owners drawn whole and their
  triangles were unchanged in the pairs and the 8 held-out scenes (42054610905,
  42054620459). Their stop histograms do move against #1436, and owners gain a
  partial primitive, with none drawn whole: 29 owners change their stops in 15
  of the 24 pair files (21 leave the scale-check stop, 8 gain a `GText` stop and
  a partial primitive), and in 3 of the 8 held-out scenes 1, 58 and 2 owners do,
  two models gaining a partial primitive and 57 of the 58 in one model going from
  partial to `decoded_saved_graphics` with no primitive (182 to 239). No file CI
  fetches holds a deferred pointer or a `FacetedTopology9` object (0 of 92 open
  files and 18 held-out models), so nothing in CI reaches those two paths and
  nothing in CI will catch a regression in them.
