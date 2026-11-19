- **A ceiling whose type says "Export Type to IFC As" `IfcSlabType` is written
  as a covering, as Revit's export writes it (#2074).** The BIM4LCA timber
  housing's ceiling type `AK-4.EI60` carries `IfcSlabType` with `USERDEFINED`,
  and rvt-rs wrote its one ceiling as `IfcSlab` `USERDEFINED` with an
  `IfcSlabType`. Revit's export ignores both values for a ceiling and writes
  `IfcCovering` `CEILING`. The ceiling now ignores them too and is written as
  Revit writes it, with its covering properties and `GrossArea`, all equal to
  Revit's. Of the 120 exports measured, held-out ones included, it is the only
  element that moves, so the rule rests on that one ceiling.
