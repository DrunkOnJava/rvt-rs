- **A door or window is no longer written as its opening alone when its
  type draws geometry in no material of its own.** RE-84 takes a type
  whose value block holds no geometry-material map as one that only cuts
  its host; the scan of that block missed a map whose values are all unset
  or name a material whose name it did not read. The type's native map
  now decides: it is empty on every door and window Revit exports as its
  opening alone (RE1's CasedOpening, Snowdon Towers' 47), and those keep
  their openings; no export of the reference models, the BIM4LCA office,
  the R2025 models or Autodesk's samples changes (#1247, RE-299).
