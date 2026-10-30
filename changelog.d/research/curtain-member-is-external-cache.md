- **Why Revit's export writes `IsExternal` true on 528 curtain members
  (RE-269, #35).** The value is not in the model: revit-ifc's
  `IsElementExternal` gives a curtain mullion or panel its curtain wall's
  value, and the exporter that made the BIM4LCA office's exports (24.0.5.432)
  also caches each member type as not external once the first wall that uses
  it is done, so every later wall's members of that type read false. Replaying
  that cache over the walls in ElementId order is Revit's value on all 10,263
  members and panels of the office as built (498 mullions and 30 panels true)
  and all 10,300 at building permit, none different; the host rule is Revit's
  on 1,651 and 1,691. rvt-rs still writes no `IsExternal` on curtain mullions
  and panels: the exporter version is not in the model, and exporters from
  24.2.20 on do not cache types. A probe lists each member's type, host and
  the host's Function, and `tools/re/member_is_external_cache_vs_ifc.py`
  scores both rules and prints the export's exporter version.
