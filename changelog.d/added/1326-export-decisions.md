- **Why an element, body, opening or outline was left out, one entry per
  element, on request (#1326).** `rvt-ifc --diagnostics out.json
  --export-decisions` (`ExportOptions::with_export_decisions(true)`, Python's
  `export_diagnostics(export_decisions=True)` and `rvt_to_ifc_diagnostics`)
  adds `export_decisions` to the diagnostics. Each fail-closed decision is an
  entry with the element's ElementId, a stage (`element_skipped`,
  `body_not_drawn`, `body_left_as_box`, `opening_not_cut`, `outline_not_read`,
  `storey_not_bound`), a reason code, the class the diagnostics count it under,
  and the stream and byte offset its record was read from where known (a frame
  with no ElementId has its place and no id; an element the walk decoded takes
  its place from its provenance). The `skipped` section is now counted from the
  same decisions and is unchanged: every item of it, the geometry gaps among
  them, has a reason in the log whose `total` is its count, and the decisions
  behind `exported.bounding_box_bodies`, `bounding_box_spaces` and
  `area_body_refusals` are in the log too (`body_is_record_box`,
  `space_outline_is_record_box`, `area_body_refused`). The log keeps at most
  5,000 entries a reason, by ElementId, and counts what it drops per reason and
  in all. It is a switch of its own, not `--element-provenance`: the log is
  bounded where the provenance table is not, and needs no table in the
  diagnostics. Without the switch the diagnostics lack the field and are the
  bytes they were, and neither goes into the IFC. The stage for a value left
  out because Revit's exporters disagree exists and records nothing: those
  omissions are rules for a whole class of element in the reader that would
  write the value, not decisions made per element, and nothing counts them.
  `rvt-rs-mcp` asks for the log when it opens a model, keeps it beside the model
  and has a new tool, `explain_element(model, element)`: the element's
  provenance and trust markers where the export holds it, and every decision
  recorded for it, whether or not it is exported (`capped_reasons` names the
  reasons that dropped entries). `tests/export_decisions_counts.rs` checks, on
  Core Interior, the RE1 models, Einhoven and the BIM4LCA as-built office, that
  every reason of `skipped` has equal counts in the log and that the entries by
  class are its classes, re-reads the bytes at the frame of each entry of the
  seven element-record frame reasons on Revit 2024 and later, and prints the
  counts per reason (CI's `export-decisions` artifact). The offsets of Revit 2015
  to 2023 files and of `Global/Latest` entries are not verified, and a scan that
  finds an element in several partitions names its last frame, which can differ
  from the frame the provenance table names. `--anonymize` scrubs the log's
  phase names as it scrubs `skipped`. Core Interior has nothing left out, so its
  log is empty; the other models name reasons that must have entries.
