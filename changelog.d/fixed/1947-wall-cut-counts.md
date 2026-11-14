- **Every wall-cut step that writes no opening is counted (#1947).** In a wall
  not drawn as an axis-aligned prism, `wall_body_not_an_axis_aligned_prism`
  counted a door's step only where its cut flag read 0 and never counted a
  shadow step, though a framed wall counts both. A cutter a wall's record names
  in two steps was written once and the second step dropped unseen; it is now
  counted as `cutter_named_again_in_wall`. No measured export holds either
  case: 61 of 61 exports and the counts in 43 of 43 `diagnostics.json` files
  are unchanged.
