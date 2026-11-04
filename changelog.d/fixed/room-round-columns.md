- **A room's outline takes off the round columns its stored solid holds
  (#1354).** The walk that reads a room's outline from its stored solid
  (RE-101) read only planar wall faces, so a vertical cylinder, a round
  column, was ignored: the room kept the column's area, and a room with
  columns against its walls kept its record box. The walk and the resolved
  span (RE-316) now read the arcs of vertical cylinders that reach the
  floor, as chords within 0.0005 ft, and a room with more than 4,096 plan
  segments is left to its box. The public IFC2X3 export of Autodesk's
  `rac_advanced` shows why: Revit writes each round column as a void of the
  room's profile, and the 43 rooms that the walk read, 0.17% to 1.24% over
  Revit's area, are within 0.007% of it once the voids are subtracted
  (kept IFC of run 41832628804 against the export). Measure's `racadv`
  input scores the 2015 to 2020 saves of that project against the export,
  fetched for the run and never stored or cached.
