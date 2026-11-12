- **Each GLB material says whether its colour, and its alpha, are the Revit
  file's or ones rvt-rs chose (#1812).** Every material carries
  `extras.colourSource` and `extras.alphaSource`, each `"file"` where the
  value is the Revit material's own and `"rvt-rs"` where rvt-rs chose it.
  The second covers the category colours an element with no material of its
  own is drawn in, translucent windows and faint rooms included, the grey of
  a material whose colour was not read, and the opaque alpha of one whose
  transparency was not read. A consumer that stores colour or opacity as
  model data could not tell them apart before. Nothing else in the GLB
  changes, and the IFC is unchanged.

  A view's visibility and graphics overrides are not read, and the support
  matrix now says so. Revit's own IFC export ignores them too: it takes colour
  and transparency from the element's material, or else its category's.

  Breaking for code that builds a `gltf::Material` with a struct literal: it
  has a new field, `extras`.
