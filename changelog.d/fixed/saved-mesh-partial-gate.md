- **`--saved-meshes` keeps the box where a saved mesh is partial.** An element
  some face or branch of whose saved graphics does not decode keeps its record
  box instead of a mesh that is only part of its body, and `rvt-ifc` counts
  them. On the BIM4LCA office 357 such meshes, none of them Revit's body, keep
  their box, and 1,294 of the 1,352 bodies drawn from a saved mesh are Revit's,
  against 1,294 of 1,709 before (RE-224).
