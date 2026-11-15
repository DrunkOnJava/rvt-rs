- **A wall's quantities are no longer read from a saved mesh that failed to
  decode in a way that can leave part of the wall out (#1947, #2020).** A beam's
  and a column's mesh were already refused when any of their solids failed to
  decode. A wall's mesh is now refused for any failure but a view-resolved
  instance. That is the only failure on the 49 partial walls among the 7,406
  measured walls with a mesh, all in Autodesk's `rac_advanced` (2023 to 2027)
  and Snowdon Towers Architectural samples, and on the 17 of them checked
  against Revit's own drawn body the decoded mesh is that body. No measured
  export changes.
