- **Saved ruled surfaces with a point profile (RE-443).** A zero profile
  pointer now uses its separately saved point, for measured point/arc
  patches with a unit envelope and rectangular trim. On Autodesk's public
  Revit 2023 basic MEP sample, 105 duct fittings and two sanitary terminals
  gain bodies instead of boxes. Default native-IFC comparison is within
  0.721 mm and 0.179% volume; it does not pass the stricter 0.001 ft shape
  threshold. Unmeasured combinations and trims remain refused. The report
  includes the pinned public bytes and local native-reference figures. The
  native reference is not shared: redistribution terms are not established.
  One cell along the straight point/arc generators reduces the 107 new
  bodies from 1,633,146 triangles to 66,098 (618 on average, 859 maximum),
  retaining the reviewed arc sampling and native-reference bounds. The
  2024 MEP IFC grows from 103.6 to 107.5 MB, rather than the original grid's
  159.2 MB. Snowdon Towers HVAC gains 32 bodies with
  5,856 triangles (183 on average): 65.5 to 65.9 MB, rather than 69.4 MB. RE-443 separates these local revision checks from the
  maintainer's Measure of the old grid.
