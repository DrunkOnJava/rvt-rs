- **A floor's void is tagged by the direction its first line is stored in, as
  Revit tags it, on 133 of 133 voids (RE-355, #1408).** Of the sketch lines of a
  void's loop, the one with the lowest ElementId runs along the loop's
  counter-clockwise walk or against it; against it that line is the tag, along it
  the line after it. That is Revit's tag on the BIM4LCA office as built (36 of
  36) and at building permit (35 of 35) and on Core Interior (62 of 62),
  including the void 3147725 that RE-339 left wrong, and on 12 of 12 voids of
  Snowdon Towers architectural (11 of 12 before). Where the direction is not read
  (a curve in the loop, two lines on one edge, no stored ends) the tag is the one
  RE-339's rule gave. A reading from the order the first two lines were drawn in
  is wrong on 20 of Core Interior's 62 and is not used.
  - Measured (Measure run 42122776008, 6d3f6c73 against `main`, 54 exports):
    3147725 moves on both office files, 1 void on Snowdon architectural (now
    Revit's) and 3 on Snowdon structural, with no export to check them; the
    other 50 exports, the 19 held-out and spent models among them, are byte for
    byte `main`'s.
  - Why Revit does this is unread, and the rule for a first line stored along
    the walk rests on 9 loops of the office. `probe_re355_void_loop_directions`
    prints each loop and the counts by branch (RE-355 section 3).
