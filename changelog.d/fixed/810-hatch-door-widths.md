- **A door or window whose symbol cuts no loop takes its `OverallWidth`
  from the lines of no category in that symbol's saved graphics, and the
  BIM4LCA office's two wall hatches now have Revit's (RE-318, #810).**
  Doors 1756089 and 1765267 are `TRAPDOOR` hatches (family `LU-3.VSS`, types
  `_90°` and `_180°`), not doors with a sidelight. Their record box was
  0.1137 ft wider than Revit's 5.8629 ft, and neither their cut loop (none),
  cut solid, stored lengths nor visible surfaces give it: the 90° type hides
  the 180° leaf its width includes. A symbol's saved graphics hold the edges
  of the boxes that bound its forms as lines of category -1, hidden forms
  among them, and their extent along X is Revit's width on all 268 of the
  office's doors Revit sizes and all 269 windows as built (268 and 265 at
  building permit), where the record box missed 2 doors and the surfaces 2.
  The rule fires only where the record box sized the door or window before,
  and a symbol with no such line, a degenerate extent, an instance or
  component reference in its graphics, or two readings that disagree, keeps
  the width it had. Against `main` (Measure 41725344128) the office's
  `IfcDoor` widths equal Revit's on 268 of 269, from 266, and three widths
  move on the as-built file (1756089, 1765267 and, by 7 micrometres, 1722215);
  the only other model with such a door, RE1 Architecture's curtain-wall
  door 445975, keeps its width, and 31 of 34 measured models and all 18
  held-out models write the same IFC. One building's families: what the
  boxes are, and why their extent is Revit's width, is inferred from Revit's
  widths, not documented (`reports/element-framing/RE-318-symbol-wire-width.md`).
