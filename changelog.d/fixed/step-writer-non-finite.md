- **An IFC export holding a number that is not finite is refused instead of
  written.** A NaN or an infinity was written as `NaN` or `inf`, which
  ISO 10303-21's REAL cannot express and IfcOpenShell refuses. `rvt-ifc`
  (both of its write paths), the Python `write_ifc` and the viewer's IFC
  download now stop with an error that names the entity, and
  `rvt::ifc::step_writer::write_step_to` returns an `InvalidData` error. The
  text-returning `write_step` functions still write what they are given, and
  say so. No Revit file is known to reach the writer with one: this is a check
  in depth, and it changes no export of a finite model.
