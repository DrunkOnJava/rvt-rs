- **Python draws doors and windows from their saved meshes, as `rvt-ifc`
  does.** Since 0.11.0 `rvt-ifc` and `rvt-gltf` draw the doors and windows
  of Revit 2023 to 2027 files from the meshes Revit saved for them, where all
  of a mesh decodes; Python's `write_ifc()` and `rvt_to_ifc()` still drew
  their bounding boxes, so the same file gave a different IFC (found checking
  the 0.11.0 wheel against the CLI). They now draw the same bodies, and
  `export_diagnostics()`, `export_diagnostics_json()` and
  `rvt_to_ifc_diagnostics()` describe that export. `saved_meshes=False` on
  each keeps the boxes, as `rvt-ifc --no-saved-meshes` does; a
  `RuntimeWarning` says when the saved graphics could not be read.
  `rvt::ifc::saved_meshes::default_scope` gives the default to Rust callers.
