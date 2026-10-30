- **Elements that do not exist in the export phase are left out (#328,
  RE-184).** A project's `AllProjectPhases` record lists its phases in
  order, and with no phase chosen Revit's export shows the last: an element
  is written when it was created in that phase or before it and not
  demolished in it or before it, as Revit 2025's exports of two models made
  for this show. On Snowdon Towers, whose phases run Legends, Existing, New
  Construction, the six slabs created and demolished in Legends are no
  longer written, and nothing else changes. The diagnostics count what is
  left out as `element_record_not_in_export_phase`. Rooms are not checked:
  Revit's exporter skips a room by its own phase parameter, which no model
  here shows to be its created phase, so a room created in an earlier phase
  is still written. A read of the phases that fails on a Revit 2023 to 2027
  file is a warning in the diagnostics, and the export then writes every
  element.
