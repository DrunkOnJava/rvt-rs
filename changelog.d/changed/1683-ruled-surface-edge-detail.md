- **A ruled-surface component is refused only where a ruled face borders an edge
  that names fewer than two faces (#1683, RE-422).** The guard that makes the
  ruled faces of a component share one grid refused the whole component for an
  edge of any of its faces that named one face and nothing on its other side,
  which kept four roofs of Snowdon Towers Architectural (Revit 2024) as the level
  block of their sketch outline: their ruled faces are closed, and the open edges
  are on the planes and the cylinder of their sides. A ruled face's neighbours
  are the faces its own edges name, so such an edge cannot join two ruled faces.
  The refusal that remains reads `malformed ruled-surface edge (m_pFace is
  <class> + <class>) is connected to face #`, each entry the class its pointer
  resolves to or `none`. Measured numbers follow the run.
