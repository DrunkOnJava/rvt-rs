- **`--anonymize` for diagnostics you post publicly.** `rvt-ifc --diagnostics`
  and `rvt-inspect --json` name things the project names: the file and
  project names, the levels, a sample of the materials and of skipped
  elements, and, in `rvt-inspect`'s report, the folder path Revit stored for
  the file, of which only a `\Users\<name>\` part and Autodesk build and
  project-id folders were replaced. With
  `--anonymize` each is replaced with a placeholder (`<redacted>.rvt`,
  `Level 1`, `Material 1`, `element-<ElementId>`), the "Export to IFC As"
  values a warning lists are dropped, and counts, releases and ElementIds
  stay. The browser viewer's Download
  diagnostics now gives the anonymized file, named `rvt-diagnostics.json`.
  `ExportDiagnostics::anonymize` and `redact::anonymize_file_name` do it
  for library callers.
