- **The property census counts curtain members' `IsExternal` apart, as the
  exporter's own output (W7, RE-269).** The exporter that made the BIM4LCA
  office's exports gives a curtain mullion or panel its curtain wall's
  `IsExternal` and then caches each member type as not external, so every
  later wall's members of that type read false: a value the model does not
  hold, which rvt-rs does not write (the maintainer, 2026-10-30).
  `tools/re/psets_vs_ifc.py` no longer counts those values as missing, nor
  the mullion types' own: 10,267 on the office as built (10,003 mullions,
  260 panels, 4 mullion types) and 10,304 at building permit are listed
  apart, with the other values Revit's export gets wrong (RE-235 §5). It
  counts them only where the export's own type objects hold the value, so
  RE1 Architecture's 12 members, exported by a later exporter, are still
  scored. On the office as built the census goes from 12,672 missing of
  57,773 to 2,405 of 47,506 with `--revit-set-placement`, and from 13,638
  to 3,371 without it; equal (45,045 and 44,079) and different (56) do not
  move, and no export changes.
