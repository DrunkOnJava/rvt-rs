- **A flat roof written as a slab carries its `Perimeter`, the length of the
  outline of the profile its body extrudes (#1208).** Revit's export writes a
  roof's `Perimeter` in the file's unit, where a floor's is the same length in
  feet, unscaled (RE-235 §5): on the BIM4LCA office as built it is the
  outline's length on all 10 roofs, exactly on the two read to the micrometre
  (`Basic Roof:KA-1` 1461758, 197,888.88 mm, and `Basic Roof:YP-1` 2991742,
  158,585.00 mm), and 10 of the 29 values the measurement finds missing are
  written. A floor's, a base slab's and a stair landing's `Perimeter` stay
  out, and a roof whose profile has holes gets none: whether the export counts
  a hole's outline in the value is unmeasured. A roof whose outline has a
  corner that is not a right angle gets none either: an arc of its sketch
  reaches the outline as chords, whose length is short of the arc's.
