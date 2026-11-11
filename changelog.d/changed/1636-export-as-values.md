- **Four more "Export to IFC As" values are acted on (#1636, RE-417).** On each
  of the four large files of the BIM4LCA housing, 42 `Turning Free Space`
  generic models carry `IfcSpaceType`, 3 generic models `IfcFurniture`, 1 roof
  `IfcSolarDevice` and 1 piece of specialty equipment `IfcTransportElement`, and
  Revit's export writes them as such where rvt-rs wrote proxies and a roof. They
  are written as `IfcSpace` (`INTERNAL`, drawn from their saved meshes),
  `IfcFurniture` (`USERDEFINED`), `IfcSolarDevice` (`SOLARPANEL`) and
  `IfcTransportElement` (`ELEVATOR`), each value limited to the class it was
  seen on, and a value that names an entity's type object no longer counts as
  one the export does not apply. 122 of the 126 models measured are
  byte-identical; the values rvt-rs writes that Revit omits fall from 163 to 168
  to 27 to 32 on each housing file.
