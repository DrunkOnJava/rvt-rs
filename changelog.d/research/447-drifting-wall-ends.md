- **A wall whose line drifts a hair off a model axis keeps its line's ends
  (RE-447, #1806).** Two walls of the BIM4LCA concrete housing at permit lost
  an exact end when #1807 gave them their type's thickness. Their lines drift
  about 5e-8 ft across an axis, so they are drawn from their line's ends and
  not their joined record box's. Over the 21 ends of four BIM4LCA models where
  the box and the line differ, the box is nearer Revit's end at 12 and the
  line at 9. At each of those 9, Revit's end lies on a layer plane inside a
  joined wall, which no box end reaches. The best per-end rule, from the end's
  own join record, picks right at 19 of the 21 and is never worse than the
  line. It was fitted on those same ends, though, and no other wall of the
  119 exports drifts. So nothing ships, and the ends join #1814.
  `tools/re/wall_ends_list.py` now lists every wall with an end off by more
  than the tolerance, not only the 40 largest.
