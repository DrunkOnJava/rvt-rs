- **A gutter is an `IfcFlowSegment` typed by an `IfcPipeSegmentType`
  `GUTTER`, as Revit's IFC4 export writes it, unless its own "Export to IFC
  As" names `IfcPipeSegment` (#1989).** The BIM4LCA housing's 3 gutters, in each
  of its six files, were written as an `IfcPipeSegment` and two `IfcCovering`
  (their own value says `IfcCovering` `ROOFING`, which Revit's export does not
  act on for a gutter) where Revit writes three `IfcFlowSegment`s. The office's
  24 gutters carry an `IfcPipeSegment` value and stay `IfcPipeSegment`. A
  gutter written as an `IfcFlowSegment` has no `PredefinedType`, quantity set
  or invert, as Revit's has none.
