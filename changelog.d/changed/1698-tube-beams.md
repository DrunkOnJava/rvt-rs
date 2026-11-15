- **A beam whose type is a rectangular tube is drawn as that tube, with its six
  base quantities (#1698).** On Revit 2024, a structural framing type that holds
  the captions `bf`, `d`, `tf`, `tw`, `ri` and `ra` as a hollow rectangle
  (outside 200 by 300 mm, a 4 mm wall, corners rounded 8 mm outside and 4 mm
  inside) is drawn as an extrusion over `IfcArbitraryProfileDefWithVoids`, its
  corners as arcs, where it was a solid box. `CrossSectionArea`,
  `OuterSurfaceArea`, `GrossSurfaceArea`, `NetSurfaceArea`, `GrossVolume` and
  `NetVolume` follow from that profile and the beam's length. The BIM4LCA
  office's 8 tube beams are the one model that holds such a type. Not yet
  measured on this head.
