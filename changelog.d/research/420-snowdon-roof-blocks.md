- **Why Snowdon Towers Architectural's roofs stay level blocks (#1630, RE-420).**
  Six of its 20 roofs keep their block, each with exactly one decoder diagnostic
  and no mesh (four `malformed ruled-surface edge is connected to face #`, a kind
  no other class shows; one `shared curved edge changes its fixed parameter`; one
  `multiple cylinder trim loops unsupported`), 33% to 133% over Revit's volume,
  and the other fourteen are drawn from their saved meshes and read within
  0.061% of it (the cosine of a 2.0 degree slope on seven, RE-415). The six
  decoders are #1683. Nothing in the reader changes.
