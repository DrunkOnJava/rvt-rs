- **mahach666's findings on the parameter store and on sketch to solid
  (RvtIO, Discussion #1241) are measured where bytes can state them and cited
  as reported, not measured, where only the Revit API did (#155, #156,
  RE-344).** On Autodesk's 58 sample projects, the BIM4LCA office, Core
  Interior, the RE1 and R2025 models and 13 families (Measure run
  42039153578), none of 641,480 positive parameter ids lacks a definition
  (the ElementId of a `ParamElem*` element of the same document); the 4,818
  records with no saved metadata are outside that test, and a negative id
  never has a definition, so that side tests nothing. Of the stored lengths
  that are whole or tenth millimetres, `mm * (1 / 304.8)` reproduces the most
  in the family slots (69% to 93% of the values on which seven conversions
  differ), as RE-310 found on 2024 to 2027, and `mm / 1000 / 0.3048` the most
  in the typed Double sets (58% and 59%). The set of conversions is not
  exhaustive: a seventh, `mm / 10 * (1 / 2.54) * (1 / 12)`, alone reproduces
  7,413 and 6,195 typed values. 17% and 21% of the typed values and 4% to 26%
  of the slots' match none of the seven, and 66% to 76% of those are more than
  3 units in the last place from every one, so a search for a length by its
  bits has to try several and cannot take a miss as absence. That the shares
  are alike on 2015 to 2023 and on 2024 to 2027 shows the stored bits survive
  re-saving, not that Revit's conversion is the same across releases: the
  samples are re-saves of one model. A `RevolutionElem` holds -1001802 and
  -1001803 on 550 form-and-file pairs (at most 207 counted once, files of
  identical form counts taken as one), each value a multiple of the
  14-decimal 3.14159265358979, a truncation of pi seven units in the last
  place below it: 517 pairs are 0 and 2 pi less seven units, and none of the
  555 nonzero values equals a multiple of pi/2 by bits, so a comparison with
  pi or 2 pi by bits fails on all of them. A `BlendElem` holds -1001804 and
  -1001805 on 1,229 pairs (at most 278), and none of 5,006 `SweepElem` (at
  most 1,383) holds any extent id. The `PartAtom` of the 2014 family of
  RE-327 gives each of its 25 shared parameters a GUID, 2,800 in 112 types,
  and the 12 corpus families, which use no shared parameter, hold none. The
  unit factors beyond the specs rvt-rs converts, the angle meaning of the
  revolution ids, the host areas and the volume rules rest on the Revit API
  and on models that are not in their folder; they are reported, not
  measured, and nothing that rests on NDA material is used (`CLEANROOM.md`,
  the maintainer's decision of 2026-11-05). Against the counts in RvtIO's
  table, `main` (5988d839, Measure run 42028541274; `main` has moved since)
  differs by category on `rac_basic` by 5 to 11 building elements (3
  Furniture Systems, a Roof, a Mass, and from 2023 on 4 mullions and 2
  curtain panels), on `rst_basic` by the 346 reinforcing bars and 16 element
  assemblies it does not read (and, on the 2022 file, the 1 space RvtIO
  writes), and on `rac_advanced` by 192 curtain panels, a door, an assembly
  and 2 Masses (and 25 Areas on the 2021 save). The causes of the roof, the
  panels, the door, the mullions and the Masses are not known. Revit's public
  export of `rac_advanced` holds the 1,349 panels and 100 doors rvt-rs reads
  (run 42031090233), but it is an export of the 2020-era state, not of the
  files where RvtIO reads 1,541 panels and 101 doors. `rac_basic`'s 14 rooms
  come back with #1435.
