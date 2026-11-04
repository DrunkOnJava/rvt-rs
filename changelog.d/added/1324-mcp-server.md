- **`rvt-rs-mcp`, a local, read-only MCP server over stdio, answers an
  agent's questions about a Revit file.** It is the new `rvt-mcp` workspace
  crate (its own `rust-version`, 1.88, rmcp's; `rvt` gains no async runtime and
  stays at 1.87) and targets MCP 2026-07-28, the stateless revision, as well
  as the older `initialize` handshake. `open_model` decodes a file once with
  the decode `rvt-ifc` runs and returns an opaque `model_id`; `get_summary`,
  `list_elements`, `get_element` (by ElementId or GlobalId), `list_rooms`,
  `list_levels` and `get_diagnostics` answer from that same model, so an
  answer and the IFC export cannot disagree, and each element carries a
  provenance object (ElementId, GlobalId, category, class, `BodySource`,
  `ProfileSource`, `SpaceOutlineSource`, `AreaBodyRefusal`, and the file's
  release, build, schema class count and ElemTable layout). `export_ifc` and
  `export_gltf` write to a path the caller names, refuse the model itself, and
  return a summary, the SHA-256 and a `resource_link`, with progress and
  cancellation. It opens no socket: its dependency tree has no HTTP crate (CI
  checks it) and its end-to-end test, which drives the built binary over real
  stdio in both lifecycles and compares its exports with `rvt-ifc`'s and
  `rvt-gltf`'s byte for byte, asserts the process holds none. The release
  archives and the container image carry the binary. Two small public
  functions let it name what the export writes:
  `ifc::step_writer::element_global_ids` (the writer's own GlobalIds) and
  `ifc::schedule_csv::room_rows` (the rooms of `rooms_csv`), with no change to
  any export. See [docs/mcp.md](docs/mcp.md) (#1324).
