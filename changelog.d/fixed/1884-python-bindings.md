- **The Python bindings warn when an element's saved mesh decodes only in
  part, release the GIL for the Rust work, and say what they return and why
  they fail (#1884).** A saved mesh that decodes in part leaves its element
  with the record's box, counted in the diagnostics; the docs promised a
  `RuntimeWarning` for it and none was raised, and it is now (the note
  `rvt-ifc` prints for the step, when an element keeps its box). `write_ifc`,
  `rvt_to_ifc`, `rvt_to_ifc_diagnostics`, `schedule_csv`,
  `export_diagnostics_json`, `export_diagnostics`, `decoded_elements`,
  `element_counts` and opening a file held the GIL for the whole decode and
  now release it, so another Python thread runs meanwhile. **A handle is used
  by one thread at a time:** a second call on one `RevitFile` from another
  thread while one runs now raises `RuntimeError`, where it used to wait for
  the GIL and then run, so a program that shares a handle between threads opens
  one for each thread or holds a lock. The stub's `TypedField` lists the eleven kinds the
  binding returns and the `float` values of one of them, where it listed four
  kinds. A failed load of the extension (a missing symbol, a wrong ABI) is the
  `ImportError` it is, where `rvt/__init__.py` caught it and it surfaced as
  `module 'rvt' has no attribute 'RevitFile'`. The name of an unread stream is
  quoted in the sentence that reports it, in `rvt-ifc`, `rvt-gltf`,
  `rvt-rs-mcp`, the diagnostics and Python alike, so a stream named with a
  newline cannot forge a line in a log. No export moves.
