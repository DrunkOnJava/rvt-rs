- **A floor drawn along the plane its shape-edited corners lie in carries that
  plane's slope as `Pset_SlabCommon.PitchAngle`, as Revit's export writes it
  (RE-384).** The angle of the floor's top plane from the horizontal, the
  arctangent of the length of the gradient of `z = a·x + b·y + c`, is what
  revit-ifc's slope calculator gives a slab with no location curve. Revit's
  export writes it on the 8 sloped floors of the BIM4LCA office (1.9 to 24
  degrees), which rvt-rs has drawn along their plane since #1384 and left
  without the value; the angle of the largest upward face of rvt-rs's own body
  for each is Revit's to 2e-7 rad. A floor whose plane is not drawn gets none.
