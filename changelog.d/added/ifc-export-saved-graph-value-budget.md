- **IFC export** accepts per-graph Saved graphics limits through
  `ExportOptions::with_saved_mesh_limits` and
  `rvt-ifc --saved-mesh-graph-values`. The default is 1,000,000 field values
  per graph. A graph that exceeds its limit keeps the ordinary fallback.
