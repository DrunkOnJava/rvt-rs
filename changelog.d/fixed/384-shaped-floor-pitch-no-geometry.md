- **`rvt-ifc --mode typed-no-geometry` keeps the `PitchAngle` of a floor drawn
  along its shape-edited plane (RE-384).** The geometry modes write it
  (`atan` of the plane's gradient), and a level slab keeps its 0 in the mode
  without geometry because the rule is read before the mode strips the body;
  the sloped floor's was read after, so the mode left it out. Revit's export
  writes the angle whatever the body.
