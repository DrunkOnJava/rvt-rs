- **Where each decoded element was read from, down to the byte, on request
  (#1325).** `rvt-ifc --diagnostics out.json --element-provenance`
  (`ExportOptions::with_element_provenance(true)`, Python's
  `export_diagnostics(element_provenance=True)`) adds
  `decoded.element_provenance` to the diagnostics, keyed by ElementId: the
  carrier of the element's record (a partition element record frame that
  starts its partition record, one inside another's record as RE-30 and
  RE-35 describe, a `Global/Latest` schema-walk instance, an ArcWall record,
  an Area chain record, or a row with no frame of its own), its stream
  (`Partitions/46`, `Global/Latest`) and byte offset in that stream inflated
  (the bytes `rvt-dump` writes as `Partitions_NN.decomp`, #291), where its
  ElementId is stored (`record_offset`, with the enclosing partition
  record's id for a second-prologue frame), its class tag and the class that
  names in the file's own schema, its `BuiltInCategory`, and the native
  record the export's native passes indexed for it (its partition, offset
  and class, and, where several partitions hold a copy, which is current:
  RE-145, RE-216). An element whose offset cannot be given is kept without
  one and counted by carrier (`by_carrier`, `unkeyed_elements`, `native`);
  the table lists at most 250,000 elements and counts the rest. It is off by
  default, since it is large: on the BIM4LCA office as built the table is
  4,462,401 bytes of compact JSON against 38,640 bytes of diagnostics
  without it (about 116 times), and the CLI's pretty-printed sidecar grows
  from 52,677 to 7,370,747 bytes, Core Interior's from 27,588 to 571,989,
  RE1 Architecture's from 13,956 to 85,470 (Measure run 41831418684).
  Without the switch the diagnostics lack the field and are the bytes they
  were, and the table is never written into the IFC. It is built from the
  walk, the inflated partitions and the native record index the export
  already holds. An `offset` is the frame the exporter selected among the
  element's frames (Revit 2024 and later: the greatest vertical extent, then
  the greater stream and offset; 2008 to 2023: the newest, with no extent
  test), which is not guaranteed to be the copy a body reader took (the
  sketch-line readers take the current one, RE-216), so it and the native
  record's stream can name different partitions. `rvt-rs-mcp` asks for it when it opens a model,
  keeps it beside the model, and returns the fields in every element's
  `provenance` (`carrier`, `stream`, `offset`, `record_offset`,
  `enclosing_record_id`, `class_tag`, `class_name`, `native_record`) and the
  file's `partition_count`, `layout_release` and `omitted_entries` (an
  element past the table's cap says `provenance_omitted`);
  `get_diagnostics` does not carry the table. `tests/element_provenance_offsets.rs` reads the bytes at
  every reported offset on Core Interior, the RE1 models and the BIM4LCA
  as-built office and checks the record decodes to the same ElementId, and
  prints the counts by carrier, the size of the table and the time to build
  it (CI's `element-provenance` artifact). Einhoven (Revit 2023) confirmed
  13 of 13 first-prologue frames and 71 of 71 native records; Revit 2008 to
  2022, and the second-prologue path of 2023, are not measured. The
  schema-walk carrier has no element on any of the seven models, so its
  offsets (into `Global/Latest` as the walk inflates it, where `rvt-dump`
  uses its own routine) have not been checked.
