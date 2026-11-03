- **`rvt-ifc` writes the IFC as it builds it.** The STEP text was built
  whole in memory and then written. It is now handed to the file every MiB,
  after the file handle and the partitions it keeps inflated are released.
  The bytes are the same. On the BIM4LCA office the text is 68 MB and the
  handle 362 MiB of inflated partitions; the saved-graphics pass, which runs
  before, still sets the peak there (#1075). With `--observation` the text
  is still built whole, since the witness reads it back.
  `rvt::ifc::step_writer::write_step_to` streams to any writer.
