- **`unsupported_features` lists
  `revit_compound_assemblies_and_walltype_widths` only for a file with a
  host element written without its layers.** It was listed on every file,
  whatever it held, though each wall, floor, roof and ceiling whose type's
  compound structure is read has been written with its layers, materials and
  widths as an `IfcMaterialLayerSetUsage` since RE-58 (Revit's on 88 of 88
  sets on Core Interior and 15 of 15 on RE1 Architecture). It now goes where
  every wall, slab other than a stair landing, roof and ceiling has a layer
  set: on RE1 Architecture, Electrical, Mechanical and Plumbing, and
  Autodesk's `rst_basic` 2024 and 2027, of 21 exports measured. Core
  Interior (41 walls and 12 floors without) and Einhoven (26 walls) keep it.
  The IFC is unchanged.
