- **`rvt-capabilities --matrix` names the Revit 2023 and 2024 project
  capabilities for what they give.** `element-records-revit-2023` was
  "Revit 2023 elements from their partition element records: identity,
  category and box (RE-81)", though 2023 projects also get wall joins, beam
  cuts, room outlines, storeys, hosts, family and type names, materials and
  layer sets; `typed-project-elements`, the Revit 2024 row, was "Decode typed
  elements from real project files". The two names change, and their notes,
  which still said names, types, storeys and joins were not read on 2023, in
  the text and JSON of the built-in matrix; ids, statuses and evidence do not.
  `rvt-doc`'s note on older releases names the reconnaissance document by its
  new path, `docs/format-reconnaissance.md`. The
  README's support-by-release table is now rendered from the matrix, and CI
  fails when the two disagree (`tools/ci/readme-matrix.py`).
