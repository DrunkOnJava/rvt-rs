- **mahach666's findings on the parameter store and on sketch to solid
  (RvtIO, Discussion #1241) are measured where bytes can state them and cited
  as reported, not measured, where only the Revit API did (#155, #156,
  RE-344).** On Autodesk's 58 sample projects, the BIM4LCA office, Core
  Interior, the RE1 and R2025 models and 13 families (Measure run
  42030865532), none of 641,480 positive parameter ids lacks a definition
  (the ElementId of a `ParamElem*` element of the same document); the 4,818
  records with no saved metadata are outside that test, and a negative id
  never has a definition, so that side tests nothing. The `PartAtom` of the
  2014 family of RE-327 gives each of its 25 shared parameters a GUID, 2,800
  in 112 types, and the 12 corpus families, which use no shared parameter,
  hold none. The unit factors beyond the specs rvt-rs converts, the host areas
  and the volume rules rest on the Revit API and on models that are not in
  their folder; they are reported, not measured, and nothing that rests on
  NDA material is used (`CLEANROOM.md`, the maintainer's decision of
  2026-11-05). Against the counts in RvtIO's table, `main` (5988d839,
  Measure run 42028541274; `main` has moved since) differs by category on
  `rac_basic` by 5 to 11 building elements (3 Furniture Systems, a Roof, a
  Mass, and from 2023 on 4 mullions and 2 curtain panels), on `rst_basic` by
  the 346 reinforcing bars and 16 element assemblies it does not read (and,
  on the 2022 file, the 1 space RvtIO writes), and on `rac_advanced` by 192
  curtain panels, a door, an assembly and 2 Masses (and 25 Areas on the 2021
  save). The causes of the roof, the panels, the door, the mullions and the
  Masses are not known. Revit's public export of `rac_advanced` holds the
  1,349 panels and 100 doors rvt-rs reads (run 42031090233), but it is an
  export of the 2020-era state, not of the files where RvtIO reads 1,541
  panels and 101 doors. `rac_basic`'s 14 rooms come back with #1435.
