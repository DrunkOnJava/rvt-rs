- **Value sources for family instances drawn from their families' own
  extrusions (#1368).** With `--element-provenance` and `--family-extrusions`
  both on, an instance whose body is rebuilt from its family's extrusions lists
  the records of the family's embedded document that it was rebuilt from
  (`family_forms`, `parts`: each visible extrusion, its sketch plane and the
  curves of its sketch, at the place the document's group puts them in their
  partition), beside the `instance_transform` that places them and the
  `selected_frame` they are drawn at. The family document is read once, as it is
  embedded, and the places are kept only while the table is asked for: with
  either flag off nothing is built or retained, and the IFC and the default
  diagnostics are unchanged. Area boundary lines are the one body still without
  a source.
