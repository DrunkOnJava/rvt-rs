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
  a phase export the last phase as before. A handle exported at one phase and
  then at another no longer carries the first's notes (the family spaces left
  out for want of a body, the Areas left unwritten) into the second's
  diagnostics. `RevitFile::project_phases` is the
  new public method that lists a project's phases. The stubs
  (`python/rvt/__init__.pyi`), [docs/python.md](docs/python.md) and
  [docs/viewer-api.md](docs/viewer-api.md) follow, and RE1 Architecture
  checks it in Python (`tests/python/test_export_phase.py`) and in the
  browser (`viewer/tests/export-phase.spec.ts`).
