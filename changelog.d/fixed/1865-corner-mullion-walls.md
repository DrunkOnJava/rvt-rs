- **A curtain part that names two curtain walls and no grid takes the wall the
  element data of its list names, so 74 of the 79 corner mullions of Autodesk's
  `rac_advanced` on Revit 2027 are aggregated again and all 79 carry
  `IsExternal` (#1865, RE-449).** The 2025 and 2026 saves name one curtain
  wall in a corner mullion's reference list, and RE-46's rule aggregates it
  there; the 2027 save names both walls of the corner, which no arm of the
  match took, so the 79 stood alone with no whole and no `IsExternal`. Neither
  the order of the list nor the part's record box tells the two walls apart,
  but the element data of another id of the list (the part's curtain grid)
  names exactly one of them, at offsets that move between grid kinds and
  releases and are RE-72's on neither save, and that wall is the one that held
  the part on 2025 for 75 of 75 parts that have such an id on each save. The
  rule reads that data without fixed offsets, and it applies only to a part
  with no grid whose list names two or more curtain walls. A part for which no
  one wall comes out, the 5 mullions whose lists hold no such id, stands alone
  and takes the `IsExternal` its walls share where they agree (all 34 walls of
  the file are true). Measured on `rac_advanced`'s 2025 and 2026 saves against
  the 2027 one: the 34 curtain walls hold 4,458 parts, from 4,384, 29 of them
  as many as on 2025; `IsExternal` lost between the saves goes from 79 to
  none, and the 2026 to 2027 pair differs in 425 fields, from 573. The rule
  reaches every file with such a part, not the 2027 sample alone: 14 other
  exports of the 146 on Measure's unfiltered run change, each by `IsExternal`
  on curtain mullions the same way (9 to 79 in the 2015 to 2023 saves of
  `rac_advanced`, 2 to 4 in the four BIM4LCA offices), values that are not
  measured and not equal: Revit's export writes them as an exporter artefact,
  the census counts them apart (RE-269) and it does not move on any model.
  Two members of the BIM4LCA timber office are aggregated into the curtain
  walls Revit's own export names (`IfcMember` same whole 7,872 to 7,874). The `rac_advanced` saves of 2015 to 2022 now
  write the same 19,979 parameter values and those of 2024 to 2027 the same
  21,035; the 2023 save stays six short (#1899). Not changed: the curtain
  door 216844, which the 2026 and 2027 saves leave out of its wall for a
  cause not found, and the 5 mullions, whose wall nothing in the file names;
  no Revit export of any of these saves exists, so the 2027 save is held to
  the same project's earlier saves.
