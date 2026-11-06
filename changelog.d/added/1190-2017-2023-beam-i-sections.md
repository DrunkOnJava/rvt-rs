- **Revit 2017 to 2023 steel beams are drawn as their type's I section (#1190,
  RE-365).** On `rst_basic` 2019 to 2023 all 323 beams drawn along their lines are
  now their 2024 save's, from 312. Revit 2015 and 2016 hold their sections under
  ids of the document's own, which are not read. **No beam of these releases is
  scored against a Revit export here**; each is held to the same Autodesk project's
  later save, which cannot catch an error that save shares.
