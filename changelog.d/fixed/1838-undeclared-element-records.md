- **Records of ElementIds the element table does not declare are no longer
  all reported as an incomplete IFC from a layout this release cannot locate
  (#1838).** The element records the reader leaves out because their ElementId
  is not one the file's element table declares were reported that way. A Revit
  2027 structural model, compared with Revit's export of the same save, showed
  84 of them were roof framing deleted between two saves: the record chain
  was the same in both, the table declared 143 fewer of its records, and
  Revit's export held none of the 84 and nothing the reader lacked (2,355 of
  2,355 elements by GlobalId). The command line's and the diagnostics'
  warnings, `rvt-inspect`'s failure mode, readiness and next step, the
  viewer's card and the docs now say what was measured: on that one model such
  records were elements deleted since the file's last Save As, which a file
  saved in place keeps, and if nothing was deleted the model is likely
  incomplete. Other causes are known (an element table read short, as on
  Autodesk's Snowdon Towers 2024 sample before RE-41) or not ruled out, so the
  count, the readiness score and the `element_record_without_element_id` item
  are unchanged. Not measured: no model of Measure has such a record (none of
  its 121 published diagnostics carries the item), so the one comparison is
  the reporter's, as counts.
