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
  - Measured on this head (ee449c3b): the 31 exports of Measure run 42118484766
    (no `ifc_flags`; Core Interior, Einhoven, the four RE1 models and
    Autodesk's samples of 2024 to 2027) are byte for byte main's
    (`ifc.sha256`), so the default export is unchanged, and its
    `same-project gates` job passed. Run 42118489328 (the 92 models with
    `--element-provenance`) counts the new sources: `wall_type_layers` on
    1,973 elements of 29 models, `area_levels` on 483 Areas of 17,
    `type_i_section` on 557 elements of 5 (none on a CI gate model),
    `native_run_type` on 84 of 17 and `native_landing_type` on 82 of 8;
    `current` stays on the one permit office (the 2021 `rac_advanced`, after
    #1450, has none: its 25 Areas read `area_plan_topology`) and
    `selected_frame_box` on none. Not yet run:
    `tests/value_sources_offsets.rs` against the permit office and the
    new readers.
