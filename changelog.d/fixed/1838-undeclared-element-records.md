- **A file saved in place after elements were deleted no longer has its IFC
  called incomplete without saying why (#1838).** The element records the
  reader leaves out because their ElementId is not one the file's element table
  declares were reported as a layout this release cannot locate, with the IFC
  incomplete. A Revit 2027 structural model, compared with Revit's export of
  the same save, showed 84 of them were roof framing deleted between two saves:
  the record chain was the same in both, the table declared 143 fewer of its
  records, and Revit's export held none of the 84 and nothing the reader lacked
  (2,355 of 2,355 elements by GlobalId). The command line's and the
  diagnostics' warnings, `rvt-inspect`'s failure mode, readiness and next step,
  the viewer's card and the docs now say what the records are: a file saved in
  place keeps those of elements deleted since its last Save As, and the model is
  incomplete only if nothing was deleted. The count, the readiness score and
  the `element_record_without_element_id` item are unchanged. Not measured: no
  model of Measure has such a record (none of its 121 published diagnostics
  carries the item), so the one comparison is the reporter's, as counts.
