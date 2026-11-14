- **`rvt-diff --elements` compares resolved values, not the rules that read
  them (#1864).** An element whose `LevelBindSource` alone differs, or whose
  `BodySource` differs while its body is the same, is counted unchanged and
  listed apart as a label change (`relabelled` in the JSON, each with its
  `labels`). Two saves of a project whose beams bind the same Level by two
  rules no longer show those beams as changed. A body that differs is still a
  change, with its `BodySource` beside it.
