- **A stair landing of a monolithic type gets its family and its type
  object (#2046, RE-491).** rvt-rs named a landing's family only where its
  landing type is not monolithic (`Non-Monolithic Landing`), so a monolithic
  landing had no family, and with no family no `ObjectType` and no
  `IfcSlabType`. Revit 2024's own export of the BIM4LCA office, made by an
  English Revit, names the family of all 21 of its monolithic landings
  `Monolithic Landing`, and rvt-rs now writes that word, on Revit 2024 from
  the landing type's flag and on Revit 2025 to 2027 from its
  `m_hasMonolithicSupport`. The family words of a stair's parts are the ones
  an English Revit writes: the file does not hold them, and a Revit in another
  language writes its own (a Spanish Revit 2027 writes `Descansillo
  monolítico`), so on such a model the type object is there and its family
  word differs from that export's.
