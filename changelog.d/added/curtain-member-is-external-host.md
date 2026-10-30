- **Curtain mullions' and panels' `IsExternal` by their curtain wall's value
  (W7, RE-288, #1187).** Revit's `IsElementExternal` gives a family instance
  with no parameter and no type Function its host's value, which exporters
  from IFC_v24.2.20 on give a curtain wall's members; the exporter that made
  the BIM4LCA office's exports caches types instead (RE-269) and is counted
  apart. rvt-rs writes each member's curtain wall's value, and none on the
  member types: 10,262 values on the office as built, 10,299 at permit and
  12 on RE1 Architecture, whose 12 members are equal to its export's (0 of 12
  before). Only those three exports change. The exterior members' values are
  checked against revit-ifc's source only: no model of a newer exporter has
  an exterior curtain wall.
