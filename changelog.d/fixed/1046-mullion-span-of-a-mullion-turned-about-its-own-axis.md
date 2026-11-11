- **A curtain mullion turned near 45 degrees about its own axis gets its `Span`
  and `Length` (#1016, RE-226).** RE-106 draws such a mullion as the box along
  its own axes, but that box is not solvable from the record box where the turn
  leaves the box's two horizontal extents nearly equal, so `add_mullion_span`
  wrote nothing: 18 of the BIM4LCA office's 10,003 mullions as built. Where the
  mullion's own Z is one of the model's axes, the record box's extent along that
  axis is the mullion's length, its two other axes being square to it, and 6 of
  the 18 now read a `Span` of 0.7 m, equal to Revit's export's. The other 12,
  turned about a horizontal axis so that their own Z crosses the model's axes,
  keep no `Span` or `Length`.
