- **A sloped roof's volume in Revit's export is its saved mesh's times the cosine
  of its slope (#1656, RE-415).** The 0.2% by which the BIM4LCA housing's roof
  1627602 read over Revit's is not a face or a shell: its surface and Revit's coincide
  to 7e-5 ft, and Revit's export writes the roof type's layers (24 and 10 mm) along the
  vertical where the saved mesh has them perpendicular to the slope (34.0000 mm),
  so its body is thinner by the cosine of the slope, 3.6436 degrees. The same holds
  on the 7 sloped roofs of Snowdon Towers Architectural (0.99939, the cosine of 2
  degrees) and the 3 flat ones match to 1e-6. Nothing in the reader changes.
