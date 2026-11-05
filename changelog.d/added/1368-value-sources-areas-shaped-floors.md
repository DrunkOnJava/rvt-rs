- **Value sources for Areas and for floors drawn along their shape-edited
  top (#1368).** Under `--element-provenance` an Area drawn from its area
  boundary lines lists the frame of each line that bounds its region
  (`area_boundary_lines`: the first copy by frame order, every placed copy
  of a line giving the same line) and the element data that gave its point,
  which picks the region among the lines (`area_point`: the Area's data
  object on Revit 2024, its element-data header on 2015 to 2023). A floor
  drawn along the plane of its shape-edited corners (#1384) lists the
  current native record whose `SlabShapeEditGStep` gives that plane
  (`native_slab_shape`), beside the frame and the sketch lines it already
  listed. Every `BodySource` now has a reader that notes its copy; an
  Area's Levels, which give its base and height, have no source.
  `tests/value_sources_offsets.rs` re-reads each line's frame, the data
  object or header and its point, and the native record's ElementId on the
  models it covers. Nothing is noted unless the table is asked for, so the
  default export and its diagnostics are unchanged.
