- **A material's colour reaches IFC viewers.** rvt-rs wrote each coloured
  material's surface style as an `IfcStyledItem` with no `Item`, which IFC4
  allows only inside an `IfcStyledRepresentation` of an
  `IfcMaterialDefinitionRepresentation`; none was written, so no viewer could
  find the colour, and elements showed in its default grey. Each coloured
  material now has that representation, naming the material, so a viewer
  colours every element associated with it. The glTF export already carried
  the colours. Found by the batch 9 export audit.
