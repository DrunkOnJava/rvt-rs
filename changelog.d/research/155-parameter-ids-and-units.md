- **mahach666's findings on the parameter store and on sketch to solid
  (RvtIO, Discussion #1241) are measured where bytes can state them and cited
  as leads where only the Revit API did (#155, #156, RE-344).** On Autodesk's
  58 sample projects, the BIM4LCA office, Core Interior, the RE1 and R2025
  models and 13 families (Measure run 42030865532, probe at 27875749), none
  of 1,545,257 parameter ids breaks the sign rule: negative for a built-in
  parameter, positive for the ElementId of a `ParamElem*` definition of the
  same document. Of the stored lengths that are whole or tenth millimetres,
  `mm * (1 / 304.8)` reproduces the most in the family slots, on Revit 2015
  to 2023 as RE-310 found on 2024 to 2027, and `mm / 1000 / 0.3048` the most
  in the typed Double sets; 37% of the typed values and 5% to 29% of the
  slots' match none of six conversions, so a search for a length by its bits
  has to try several. A `RevolutionElem` holds -1001802 and -1001803 on 550
  of 550 (517 full turns, stored with the end 2 pi less seven units in the
  last place, and 33 partial), a `BlendElem` -1001804 and -1001805 on 1,229
  of 1,229, and none of 5,006 `SweepElem` any extent id. The `PartAtom` of
  the 2014 family of RE-327 gives each of its 25 shared parameters a GUID, 2,800
  in 112 types, and the 12 corpus families, which use no shared parameter,
  hold none. The unit factors beyond the specs rvt-rs converts, the angle
  meaning of the revolution ids, the host areas and the volume rules rest on
  the API and are recorded as reported, not confirmed (`CLEANROOM.md`, the
  maintainer's decision of 2026-11-05). Against the counts in RvtIO's table,
  `main` (5988d839, Measure run 42028541274) differs on `rac_basic` by 5 to
  11 building elements (3 Furniture Systems, a Roof, a Mass, and from 2023 on
  4 mullions and 2 curtain panels), on `rst_basic` by the 346 reinforcing
  bars and 16 element assemblies it does not read, and on `rac_advanced` by
  192 curtain panels, a door, an assembly and 2 Masses (and 25 Areas on the
  2021 save); Revit's public export of `rac_advanced` holds the 1,349 panels
  and 100 doors rvt-rs reads (run 42031090233). `rac_basic`'s 14 rooms come
  back with #1435.
