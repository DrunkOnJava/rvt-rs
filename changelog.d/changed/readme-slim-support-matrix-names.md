- **`rvt-capabilities --matrix` names the Revit 2023 and 2024 project
  capabilities for what they give.** `element-records-revit-2023` was
  "Revit 2023 elements from their partition element records: identity,
  category and box (RE-81)", though 2023 projects also get wall joins, beam
  cuts, room outlines, storeys, hosts, family and type names, materials and
  layer sets; `typed-project-elements`, the Revit 2024 row, was "Decode typed
  elements from real project files". Only the two names change, in the text
  and JSON of the built-in matrix; ids, statuses and evidence do not. The
  README's support-by-release table is now rendered from the matrix, and CI
  fails when the two disagree (`tools/ci/readme-matrix.py`).
