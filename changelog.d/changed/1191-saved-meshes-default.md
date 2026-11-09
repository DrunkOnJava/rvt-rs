- **Every element that would be a bounding box in a Revit 2023 to 2027 file is
  drawn from the mesh Revit saved for it by default, where all of the mesh
  decodes, in `rvt-ifc`, `rvt-gltf`, Python, the MCP server and the viewer;
  `--no-saved-meshes` keeps the boxes (#1191).** Doors and windows alone were
  until now, and furniture, fixtures, equipment, MEP devices and the rest took
  `--saved-meshes`. The BIM4LCA office as built draws 11,078 of its 11,246
  boxed bodies from their meshes, 568,885 triangles against 257,108, and its
  IFC is 83 MB against 68. **Cost:** reading every box's saved graphics takes
  1% to 17% more time and up to 59% more peak memory on one runner (Measure run
  42614230465, median of 3): the office as built 24.4 s and 1,423 MiB against
  21.2 s and 913 MiB, RE1 Electrical 1.94 s and 264 MiB against 1.75 s and 166
  MiB, and Core Interior 4.28 s and 305 MiB against 4.23 s and 305 MiB. In the
  viewer's Playwright measurement (viewer tests run 42631322761, one open
  each) the wasm memory with the meshes on is the boxes' on Einhoven (17 MiB),
  RE1 Mechanical (68) and Plumbing (54), 16 MiB more on Core Interior (308 to
  324, as with doors and windows alone), 21 MiB more on RE1 Architecture (96 to
  118) and 52 MiB more on RE1 Electrical (161 to 213), and the wasm open call
  takes at most 0.52 s longer (RE1 Electrical). **Not every
  mesh is Revit's:** of the bodies scored on 31 models (Measure run
  42612357300) 17 classes have every vertex of Revit's export within 0.001 ft
  of the mesh, but doors are 675 of 681, sanitary terminals 68 of 94, flow
  fittings 71 of 89, flow terminals 30 of 35 and walls 14 of 18. No roof, beam,
  slab, footing or stair flight mesh is among the bodies scored on those 31
  models: on Autodesk's samples the meshes of four roofs of the 2026 and 2027
  saves are not the 2025 save's bodies (the same-project gate names them), and
  `rac_advanced`'s eight beams are held to its 2024 save. A mesh with a face
  that does not decode still keeps its box. **A very large file may need
  more memory than before:** it depends on how many boxed elements the file
  holds, and none over 1.4 GiB has been measured. The opt-out in each interface
  returns the previous memory: `--no-saved-meshes` in `rvt-ifc` and `rvt-gltf`,
  `saved_meshes=False` in Python, `saved_meshes: "off"` in the MCP server,
  `ExportOptions::with_saved_meshes(SavedMeshChoice::Off)` in the Rust library,
  `savedMeshes: false` in the wasm binding, and the viewer's option or
  `?savedMeshes=0`. `SavedMeshChoice::DoorsAndWindows` (new) and the MCP's
  `saved_meshes: "doors_and_windows"` give the previous default. **For library
  callers** `SavedMeshChoice::Default` and `saved_meshes::default_scope` now mean
  every boxed element, not doors and windows alone.
