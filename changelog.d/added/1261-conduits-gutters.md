- **Conduits, conduit fittings and gutters are exported from their element
  records, on every release the exporter reads.** `OST_Conduit`,
  `OST_ConduitFitting` and `OST_Gutter` join the recovered categories under
  the unchanged RE-21 instance rule, with the record's box as the body and the
  Level its record names (a gutter's by its elevation). Against Revit's own
  IFC2X3 export of BIM-Speed's Vitoria `ALD26 Before` (Revit 2018), the one
  file with an export that holds any of them, all 102 conduits, 74 conduit
  fittings and 2 gutters the rule selects are elements of Revit's by `Tag` and
  none is outside it: 1,314 of its 1,321 elements are now written (1,136
  before), 1,305 of them in Revit's class or IFC4's for its IFC2X3 one
  (conduits as `IfcCableCarrierSegment`, their fittings as
  `IfcCableCarrierFitting`, gutters as Revit's `IfcFlowSegment`, no
  `PredefinedType`), and 1,251 on Revit's storey (1,073 before), with every
  validation finding at 0 and the properties Revit does not write unchanged at
  20. The 7 still missing are 4 panel walls and 3 assemblies. Cable trays and
  cable tray fittings are not recovered, since no file with a Revit export
  holds one, and neither are the conduit centerline categories Revit's export
  leaves out. Autodesk's MEP samples of 2015 to 2027 (19 or 20 conduits and 14
  conduit fittings on `rme_advanced` and `rme_basic`) and Snowdon Towers
  Electrical (530 conduits and 322 fittings) now export them too, counted and
  not scored: no Revit export of those files exists. See
  [RE-319](reports/element-framing/RE-319-conduits-conduit-fittings-gutters.md)
  (#1261).
