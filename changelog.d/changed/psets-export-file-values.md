- **The property census counts what a project's own export-time property set
  file places apart, as out of W7's scope (RE-239).** The maintainer ruled
  those values out of scope on 2026-10-26; `tools/re/psets_vs_ifc.py` now
  lists them apart instead of as missing: in an IFC4 Reference View export,
  the glazing set on elements that are not doors or windows, and the space
  occupancy set. On the BIM4LCA office as built that is 1,981 values (1,907
  at permit); with `--revit-set-placement` the census's missing values go
  from 2,381 to 455 and its different ones from 56 to 1, and equal is
  unchanged on every model. No export changes.
