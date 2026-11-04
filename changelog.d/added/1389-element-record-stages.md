- **The diagnostics of a Revit 2008 to 2023 file count where its element
  records stop (#1389).** `decoded.element_record_stages` counts, per partition
  stream and in total, the markers found, those with a declared ElementId, those
  that gave a record, the placed instances, those with volume, the frames kept,
  the elements handed to the export and the elements the IFC model holds, each
  step exactly what the one before it passed on and left. The frames that
  stopped before they were records, and the placed instances with no volume, are
  also in `skipped` as `element_record_frame_stopped`, by stage, and in the log
  of decisions with the place of their ElementId. When no record is written,
  `stopped_at` names the stage they stopped at, and `rvt-ifc` prints a line with
  it. The rejected-frame reasons of 2024 and later counted nothing on these
  layouts, so a file such as the four on #421 that exports no element showed only
  `low_confidence_schema_scan_candidate`. The counts come from the scan and the
  recovery as they decide, with no further pass over the file; the IFC is
  unchanged. `tests/element_record_stages.rs` re-reads the markers on Einhoven
  and checks a file with no record; Measure checks the sums on each sample
  project with `tools/re/element_stage_counts.py`. Not yet measured on this head:
  the counts on the sample projects.
