- **A per-release table in `docs/support-matrix.json`.** `releases` lists
  Revit 2015 to 2027, oldest first. For each release it names the row that
  rates its projects' element records (the row `rvt::capability` already
  cites) and the rows measured on its family files (none for 2015 and 2027,
  which the family corpus does not have). Statuses stay in the capabilities.
  `rvt-capabilities --matrix` prints the table, and
  `tests/support_matrix.rs` pins it to the crate and the family corpus.
