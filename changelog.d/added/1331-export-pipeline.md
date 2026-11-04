- **`ifc::run_export`, one function that builds the model `rvt-ifc` writes,
  which `rvt-ifc`, `rvt-gltf`, the Python package, the viewer and
  `rvt-rs-mcp` all call (#1331).** The steps were written out five times: the
  exporter, the project phase, the bodies drawn from saved graphics, Revit's
  property sets and their placement, the base-quantity switch and the record of
  streams a resource limit kept unread. `run_export(&mut rf, &ExportOptions)`
  takes the switches `rvt-ifc` has, `--phase` included
  (`ExportOptions::with_phase`), and returns an `ExportRun`: the model, its
  diagnostics, the unread streams and the notes `rvt-ifc` prints (`ExportNote`,
  whose `Display` is the sentence, and whose `...Skipped` variants say why a
  step did not run). `run_export_with_limits` adds the walker limits and
  `run_export_with_limits_and_control` a `WalkerControl`, as the exporters'
  own `..._with_limits` and `..._with_control` entry points do.
  `RevitFile::export_phase` reports the ElementId that `set_export_phase`
  resolved from a phase's name or number (`None` while the last phase is in
  effect).
  The callers keep their defaults and their signatures. `rvt-rs-mcp` gains an
  optional `phase` argument on its tools that take a path: a model is opened
  at a phase, has its own `model_id` (keyed by the phase's ElementId, so a
  name and its number are one model), and exports what `rvt-ifc --phase`
  writes. Two behaviors move, neither expected to change an export:
  `rvt-ifc` prints its step notes after the decode instead of between steps
  (stderr only, the same text), and a cancelled `WalkerControl` now ends an
  export with `Error::Cancelled` also when it stops the saved-graphics step,
  where the viewer's bindings used to turn that into a diagnostics warning and
  carry on (the viewer cannot cancel a decode, it ends its worker, so it is
  not affected). `ExportKind`, `ExportOptions`, `ExportNote`, `ExportRun` and
  `SavedMeshChoice` are new public items.
