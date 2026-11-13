- **An export whose own geometry stops at a work budget says so (#1845).**
  The bounds #1845 put on wall tops, roof and slab envelopes, mullion grid
  cells and space containment left the shape out or the element uncontained
  without a word. Each now notes its stop, as the readers' budgets do
  (#1833): the export's diagnostics list it under `work_budget_reached`, and
  the export is written with a warning naming it. No file of the measured
  corpus reaches one.
