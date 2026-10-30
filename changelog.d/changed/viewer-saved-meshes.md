- **The viewer draws doors and windows from their saved meshes, with a
  toggle.** Since 0.11.0 `rvt-ifc` and `rvt-gltf` draw the doors and windows of
  Revit 2023 to 2027 files from the meshes Revit saved for them, where all of a
  mesh decodes; the browser viewer still drew their bounding boxes, so the
  model on screen, its glTF and its IFC differed from the command line's for
  the same file. It now draws the same bodies, and File status has a "Draw
  doors and windows from saved meshes" option, on by default; changing it
  reopens the open file, and `?savedMeshes=0` in the address starts with it
  off. `openRvtBytesWithDiagnosticsModeAndOpenLimits` takes `savedMeshes` for
  it. The cost, measured in Playwright (Chromium on a 4-CPU runner, one open
  each, `tests/saved-meshes-measure.spec.ts`, which Deploy viewer runs and
  uploads): Core Interior (32 MiB, 138 doors and windows, all drawn from
  their meshes) loads in 25.6 s against 24.5 s with boxes, and the wasm
  memory after the load is 324 MiB either way; Einhoven and RE1
  Architecture load 0.1 s and 0.3 s slower with the same memory, and the three
  RE1 models with no doors or windows read nothing extra.
