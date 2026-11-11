- **Every box says why, on every release (#1645).** An element drawn as a record
  box, a beam's box or a turned box that no saved-mesh pass was offered (a
  Revit 2008 to 2022 file, whose saved graphics are not read; a caller that
  turned the pass off; a file whose release could not be read; a pass that
  stopped on an error) now has `not_offered: ` and the reason
  (`release_outside_2023_to_2027`, `saved_meshes_off`, `release_not_read`,
  `pass_failed`) as its `SavedMeshRefusal`, so that
  `exported.saved_mesh_refusals` counts every such box on every release. No body
  is drawn differently, and the files of Revit 2023 to 2027 do not change.
