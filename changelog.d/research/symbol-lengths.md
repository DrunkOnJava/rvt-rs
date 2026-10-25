- **Stored symbol lengths against Revit's door and window sizes (RE-229,
  #810).** A probe and scorer, `examples/probe_re229_symbol_lengths.rs` and
  `tools/re/symbol_lengths_vs_ifc.py`, compare every length a door's or
  window's symbol stores with the sizes Revit's export writes. The width is a
  family's own parameter on the BIM4LCA office and a built-in one on Core
  Interior and RE1, and no stored length is the width of the office's two
  doors #810 still leaves unsized.
