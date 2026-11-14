- **A wall of a curtain wall type is an `IfcCurtainWall` on Revit 2024 to 2027
  although no mullion or grid names it, and a curtain wall that is the host
  panel of another is not one on any release (#1960).** The 2024 reader made a
  wall a curtain wall only where a mullion named it or a grid was found, so a
  single-cell curtain wall with no mullion (walls 573660 and 573664 of
  Autodesk's `rme_basic` and `rme_advanced`) came out as an `IfcWall`, where
  the 2023 reader, which goes by the wall's type, writes an `IfcCurtainWall`
  with its panel. Revit's own exporter (revit-ifc, `CurtainSystemExporter`)
  makes a wall a curtain wall by the curtain grid it has, with no test of
  mullions or cells, so the 2024 reader goes by the type too. A curtain-type
  wall that is the host panel of another curtain wall's grid (its record holds
  a `HostPanelInfo`; Snowdon Towers has 23) is written by Revit only as parts
  of its owner, and neither reader makes it a curtain wall. No Revit export in
  the corpora holds a mullion-free curtain wall: that case rests on the
  exporter's source (RE-462).
