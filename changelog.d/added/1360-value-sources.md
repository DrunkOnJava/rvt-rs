- **The copy each body reader took its value from, in the provenance table
  (#1360).** With `rvt-ifc --element-provenance` (and the MCP server, which
  asks for the table), an element's entry in `decoded.element_provenance` now
  lists `value_sources`: for each reader that took a value of its body from a
  copy other than the frame the entry's `offset` names, the reader, the rule
  that chose the copy (`selected`, `agreeing`, `current` or `native`), how many
  partitions held a copy, the copy's stream and offset, and whether it is in
  another partition than the frame. A reader notes its choice where it makes it
  (the copy-aware scans of a wall's or beam's location line, of a sketch's
  curves and of a family instance's transform, and the native record readers
  of saved graphics, stair runs and landings), only while the table is being
  built, so the default export does nothing more and its IFC and diagnostics
  are byte for byte what they were. The table's `value_source_counts` counts,
  by reader, the elements whose source is in another partition than their
  frame (for an `agreeing` copy, in none of the partitions that hold one). The
  entry lists a source only where the element's final `BodySource` is drawn
  from that reader. A body that is the record's box but whose profile is read
  from sketch lines or the type's section (floors, columns) names the frame as
  `selected_frame_box`, for the box and the placement only; the profile has no
  source yet. The MCP `get_element` and `explain_element` list
  the sources in the element's provenance; `list_elements` does not. This
  replaces the #1325 caveat that the offset may not be the copy a body came
  from. `tests/value_sources_offsets.rs` reads the bytes at each recorded
  offset on Core Interior, the RE1 models, Einhoven and the BIM4LCA as-built
  office, and names every `BodySource` on them as covered or not yet. No
  reader notes its copy yet for area boundary lines, hollow and I sections,
  pipe axes, a stair run drawn from its sketch, roof slopes and extrusions, or
  family extrusions; those elements have no value source, which is not a claim
  that they read the frame. Not measured: that the copy
  taken is the right one (`agreeing` copies are equal; `current` is
  `current_partitions`).
