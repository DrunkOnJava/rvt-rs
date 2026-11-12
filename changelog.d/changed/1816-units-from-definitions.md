- **The export reads the project's display units from the parameter
  definitions pass it already makes, rather than walking the file again for
  its `UnitsElem` (#1816).** That pass decodes every `UnitsElem` the units
  read would, in the same order, so the units, and any error reading them,
  are the same. The read walks the file itself only where no such pass has
  been made. Exports and their diagnostics are unchanged.
