- **`rvt-ifc-compare`'s note on spaces says how a room is drawn now.** It
  said a room whose solid does not close keeps its bounding box. Such a room
  is read again from its solid's faces or edges; from Revit 2023 one none of
  those reads is drawn from its saved mesh where that mesh decodes whole
  (#1191), and only otherwise does it keep its box (#2130).
