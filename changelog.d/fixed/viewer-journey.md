- **The viewer's downloaded diagnostics keep their per-type counts, and its
  Shapes row appears.** The bindings hand a Rust map over as a JavaScript
  `Map`, which `JSON.stringify` writes as `{}`. So `exported.by_ifc_type`,
  `body_sources` and the other maps were empty in the diagnostics the viewer
  showed and saved. The status panel's Shapes row, which says how many
  bodies are bounding boxes (#409), summed an empty list and never appeared.
  The viewer now turns the diagnostics' maps into objects when it receives
  them, and `docs/viewer-api.md` no longer says results are plain JSON.
