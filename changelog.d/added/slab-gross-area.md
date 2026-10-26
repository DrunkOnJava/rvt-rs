- **Floors' `GrossArea` (#1016, RE-249).** A floor drawn as one extrusion
  carries `Qto_SlabBaseQuantities.GrossArea`, the area of the profile it
  extrudes, its outline less its holes, as Revit's export writes it: 31 of
  the BIM4LCA office's slabs as built and at permit, each Revit's value,
  none different. A floor of several loops, drawn as several elements, is
  left out.
