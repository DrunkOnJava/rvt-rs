- **Doors and windows are drawn from the meshes Revit saved for them, by
  default.** `rvt-ifc` and `rvt-gltf` now draw every door and window whose
  body would be its record's bounding box from its saved mesh, on Revit
  2023 to 2027, where the whole mesh decodes: every vertex within 0.001 ft
  of Revit's export on all 132 of Core Interior's doors and 6 windows and
  all 269 of the BIM4LCA office's windows (Measure run 40932136360). A door
  whose saved graphics do not decode whole keeps its box (221 of the
  office's 269 doors). `--no-saved-meshes` keeps every box; `--saved-meshes`
  still draws every other family from its mesh too.
  `saved_meshes::attach_scoped_with_control` with `SavedMeshScope` does the
  same from Rust; Python's `write_ifc()` and the viewer are unchanged.
