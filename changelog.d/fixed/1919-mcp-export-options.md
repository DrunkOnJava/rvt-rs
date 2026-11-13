- **The `export_ifc` tool's schema says which `rvt-ifc` switches it offers and
  what its mesh choices draw (#1919).** The `options` description called them
  "the switches that change the model", but `--placeholder`,
  `--diagnostic-proxies` and `--family-extrusions`, which do, are not offered;
  it now says so. The `default`, `doors_and_windows` and `all` choices of
  `saved_meshes` also draw the walls that embed a curtain wall from their saved
  graphics where Revit raised their base, on Revit 2024 to 2027 (RE-340), and
  `off` keeps their record's body, none of which the descriptions said.
  `docs/mcp.md` and the committed tool contract say the same.
