- **A floor drawn along its shape-edited top keeps its slab quantities, and a
  one-layer wall cut at it keeps its layer's colour in the GLB (#1384).**
  Drawing the BIM4LCA office's 8 shape-edited floors along their sloped top
  left out their `Qto_SlabBaseQuantities` `GrossArea` and `NetVolume`, which
  are the plan profile's area and that area times the type's thickness and
  were Revit's on all 8: 16 values equal before became missing on each office
  file (Measure run 41944962415, main at 77d71cbb against v0.14.0). They are
  written again from the plan profile the floor's extrusion still holds. The
  14 one-layer Basement walls as built (16 at permit) whose base is cut at
  a shape-edited or level floor were drawn in the GLB in the category colour instead of their
  layer's, since their solid is not one the layer splitters take apart; a
  one-layer element's solid is now drawn as that layer where its thickness
  across the wall is the layer's within 0.02 ft.
