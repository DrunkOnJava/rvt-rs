- **A wall whose edited elevation profile has more than 512 regions to cut keeps
  them as its openings and has none cut in its body (#1845).** Each region cut
  in a wall's body is one level of a chain of boolean results, and a chain's
  depth is the depth of every recursion over it. On a thread of 1 MiB the derived
  `Debug` of a `SolidShape` breaks at 3,072 levels and `Clone` at 4,096, and
  `Drop` and the serialiser at 65,536 (`probe_1845_boolean_chain_depth`); a
  profile of 4,096 curves can make 4,096 regions, one circle each. Nothing in the
  export clones or prints a model, so the hazard was a library consumer's: a
  crafted file whose model it cloned or printed. The openings Revit's export
  writes for the regions are written either way, as for a wall whose regions
  are not all solids; only the body is not cut. No wall of the corpus has that
  many regions: 146 of 146 exports and 121 of 121 diagnostics files of an
  unfiltered Measure run are byte for byte as before.
