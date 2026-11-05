- **The provenance table names the records behind a wall's layers, an I-section
  beam, an Area's Levels, a tread landing and a spiral run, and the `current`
  copy rule is read in CI (#1368).** With `--element-provenance`:
  - a wall drawn at its type's thickness or in its layers lists the type's
    layers (`wall_type_layers`), a beam or column drawn as its type's I section
    lists the type's six section entries (`type_i_section`, before listed for
    a column only), and an Area lists the block of every Level its base and
    height compare (`area_levels`, one part for each Level);
  - a landing drawn as its runs' tread lists the landing type's record and the
    record of each distinct run type of its stair's runs, and a spiral run
    lists its run type's record, as a straight run already did.
  - The export itself is unchanged: only the provenance table gains sources.
  - The `current` copy rule is on one of the 92 models Measure run 42111951132
    read, the BIM4LCA office at building permit (the sketch lines of 3 of its
    floors, the Sketch curve list of one), and on none of the readers
    `instance_transform`, `sketch_line_data` or `roof_line_data`. CI's tier-two
    job now also fetches that permit file (`research/bim4lca-models.tsv`), and
    `tests/value_sources_offsets.rs` reads its sources at their bytes and fails
    if the office stops taking the rule.
  - No element has `selected_frame_box`, a body whose profile no profile reader
    noted, on any of the 92 models (the same run), so the text that called it a
    missing source is now the stated fallback it is. One value still has no
    source, with its reason in `docs/export-diagnostics.md`: a rectangular
    column's two sides, which only decide whether its box quantities are
    written.
  - Not yet measured on this head: that the exports are byte for byte
    unchanged, and the counts of the new sources.
