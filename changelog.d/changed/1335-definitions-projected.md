- **The parameter definitions pass no longer builds the values of the
  `Family` and `FamilySymbol` records it reads two fields of (#1335).** The
  pass decoded each of those records in full and kept the record's `m_id` and
  its `m_categoryId` or `m_familyId`, which was most of the pass on a Revit 2015
  to 2023 project. It now reads them as before, with the same checks and the
  same refusals, and builds only the fields it keeps
  (`native_parameters::decode_graph_projected`, `Options::graph_fields`). No
  export changes. The figures are in the pull request.
