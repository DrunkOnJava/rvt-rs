- **`ifc::run_export`, one function that builds the model `rvt-ifc` writes,
  which `rvt-ifc`, the Python package, the viewer and `rvt-rs-mcp` all call
  (#1331).** The steps were written out four times: the exporter, the project
  phase, the bodies drawn from saved graphics, Revit's property sets and their
  placement, the base-quantity switch and the record of streams a resource
  limit kept unread. `run_export(&mut rf, &ExportOptions, limits, &control)`
  takes the switches `rvt-ifc` has, `--phase` included
  (`ExportOptions::with_phase`), and returns an `ExportRun`: the model, its
  diagnostics, the unread streams and the notes `rvt-ifc` prints
  (`ExportNote`, which says which step it reports and why a step was skipped).
  The four callers keep their defaults and their signatures, and every export
  is byte for byte what it was. `rvt-rs-mcp` gains an optional `phase` argument
  on its tools that take a path: a model is opened at a phase, has its own
  `model_id`, and exports the bytes `rvt-ifc --phase` writes. `ExportKind`,
  `ExportOptions`, `ExportNote`, `ExportRun`, `ExportStep` and
  `SavedMeshChoice` are new public items.
