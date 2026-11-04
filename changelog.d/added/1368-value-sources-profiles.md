- **Value sources for profiles, stair runs and landings (#1368).** Under
  `--element-provenance` an element whose body is its record's box but whose
  profile was solved from sketch lines (floors, slabs, ceilings, and roofs
  whose body is the record's box) or taken from its type symbol's box
  (columns) now lists where the profile came from, as well as
  `selected_frame` for the box: `sketch_lines` (`parts`: the frame of each
  sketch line the solve was given, the first of its frames by frame order, or
  the current partition's where they disagree) and `sketch_list` (the
  Sketch's curve list, with the rule that chose its copy), or `type_section`
  (`parts`: the type symbol's frame; rule `agreeing`, or `frame` where the
  symbol's frames give different boxes and the last is taken). A stair run
  drawn from its sketch lists `stair_run_lines`, `stair_run_ends`,
  `stair_run_type` and `stair_run_base_cuts` (a Revit 2025 run, `native_run`
  and `native_run_type`),
  and a landing slab `landing_lines`, `native_landing` and
  `native_landing_type` (the landing type's record gives the thickness). A
  source made of several records lists them in `parts`, at most 64, with
  `parts_left_out` counting the rest. The sources add 2.4% to the BIM4LCA
  office's table (5,987,841 to 6,131,584 bytes of compact JSON) and 63% to
  Core Interior's (440,909 to 718,828; Measure run 41882133812). The table's
  `value_source_counts` also counts,
  by reader, the elements whose copy was chosen as the current partition's,
  so a run on any model shows whether the rule is exercised. The readers
  still without a source, and the elements that keep `selected_frame_box`
  (every room, whose outline is read from its solid's faces), are listed in
  `docs/export-diagnostics.md`: area boundary lines, hollow and I sections
  (the type's section parameters), pipe axes, roof slopes and extrusions, and
  family extrusions. Nothing is noted unless the table is asked for, so the
  default export and its diagnostics are unchanged.
