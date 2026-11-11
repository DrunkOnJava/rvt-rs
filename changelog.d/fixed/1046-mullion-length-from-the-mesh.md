- **A curtain mullion's `Length` and `Span` are its saved mesh's extent along its axis where the record box overruns the solid (#1046, RE-418).**
  49 mullions of the BIM4LCA timber office at permit were written 0.01 m to
  0.279 m (0.49% to 14.3%) longer than Revit's export has them, because the
  record box they took their length from overruns the solid. A mullion whose
  box volume is not its mesh's (RE-403) now takes the extent of its mesh along
  the axis its record runs it along, where the mesh is a prism along it and the
  extent lies within 35% of the box's `Length`, and its `Span` the same;
  otherwise the box's length stays. 49 more of 7,872 read equal to Revit's, and no other value on
  that file or any model with a Revit export changes.
