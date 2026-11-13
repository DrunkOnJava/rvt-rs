- **A room bound by its parameter block says so, every refused wall end is
  counted, and a material layer's name is its `Name` (#1947).** Found by the
  batch 9 export audit.
  - A room bound to the Level its own parameter block names (RE-29), where its
    record names none, carried `LevelBindSource`
    `partition_element_record_reference_list`; it now carries
    `partition_room_parameter_block`.
  - A wall with one end refused with face evidence and the other without
    counted only the first: `exported.wall_end_reads_unmarked` now counts the
    second, as `docs/export-diagnostics.md` says every refusal is counted.
  - `IFCMATERIALLAYER` wrote a layer's name in its sixth attribute
    (`Category`). IFC4 puts `Name` fourth, and the name now goes there. No
    layer is named today, so no export changes from it.
