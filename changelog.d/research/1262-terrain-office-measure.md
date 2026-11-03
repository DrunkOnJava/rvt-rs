- **Measure scores the BIM4LCA terrain office, in concrete and in timber,
  against Revit's exports of them.** `-f nordic=true` now reads the two
  files at building permit and their IFC4 exports by byte range, beside the
  concrete office. `probe_re312_parking_spaces` and
  `tools/re/parking_spaces_vs_ifc.py` list the family instances written as
  `IfcSpace` against Revit's export and the model forms of their families
  (RE-312, #1262).
