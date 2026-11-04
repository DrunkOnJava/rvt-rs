- **A wall whose T-joint host lies above its base stops at the wall that holds
  its base (#1396, RE-337).** The end of a wall that meets one wall with that
  wall's side stopped half the host's thickness short of the wall's line
  (RE-222), whatever height the host stood at. Where the host lies wholly above
  the wall's base, the end's `JoinEndGStep` names the walls its geometry is cut
  against, and one of them other than the host holds the base, the end now
  stops at that wall's face. Revit's body steps the end up the height and rvt-rs
  draws one prism, so the end is Revit's at the base and below the wall that
  holds it. The measured effect is in
  `reports/element-framing/RE-337-tee-host-above-the-base.md`.
