- **A saved mesh's shell lying inside another is left out (#1103).** On the
  BIM4LCA office four doors drawn from their saved mesh carried a second
  shell lying wholly inside the first, a part flush with the leaf, so their
  bodies enclosed a quarter more volume than Revit's with the same
  surfaces. Revit's export writes the two as one solid; rvt-rs now leaves
  the inner shell out, and the larger shell is Revit's body.
