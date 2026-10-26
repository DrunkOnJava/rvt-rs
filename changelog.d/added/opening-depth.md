- **Openings' `Depth` (#1016, RE-244).** The opening a door or window cuts
  in a wall carries `Qto_OpeningElementBaseQuantities.Depth`, the wall's
  thickness, where the symbol that draws it cuts an outline, as Revit's
  export writes it: 184 of the BIM4LCA office's openings as built and 159 at
  permit, each Revit's value. Their `Width` and `Height`, which Revit's
  export swaps on every door opening (its `Width` is the opening's height),
  are left out.
