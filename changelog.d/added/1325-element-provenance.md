- **Where each decoded element was read from, down to the byte (#1325).**
  The export diagnostics (`rvt-ifc --diagnostics`, `ExportRun.diagnostics`,
  `rvt-py`, `rvt-wasm`) gain `decoded.element_provenance`, keyed by
  ElementId: the carrier of the element's record (a partition element
  record frame that starts its partition record, one inside another's
  record as RE-30 and RE-35 describe, a `Global/Latest` schema-walk
  instance, an ArcWall record, an Area chain record, or a row with no frame
  of its own), its stream (`Partitions/46`, `Global/Latest`) and byte
  offset in that stream inflated (the bytes `rvt-dump` writes as
  `Partitions_NN.decomp`, #291), where its ElementId is stored
  (`record_offset`, with the enclosing partition record's id for a
  second-prologue frame), its class tag and the class that names in the
  file's own schema, its `BuiltInCategory`, and the native record the
  export's native passes indexed for it (its partition, offset and class,
  and, where several partitions hold a copy, which is current: RE-145,
  RE-216). An element whose offset cannot be given is kept without one and
  counted by carrier (`by_carrier`, `unkeyed_elements`, `native`); the
  table lists at most 250,000 elements and counts the rest. It is built
  from the walk, the inflated partitions and the native record index the
  export already holds, and it is never written into the IFC. `rvt-rs-mcp`
  returns the
  fields in every element's `provenance` (`carrier`, `stream`, `offset`,
  `record_offset`, `enclosing_record_id`, `class_tag`, `class_name`,
  `native_record`) and the file's `partition_count` and `layout_release`.
  `tests/element_provenance_offsets.rs` reads the bytes at every reported
  offset on Core Interior, the RE1 models and the BIM4LCA as-built office
  and checks the record decodes to the same ElementId, and prints the
  counts by carrier and the table's size in bytes as JSON (CI's
  `element-provenance` artifact). The Revit 2008 to 2023 layout is
  reported but not measured by it.
