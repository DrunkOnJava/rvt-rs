- **A wall with an edited elevation profile carries the openings Revit's export
  writes for it, the pockets of the profile against its convex hull, and its
  profile's cut is made in its body (#1607, RE-380).** RE-151 read the profile as
  the box less one opening for each run of its lines that leaves its bounding
  rectangle. That matched Revit's export on Core Interior's wall 55840 and wrote
  an opening Revit does not write on ten more walls: RE-358's eight walls of the
  BIM4LCA office, in both office files, and both of its terrain files' walls, one
  of which, 1387731, has an opening in Revit's export that is the first three
  lines of its top closed by a chord. Revit writes the body as the profile and an
  opening only where the profile has a pocket against its convex hull, tagged with
  the highest ElementId of the pocket's lines, none on a straight sloped top. The
  wall's body is now its box less the profile's regions as one boolean difference
  (`ProfileCutSource` `wall_profile_cut_in_body`, counted in
  `exported.wall_profile_cuts`), the shape it had, and its openings are the pockets.
  A wall the cut cannot be made for keeps its regions as its openings, as before.
