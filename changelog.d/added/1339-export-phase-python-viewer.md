- **The export phase in Python and the viewer, as `rvt-ifc --phase` has it
  (#1339, RE-326).** An export leaves out a room whose own phase is not the
  export phase, and the elements that do not exist in it; the export phase is
  the project's last unless told otherwise, and only `rvt-ifc` could tell it.
  Now `RevitFile.write_ifc`, `schedule_csv`, `export_diagnostics` and
  `export_diagnostics_json`, and `rvt_to_ifc` and `rvt_to_ifc_diagnostics`,
  take `phase: str | int | None = None`: a phase's name, matched exactly first,
  or its ElementId. A phase the file lacks raises `ValueError` listing the
  file's phases; one `RevitFile` can export different phases in turn, and a
  call without `phase` is the last phase again. In the browser,
  `limits.phase` of `openRvtBytesWithDiagnosticsModeAndOpenLimits` does the
  same, its result gains `phases` (`[{ id, name? }]`, in order, the last the
  default), and the File status panel lists the file's phases with the last
  as the default and reads the file again with the one chosen. Calls without
  a phase export the last phase as before. `rvt-schedule --phase <name|id>`
  schedules another phase, and Python's `schedule_csv(phase=...)` builds the
  model it builds (the document exporter at the phase set on the handle
  first). `RevitFile::project_phases` lists a project's phases and
  `RevitFile::clear_export_phase` goes back to the default; `set_export_phase`
  may now be called at any time on a handle, since choosing another phase drops
  the memoised production walk, which holds what the phase leaves out, so a
  handle exported at one phase and then another gives what a handle opened for
  that phase gives. The notes a walk makes for the diagnostics (family spaces
  left out for want of a body, Areas left unwritten, doors and windows written
  though their type was not read) now follow that walk, so exporting a handle
  twice at the same phase reports them both times. The stubs
  (`python/rvt/__init__.pyi`), [docs/python.md](docs/python.md) and
  [docs/viewer-api.md](docs/viewer-api.md) follow, and RE1 Architecture
  checks it in Python (`tests/python/test_export_phase.py`) and in the
  browser (`viewer/tests/export-phase.spec.ts`).
