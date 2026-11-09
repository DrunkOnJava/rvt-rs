- **A wall with an edited elevation profile carries the openings Revit's export
  writes for it, the pockets of the profile against its convex hull, and the
  rest of its profile's cut is made in its body (#1607, RE-380).** RE-151 read
  the profile as the box less one opening for each run of its lines that leaves
  its bounding rectangle. That matched Revit's export on Core Interior's wall
  55840 and wrote an opening Revit does not write on ten more walls: RE-358's
  eight walls of the BIM4LCA office, in both office files, and both of its
  terrain files' walls, one of which, 1387731, has an opening in Revit's export
  that is the first three lines of its top closed by a chord. Revit writes an
  opening only where the profile has a pocket against its convex hull, none on a
  straight sloped top, and tags it with the first line of the pocket's stretch
  (12 of the 15 pockets measured; the last line on the 3 of one Snowdon Towers
  wall, which nothing read explains). A pocket is closed by the hull's edge, or
  through the profile rectangle's corner where the edge cuts it off, as on Core
  Interior's wall 55840 (301.65 ft2); a pocket whose edge ends otherwise is not
  written. A region of RE-151 that a pocket equals is no longer an opening of
  its own: the pocket is written in its place and the wall keeps its body. Any
  other region is taken out of the wall's body as one boolean difference
  (`ProfileCutSource` `wall_profile_cut_in_body`, counted in
  `exported.wall_profile_cuts`), so the shape is the one it had and the openings
  are Revit's. A wall the cut cannot be made for keeps its regions as its
  openings, as before.
