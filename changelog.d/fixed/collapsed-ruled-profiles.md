- **Saved ruled surfaces with a point profile (RE-443).** A zero profile
  pointer now uses its separately saved point, for measured point/arc
  patches with a unit envelope and rectangular trim. Bodies containing
  these patches qualify only by the source record's duct-fitting category,
  independently of their exported IFC class. On Autodesk's public Revit
  2023 basic MEP sample, 105 duct fittings gain bodies instead of boxes;
  two sanitary terminals remain boxes with an explicit refusal and are
  negative controls. Default native-IFC comparison of the fittings is within
  0.697 mm and 0.179% volume; none passes the stricter 0.001 ft shape threshold
  by default, while all 105 do with fine reference tessellation. Unmeasured
  categories, combinations and trims remain refused. The report includes
  pinned public bytes, category-override controls and local native-reference
  figures. The native reference is not shared: redistribution terms are not
  established. One cell along the straight point/arc generators retains the
  reviewed arc sampling: the 105 admitted bodies hold 64,380 triangles
  (613 on average, 636 maximum). The 2024 MEP IFC grows from 103.6 to 107.5 MB.
  Snowdon Towers HVAC retains 32 recovered duct fittings with 5,856 triangles
  (183 on average): 65.5 to 65.9 MB. RE-443 separates the current local checks
  from the maintainer's Measure of the earlier 107-body scope.
