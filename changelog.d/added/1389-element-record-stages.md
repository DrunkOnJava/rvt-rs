- **The diagnostics of a Revit 2008 to 2023 project count where its element
  records stop (#1389).** `decoded.element_record_stages` counts, per partition
  stream and in total, the markers found, those with a declared ElementId, those
  that gave a record, the placed instances, those with volume, the frames kept,
  the elements handed to the export and the elements the IFC model holds, each
  step exactly what the one before it passed on and left; the elements handed to
  the export are the elements written plus those set aside, each for one cause
  (an ElementId an earlier reader already held, a confidence below the floor, or,
  by Revit class, an element the exporter writes nothing for), and a stream that
  does not inflate is counted. The frames that stopped before they were records,
  and the placed instances with no volume, are also in `skipped` as
  `element_record_frame_stopped`, by stage (frames of every category, not lost
  elements), and, with `--export-decisions`, in the log of decisions with the
  place of their ElementId, 1,000 of each stage at most. When no record is
  written, `stopped_at` names the stage they stopped at, and `rvt-ifc` prints a
  line of its own with it. The rejected-frame reasons of 2024 and later counted
  nothing on these layouts, so a file such as the four on #421 that exports no
  element showed only `low_confidence_schema_scan_candidate`. The counts are
  integers added where the scan and the recovery already decide: no further pass
  over the file, and the default export lists no frame. A family file and a
  placeholder export have no report. The IFC is unchanged. On Autodesk's 33
  sample projects of 2015 to 2023 (Measure run 41932213967), every count added
  up, per stream and in total, and the IFC was byte-identical to main's; 192,811
  of their 197,463 markers gave a record, 87,366 elements were handed to the
  export and the models hold 87,315 of them. The causes for the 51 not written
  are counted after that run and not yet measured on this head.
  `tests/element_record_stages.rs` re-reads the markers on Einhoven and checks a
  file with no record and a family; Measure checks the sums on each sample
  project with `tools/re/element_stage_counts.py`.
