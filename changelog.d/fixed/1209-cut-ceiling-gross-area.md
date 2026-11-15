- **A ceiling with no `GrossArea` takes its saved mesh's face area where that
  face is one piece (#1209, RE-480).** Revit's export gives a ceiling that a
  step cuts its own computed area, which no saved parameter holds, so rvt-rs
  wrote none. The saved mesh holds the cut body: where its largest upper and
  lower horizontal faces are each one piece of the same area, that area is
  Revit's `GrossArea` on all 370 such ceilings of the BIM4LCA office and
  housing, cut or uncut. The office's 22 cut ceilings of one piece now carry
  it, equal to Revit's on all 22, with nothing else moving; 15 ceilings of
  Autodesk's sample projects gain one that no Revit export checks. A ceiling
  in pieces, a sloped one and one whose saved mesh does not decode whole keep
  none.
