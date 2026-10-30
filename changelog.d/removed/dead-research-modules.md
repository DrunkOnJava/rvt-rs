- **Three research modules nothing used are removed.**
  `rvt::research::class_tag_map` (a 2016 to 2026 tag lookup from
  `docs/data/tag-drift-2016-2026.csv`; `formats::SchemaClasses` reads each
  file's own tags), `compound_framing` (a marker tokenizer for the RE-15
  compound ArcWall study) and `relations` (typed-relation domains its own
  documentation called architecture stubs, wired into nothing). They were
  behind `unstable-research`, outside the semver promise. The CSV stays for
  `examples/tag_drift_svg.rs`.
