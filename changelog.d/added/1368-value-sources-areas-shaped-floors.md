- **Value sources for Areas and for floors drawn along their shape-edited
  top (#1368).** Under `--element-provenance` an Area drawn from its area
  boundary lines lists the frame of each line that bounds its region
  (`area_boundary_lines`: the first placed copy by frame order, every placed
  copy of a line giving the same line) and the element data that gave its
  point, which picks the region among the lines (`area_point`: the Area's
  data object on Revit 2024, its element-data header on 2015 to 2023). A
  floor drawn along the plane of its shape-edited corners (#1384), which had
  no value source, lists its frame, the sketch lines of its outline and the
  current native record whose `SlabShapeEditGStep` gives that plane
  (`native_slab_shape`). Every `BodySource` now has a reader that notes its
  copy; an Area's Levels, which give its base and height, have no source.
  `tests/value_sources_offsets.rs` re-reads each line's frame, the native
  record's ElementId, and each Area's point, which must lie inside the
  outline the export drew. On the BIM4LCA office as built all 30 Areas and
  all 8 such floors have their sources, and the compact diagnostics with the table grow 0.9%
  (6,236,391 to 6,291,246 bytes with the diagnostics; CI's Corpus health on
  `main` at 77d71cbb, run 41944703690, against this change at 779ee16c). The
  Revit 2015 to 2023 path of `area_point` and the lines is not exercised by
  a model in CI. Nothing is noted unless the table is asked for, so the
  default export and its diagnostics are unchanged.
