- **The openings a rectangular wall opening cuts in a wall are written.** A
  wall's native record names each element that cuts it, in the order Revit
  numbers the wall's openings, and Revit's export writes an opening with
  nothing to fill it for each rectangular wall opening among them. rvt-rs now
  writes it voiding the wall Revit's export voids, as the rectangle the
  opening's record saves, through the wall, tagged with the opening's
  ElementId and named for the wall, with the wall's type name as
  `Pset_OpeningElementCommon.Reference` and
  `Pset_EnvironmentalImpactIndicators.Reference`, the rectangle's width and
  height and the extrusion's depth as `Qto_OpeningElementBaseQuantities`, and
  Revit's own GlobalId: 30 of 30 on the BIM4LCA office as built and 24 of 24
  at building permit, each within 0.003 ft of Revit's box with every one of
  those values equal. The office at permit holds one more such opening than
  its Revit export (the records are the same in both files and Revit writes
  it only as built), which the census counts apart. Fire hose cabinets (39
  of 39 exact, one more than Revit's export in the office as built and two
  at permit) and curtain walls (box exact on 52 of 68) are measured and not
  written, and a door's further cut in a second wall, a door's cut in itself
  and a wall's own profile are not read yet (RE-321, #1165).
