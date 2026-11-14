- **A door's or window's type takes no guessed flip in its GlobalId (#1947).**
  Revit hashes a door's type GlobalId with the door's flip (RE-167), and
  RE-167 measures that a door with no host is not flipped. rvt-rs also took a
  hosted door whose host frame or facing was not read as not flipped. Such a
  door's type now gets the GlobalId the writer derives, as any type rvt-rs
  cannot match does. Found by the batch 9 export audit.
