- **The saved-graphics reader reads an `InstanceInfo` with no `m_GRepId`
  (RE-342, #1158, #1190; STE1200, discussion #112).** Revit 2015 to 2022's
  `InstanceInfo` has no `m_GRepId`, which Revit 2023 to 2027 set to 0 in all
  220,686 of the objects measured; the readers took its absence for an
  unsupported representation, and now take it for the default one. With
  `Options::releases` widened to `NATIVE_RELEASES`, `native_saved_scene`
  draws 5,459 to 5,756 of the 6,241 to 6,433 `FamilyInstance` elements of
  the three sample projects of each release 2015 to 2022, where it drew 68
  to 70. On the 2021 and 2022 projects 11,482 of the 11,482 elements both
  that file and its 2023 copy draw whole have the same triangles, world box
  and surface area, every one exactly; so do all of the `rac_basic` of 2016
  to 2022 and the `rst_basic` of 2019 to 2022; 3,662 of 45,349 do not over
  the 24 pairs (the `rac_advanced` saves of 2015 to 2020, the `rst_basic`
  of 2016 to 2018 and the 2015 `rac_basic`, within 1 ft, not traced to a
  cause). No export changes (92 of 92 IFC hashes equal `main`'s, with and
  without `--saved-meshes`), and saved graphics stay a research reading on
  2015 to 2022. STE1200's
  patch also read deferred history episodes, `FacetedTopology9` under normal
  binding 2 and a set `m_bHasScale`; no file Measure fetches has the first
  two (0 of 93 open files, 0 of 18 held-out models, 0 of 13 families) or
  draws a body from the third, so they are not in this change. The probe
  `probe_re342_saved_graphics_pre2023` counts what each meets in a file,
  and runs on your own: Measure runs 42015502520, 42015507053, 42015511974
  and 42015516241.
