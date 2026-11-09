- **A roof drawn as the level block of its sketch outline is drawn from its saved
  mesh on Revit 2023 to 2027 too, where the mesh decodes whole (#1612, RE-389).**
  The block is the outline extruded over the record box's full height, which holds
  the roof's reference surface and not its slope or thickness; the mesh is what
  Revit drew. Against Revit's own exports, Snowdon Towers Architectural's 10 roofs
  that move (local only) are within 0.4% of Revit's volume and 0.01 ft of its box,
  where 7 of them were 40% to 125% too large as blocks, and the roof of the T and
  U footprints of puzzbobb's Revit 2025 models goes from 37,357 ft3 to Revit's
  1,845.6. The same project's saves now agree: rac_advanced's three 15 degree
  gables are meshes on its 2025 save as on 2026 and 2027. A roof whose mesh does
  not decode whole keeps its block, and a resolved outline stays the body of every
  other class. The outline's own properties (`ProfileSource`, `ProfileVertexCount`,
  `ProfileVoidCount`) and the `SlabShapeRefusal` of a body replaced by its mesh are
  left out, and `ProfileResolved` reads false. **Not measured:** the Autodesk
  samples' roofs have no Revit export, and on 3 held-out roofs two scorers
  disagree (RE-389 section 4). `--no-saved-meshes` keeps the blocks.
