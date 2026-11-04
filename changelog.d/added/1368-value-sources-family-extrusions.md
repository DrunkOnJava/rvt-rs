- **Value sources for family instances drawn from their families' own
  extrusions (#1368).** With `--element-provenance` and `--family-extrusions`
  both on, an instance whose body is rebuilt from its family's extrusions lists
  the records of the family's embedded document that it was rebuilt from
  (`family_forms`, `parts`: each visible extrusion, its sketch plane and the
  curves of its sketch, at the place the document's group puts them in their
  partition), beside the `instance_transform` that places them and the
  `selected_frame` they are drawn at. A record the file holds in several
  partitions is named at its copy in the highest partition, the copy the
  rebuild reads; an instance one of whose records has no place (its group's
  bytes are not the partition's own) gets no `family_forms` source, and keeps
  its other sources. The family document is read once, as it is embedded.
  Where the table is not asked for, no place and no record list is collected
  or kept (the document is read as before), and the IFC and the default
  diagnostics are unchanged. Area boundary lines are the one body still
  without a source.
