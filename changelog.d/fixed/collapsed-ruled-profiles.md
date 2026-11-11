- **Saved ruled surfaces with a point profile (RE-443).** A zero profile
  pointer now uses its separately saved point, for measured point/arc
  patches with a unit envelope and rectangular trim. On Autodesk's public
  Revit 2023 basic MEP sample, 105 duct fittings and two sanitary terminals
  gain bodies instead of boxes. Default native-IFC comparison is within
  0.963 mm and 0.179% volume; it does not pass the stricter 0.001 ft shape
  threshold. Unmeasured combinations and trims remain refused. The report
  includes the pinned bytes, native reference and a replay without Revit.
