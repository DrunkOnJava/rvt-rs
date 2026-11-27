- **Two rules for faces whose stored edges part by a hair are on in every export
  (#2085, RE-498).** Where two cylinder or revolved rings each hold a stored gap
  under 0.000375 ft at the same corner, in the pattern a contributor's chair save
  shows, each ring keeps the point both name and the loop closes there (the
  coordinated join). Where a curved face's end sample leaves the flat face it
  shares an edge with by at most 0.00039 ft, the flat face's own saved sample is
  taken (the plane anchor); before, the flat face was refused as `shared curved
  edge leaves adjacent plane`, and with it the body. Both were fitted to that one
  family on one save, which Measure does not hold, with a 4,000,000-value graph
  budget: at the default limit of 1,000,000 the chairs' saved graphics are
  refused before either rule is reached (`rvt-ifc --saved-mesh-graph-values`
  raises it). Griffin chose to turn both on by default. On the models Measure
  reads, the plane anchor draws one body more, a cooktop on each of Autodesk's
  `rac_basic` saves of 2023 to 2027, whose saved mesh spans its record box to
  the millimetre and which no Revit export checks; the coordinated join takes no
  face (Measure 46058172915, 46060234028). No other export changes.
