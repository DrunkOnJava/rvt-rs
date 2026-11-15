- **A curtain mullion turned about its own axis takes its length from its saved
  mesh along that axis (#1193).** Where its record box could not give its
  extents along its own axes, the mesh pass had no axis for it, so 12 mullions of
  the BIM4LCA office as built were written with no `Pset_MemberCommon.Span`. The
  pass now reads them along the mullion's own Z, the axis its transform records.
