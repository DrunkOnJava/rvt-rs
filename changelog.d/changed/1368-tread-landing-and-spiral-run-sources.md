- **A tread landing and a spiral run name the type records they read, and the
  `current` copy rule is read in CI (#1368).** With `--element-provenance`, a
  landing drawn as its runs' tread now lists the landing type's record (its
  flags say it is drawn as a tread) and the record of each distinct run type of
  its stair's runs (the tread thickness), and a spiral run lists its run type's
  record (tread thickness, nosing, name), as a straight run already did. The
  export itself is unchanged: only the provenance table gains sources.
  - The `current` copy rule is on one of 92 models, the BIM4LCA office at
    building permit (the sketch lines of 3 of its floors, the Sketch curve list
    of one), and on none of the readers `instance_transform`, `sketch_line_data`
    or `roof_line_data` anywhere (Measure run 42111951132 at main d2439b09).
    CI's tier-two job now also fetches that permit file
    (`research/bim4lca-models.tsv`), and `tests/value_sources_offsets.rs` reads
    its sources at their bytes and fails if the office stops taking the rule.
  - No element has `selected_frame_box`, a body whose profile no profile reader
    noted, on any of the 92 models (the same run), so the text that called it a
    missing source is now the stated fallback it is. Four values still have no
    reader, with the reason for each in `docs/export-diagnostics.md`: a wall's
    layers, a beam's I section, a rectangular column's two sides and an Area's
    Levels.
  - Not yet measured on this head: the new sources on the office's tread
    landing and spiral run, and that the exports are byte for byte unchanged.
