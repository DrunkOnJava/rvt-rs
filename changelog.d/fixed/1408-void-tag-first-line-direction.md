- **A floor's void is tagged by the direction its first line is stored in, as
  Revit tags it, on 133 of 133 voids (RE-355, #1408).** Of the sketch lines of a
  void's loop, the one with the lowest ElementId runs along the loop's
  counter-clockwise walk or against it; against it that line is the tag, along it
  the line after it. That is Revit's tag on the BIM4LCA office as built (36 of
  36) and at building permit (35 of 35) and on Core Interior (62 of 62),
  including the void 3147725 that RE-339 left wrong, and on 12 of 12 voids of
  Snowdon Towers architectural (11 of 12 before, scored locally against Revit's
  export). Where the direction is not read (a curve in the loop, two lines on one
  edge, no stored ends) the tag is the one RE-339's rule gave. A reading from the
  order the first two lines were drawn in is wrong on 20 of Core Interior's 62 and
  is not used.
  - This moves tags in users' exports, and a moved tag moves GlobalIds: the
    opening's id is keyed on the tag, and the ids of its `IfcRelVoidsElement`,
    its `Pset_OpeningElementCommon` and its defining relation follow it, so 4 ids
    move on each of the 6 voids that moved. The id is not redesigned.
  - Measured on Revit 2024 only. Measure runs 42122776008 (6d3f6c73 against
    `main`) and 42131336084 (e77ef0dd, every scorer, 111 exports): 3147725 moves
    on both office files, 1 void on Snowdon architectural (now Revit's) and 3 on
    Snowdon structural, with no export to check them; the other 107 exports, the
    24 Revit 2025 models, the samples of 2025 to 2027 and the 19 held-out and
    spent models among them, are byte for byte `main`'s. No Revit 2025 export
    holds a floor void to check. Of the 19 held-out and spent models the probe
    read 11, each with no loop to read, and could not read 8 (Revit 2018 to
    2022): the rule is exercised on no held-out model.
  - The exporter reads a subset of what the probe reads and keeps RE-339's tag
    where it declines; why Revit does this is unread, and the rule for a first
    line stored along the walk rests on 9 loops of the office (RE-355 sections 3
    and 4).
