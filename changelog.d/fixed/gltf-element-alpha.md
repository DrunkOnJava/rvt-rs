- **glTF draws an element made only of transparent materials see-through
  (#2109).** An element drawn in its category's colour, as one body, whose
  materials were all read as transparent now takes their alpha, the least
  transparent one's, in a material named `Category <class> transparent` whose
  `alphaSource` is `"file"`. One that also holds an opaque material, or a
  material whose transparency was not read, keeps its category's alpha: drawn
  at its glass's, a door with a glass panel would look all glass, and which part
  of a saved mesh is the glass is not read. Windows and curtain panels keep
  their translucent category colour.
