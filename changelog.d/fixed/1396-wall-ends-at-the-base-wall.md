- **A wall with an edited elevation profile that stops short of the wall beneath
  it is drawn to that wall's face (#1396, RE-337).** The Basement walls of the
  BIM4LCA office that stand on the sloped footings of RE-336 were cut back, by
  the join of their record, to the face of a Level_01 wall that overlaps them by
  a fraction of their height; Revit's body at the base runs on to the face of the
  Basement wall beneath. A wall with an edited profile that is a rectangle along
  a model axis now has an end drawn to the face of the one perpendicular wall
  beneath it that holds its base and spans its width, where it stands beyond the
  end by no more than its own thickness and no such wall touches the end. It
  carries `BodyEndSource` and `EndCoverElementId`. Revit steps the end up the
  height and rvt-rs draws one prism, so the end is Revit's at the base. The
  measured effect is in
  `reports/element-framing/RE-337-wall-ends-at-the-base-wall.md`.
