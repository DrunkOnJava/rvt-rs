- **Value sources for profiles, stair runs and landings (#1368).** Under
  `--element-provenance` an element whose body is its record's box but whose
  profile was solved from sketch lines (floors, slabs, roofs) or taken from
  its type symbol's box (columns) now lists where the profile came from, as
  well as `selected_frame` for the box: `sketch_lines` (`parts`: the frame of
  each sketch line the solve was given, the first of its frames by frame
  order, or the current partition's where they disagree) and `sketch_list`
  (the Sketch's curve list, with the rule that chose its copy), or
  `type_section` (rule `frame`; `parts`: the type symbol's frame). A stair run
  drawn from its sketch lists `stair_run_lines`, `stair_run_ends`,
  `stair_run_type` and `stair_run_base_cuts`, and a landing slab
  `landing_lines` and `native_landing`. A source made of several records lists
  them in `parts`, at most 64, with `parts_left_out` counting the rest; the
  table's size is stated in `docs/export-diagnostics.md`. The table's
  `value_source_counts` also counts, by reader, the elements whose copy was
  chosen as the current partition's, so a run on any model shows whether the
  rule is exercised. The readers still without a source, and the elements that
  keep `selected_frame_box`, are listed in `docs/export-diagnostics.md`:
  area boundary lines, hollow and I sections (the type's section parameters),
  pipe axes, roof slopes and extrusions, and family extrusions. Nothing is
  noted unless the table is asked for, so the default export and its
  diagnostics are unchanged.
