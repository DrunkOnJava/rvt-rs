# RE-344: Parameter ids, stored lengths and a family's form extents, against RvtIO's findings

**Date:** 2026-11-05
**Issues:** #155 (the parameter store, its ids and units) and #156 (sketch to solid); Discussion #1241; follows RE-77, RE-93, RE-207, RE-310 and RE-327
**Credit:** mahach666, author of RvtIO (a .NET reader of Revit files, MIT), who published findings on both issues in Discussion #1241 on 2026-11-05: the record shapes of the parameter stores, the sign rule of parameter ids, the unit factors, the two millimetre conversions and the form parameters of extrusions and revolutions, with the count tables for Autodesk's samples that section 4 answers. This report restates facts with that credit and copies none of their tables or prose.
**Artefacts:** Autodesk's sample projects of Revit 2015 to 2027 as `research/autodesk-sample-projects.tsv` pins them, the `phi-ag/rvt` family corpus, the BIM4LCA office, Core Interior, the RE1 models and puzzbobb's R2025 models, as Measure fetches them; none is committed.
**Probe:** `probes/examples/probe_re344_parameter_ids_and_units.rs`
**Runs:** Measure 42028541274 (`main` at 5988d839, the samples of 2021 to 2027, section 4), 42031090233 (the same head, the class sets of `racadv-2020` against Revit's public IFC2X3 export, section 4.2), 42030865532 (the probe at 27875749: `-f base=none -f samples=true -f sample_years="2015 2027" -f families=true -f nordic=true -f r2025=true -f scorers=none`, 105 models) and 42039153578 (the same selection, the changed probe at head 37f377d3, sections 5.1 to 5.5). The probe's code is the one at 37f377d3; the two runs' counts of 5.1, 5.2 and 5.5 are identical.
**Status:** measured. Sections 4 and 5 name their runs. Nothing here changes an export: no value is added.

## 1. What the findings rest on, and what may be used

mahach666 states the sources of every finding: bytes of Revit files (including the class schema each file carries in `Formats/Latest`), the output of the public Revit API (Revit 2022) on Autodesk's sample projects and on their own models, and the IFC specification. They state that no program or library was disassembled or decompiled and no converter's output was used. Their findings carry their own marks: VERIFIED (compared with an independent value), OBSERVED (bytes only) and UNKNOWN.

`CLEANROOM.md` lists public byte observations, `RevitAPI.dll`'s exported symbols, previously published reverse-engineering work, buildingSMART's specifications and `revit-ifc` as accepted sources. The output of the Revit API run on a file is on neither list, accepted or forbidden. This report therefore splits every finding by what it rests on, and takes only the facts that can be stated and proved from bytes of files Measure fetches, against Revit's own IFC export or Revit's saved geometry where one exists. A value that only an API capture confirms is recorded as reported and not measured. The maintainer decided on 2026-11-05 that a third party's API capture is published reverse-engineering work (accepted source 3): it is cited and used to know where to look, and a value lands only when bytes of files CI can fetch, with Revit's own export or saved geometry, confirm it. `CLEANROOM.md` says so in one sentence.

Nothing that rests on NDA material is used (`CLEANROOM.md`, forbidden source 4). mahach666 says in the discussion that the other models they test on locally are real projects under NDA, and their sketch finding says its volume and area comparisons rest on their own models and on API captures that are not in the folder; the folder does not say which of those are NDA material. Whatever rests on models or captures that are not in the folder (the unit-factor, 1713-of-1872, host-area and volume rows of section 2) is cited as reported, not measured, and no value, number or rule is taken from it.

Other handling:

- The folder carries no licence: no licence file, and no licence statement in its README. Facts (counts, layouts, ids) are restated here with credit; no table, prose or script is copied or committed. RvtIO's own repository is MIT.
- None of their scripts was run. Their parameter probe needs RvtIO's command line to dump the schema and the inflated partitions, and the others read the files themselves; this probe proves the same facts on Measure's runners with rvt-rs's own readers. Their finding files, count tables and outputs were read, and only the opening of one probe.
- Their copies of Autodesk's samples were not used (Measure fetches the pinned files from Autodesk's host) and their NDA models were never available.
- RvtIO's README says its built-in name tables are re-verified against copies of Revit's resource DLLs, and mahach666's reply says their findings use only bytes of Revit files, the public Revit API's output and the IFC specification. The two are not reconciled. This report takes only ids from RvtIO and no name: parameters are named by id here and in the probe, and the names rvt-rs already uses come from its own exports (RE-77, RE-93), so none of the tables is used. `CLEANROOM.md` asks the contributor to confirm a source that cannot be confirmed as an accepted one, and the draft reply to the discussion asks how the tables were produced.

## 2. Which findings are new to rvt-rs

"Bytes" means the finding can be stated and proved from bytes of the files Measure fetches; "API" means their evidence is a capture of the Revit API, which is cited and recorded as reported, not measured.

| Finding (as RvtIO reports it) | Rests on | rvt-rs before | Here |
|---|---|---|---|
| Each element has four typed value sets (Double, Int, AString, ElementId) in the first four pointer fields of `Element` | bytes | known: RE-207 section 2, the native reader | not new |
| A family parameter is a slot of `FamilyParams.m_params` with one member per storage type, its `m_elemId` and its `m_paramId` | bytes | known: RE-207 section 2 and 7. RE-93's entry `f64 feet · ff×8 · i64 parameter` has the layout of such a slot (the value, an unset `m_elemId` of -1, the parameter): the identification is an inference from the two layouts | inferred, not tested: the probe reads the slots and does not compare them with RE-93's entries; section 5.3 leans on it only to compare with RE-310 |
| The id is negative for a built-in parameter and positive for the ElementId of a definition element (`ParamElem*`) of the same document | bytes | in the code (`ParameterKind`), never counted | measured on the positive side, section 5.1: no positive id lacks a definition (the negative side tests nothing: definitions are keyed by positive ids) |
| The Double item of the 2027 schema has the id before the value | bytes | read by the schema, so right by construction, not documented | section 5.2: the 2027 samples read as the 2026 ones do |
| The shared parameter's GUID is the 16 bytes of `ParamElemExternal.m_externalParamKey` | bytes, and the API for the captions | known (`Definition::shared_guid`, RE-207) | not new |
| `PartAtom` holds no GUID in the sample families | bytes | #155 recorded the claim as reported | section 5.5: none in the 12 corpus families, as they say; the `PartAtom` of a real family with shared parameters gives each its GUID |
| `ExtrusionElem` holds -1001800 and -1001801 | bytes; the API for which is the start | known: RE-327, proved against the saved meshes | not new |
| `RevolutionElem` holds -1001802 and -1001803, a full turn stored as 0 and 2 pi less seven units in the last place | bytes for the ids and the values; the API for "start angle" and "end angle" | not known | the ids and the values are measured in section 5.4, with revolutions they did not have; that they are the start and end angle is reported, not measured |
| `BlendElem` and `SweptBlendElem` hold -1001804 and -1001805 | bytes (unverified by them) | not known | measured in section 5.4 |
| Unit factors for specs rvt-rs does not convert (mass density, specific heat, thermal conductivity, resistivity, permeability, stress, unit weight, the scale factors): every dimensioned quantity is stored as its SI value times a power of 0.3048 | the API (the display text of `AsValueString`; the captures are not in the folder) | length, area, volume, angle, air flow, pressure, power, potential and current are converted, each measured against Revit's exports (RE-207, RE-210) | reported, not measured |
| `n / 25.4 / 12` and `n / 304.8` differ for 57,067 of the integers 1 to 100,000 (54,471 by one unit in the last place, 2,596 by two) | arithmetic | RE-310 measured that no one conversion reproduces the stored lengths, without this count | reproduced with an independent program, 42,933 / 54,471 / 2,596; the probe prints it from Rust's `f64` |
| No one conversion explains the stored values; the forms `n / 12 / 25.4` and `n / 1000 / 0.3048` exist | bytes | RE-310 (four forms, 2024 to 2027) | the same finding reached independently; section 5.3 adds three forms and Revit 2015 to 2023, and the typed sets |
| The stored double equals the API's in 1713 of 1872 items | the API (a capture of one file's parameters, not in the folder) | not applicable | reported, not measured |
| The element-block walk closes exactly on 859,650 of 859,700 blocks of 19 project files | bytes | the native reader frames every group by its declared counts (RE-280) | a different method; no number to compare |
| Blocks outside the element table of a project (17 `ExtrusionElem`, 10 `SweepElem` in `rst_basic` 2022) were not examined | bytes | the records outside the element table are loaded families' own documents (RE-135, RE-163, RE-204; RE-327 counts 9,587 extrusions in them on the samples); that these 27 are such records is not itself measured | answers their open point in kind |
| Sketch curve classes, `GLine` and `GArc` fields, curves in model coordinates, loops by end points, holes by nesting parity, open chains left out | bytes | known: RE-327 sections 2 and 3, on 12 releases of the family corpus and 6,172 sample extrusions | not new |
| A floor's, ceiling's or roof footprint's area is the net area of its sketch's loops (98 of 105 hosts of four 2022 samples) | the API (the Area parameter of four 2022 samples, a capture not in the folder) | not applicable: rvt-rs reads these from element records | reported, not measured |
| Revolution volume is the angle times the profile's first moment about the axis; a sweep's is the profile area times the path length | the API (volumes of their own models, which are not in the folder and some of which may be under NDA) | revolutions and sweeps not rebuilt (RE-327 section 8) | reported, not measured |
| The stored bounding box is bit-identical to the API's (1019 of 1019) | the API | known as bytes: RE-32 | not new |
| `Formats/Latest` grammar, tags counted from 12, class counts by release | bytes | known: `schema_registry`, #255 review | not new |
| Paging segments of 353 bytes every 64,896 bytes; sector size 4096 | bytes | known: `compression.rs` | not new |
| Each `Partitions` member is inflated with the previous member's last 32 KiB as a preset dictionary | bytes | not known: rvt-rs inflates every member on its own and checks each member's CRC-32 and size before it accepts a group (`native_segments`) | a question in the discussion reply, not a finding here |
| The riser height and tread depth of a stair run lie as a pair near its id | bytes, with weak attribution by their own account | not known as a pair | not used: their own attribution is weak |

What is new and can be proved from bytes is therefore small: the sign rule counted on its positive side, the revolution and blend ids and values, three more conversions, the pre-2024 census and the `PartAtom` count. What is new and rests on the API (the unit factors beyond the converted specs, the angle meaning of the revolution ids, the host areas, the volume rules) is cited as reported, not measured: each lands only when bytes and Revit's export or saved mesh confirm it, and nothing that rests on NDA material is used.

## 3. The probe

`probe_re344_parameter_ids_and_units` takes one file and prints, per record:

- `arith`: the unit-in-the-last-place distance of `n / 25.4 / 12`, `n * (1 / 304.8)`, `n * (1 / 25.4) / 12` and `n / 10 * (1 / 2.54) * (1 / 12)` from `n / 304.8` over the integers 1 to 100,000.
- `sign` tallies, per store (`typed` for the four typed sets, `slot` for the slots of every `FamilyParams`): a negative id, a positive id by the class of the definition it resolves to (`positive undefined` for one that resolves to none) and a zero id. A project's definitions are those the native reader resolves; a family file's are its `ParamElem*` records. Only the positive side can fail: definitions are keyed by positive ElementIds, so a negative id never has one.
- `ulp` tallies of the stored doubles that are whole or tenth millimetres and not whole 1/256 inches (RE-310's selection), by store and kind (`builtin`, `declared`, `other`): how many agree under all seven conversions, how many differ, which conversions reproduce the stored bits, and for the values no conversion reproduces their distance from `div` and from the nearest of the seven. The seven are `chain` `mm / 25.4 / 12`, `div` `mm / 304.8`, `mul` `mm * (1 / 304.8)`, `inch` `mm * (1 / 25.4) / 12`, `inv12` `mm / 12 / 25.4`, `metre` `mm / 1000 / 0.3048` and `cmchain` `mm / 10 * (1 / 2.54) * (1 / 12)`. A distance is in units of the last place of the magnitude: a value of either sign whose magnitude is one unit smaller than the conversion's is -1.
- `form` tallies of a family's forms: which parameter ids each form class holds in each typed set, and for `ExtrusionElem`, `RevolutionElem`, `BlendElem`, `SweptBlendElem` and `SweepElem` whether the first and second id are held. For a revolution, the two values' bits, whether the first is below the second, the pair as half turns of the 14-decimal value 3.14159265358979 (a truncation of pi, `0x400921fb54442d11`, seven units in the last place below pi), and whether each is a multiple of pi/2 by bits and to within 1e-9. The probe names the ids first and second and takes no meaning for them from the API.
- `partatom`: the stream's size and the count of strings shaped like a GUID.

A positive id with no definition falsifies the sign rule for the records read; a sweep that holds one of the extent ids, or a revolution that does not hold both, falsifies the form claims; a GUID-shaped string in a corpus family's `PartAtom` falsifies their negative for that file.

## 4. Counts on the same files

RvtIO's counts are the ones in the discussion's `counts.csv` for the 21 samples that are byte for byte the pinned files. rvt-rs's are `exported.building_elements` and `exported.by_ifc_type` of each sample's `diagnostics.json` in Measure run 42028541274 (`-f base=none -f samples=true -f sample_years="2021 2027" -f scorers=none -f models="sample-*"`, `main` at 5988d839). `main` has moved since and these counts are not re-measured on it. RvtIO's table quotes rvt-rs's reports (RE-140, RE-179 to RE-181) at an earlier head, so some of its rvt-rs figures are no longer the head's: rooms follow the export phase since 2026-11-04 (#1328), Areas are written as spaces (#1428), and storeys are bound to the elements they hold.

`building_elements` is the sum of `by_ifc_type`, `IfcSpace` included.

| sample | RvtIO | rvt-rs `main` | the difference, by category (section 4.1) |
|---|---:|---:|---|
| `rac_basic` 2022 | 455 | 450 | 5: 3 Furniture Systems, 1 Roof, 1 Mass |
| `rac_basic` 2023, 2024, 2026, 2027 | 461 | 450 | 11: those 5, 4 mullions, 2 curtain panels |
| `rst_basic` 2022 | 817 | 454 | 363: 346 reinforcing bars, 16 element assemblies, 1 space RvtIO writes and rvt-rs does not on a 2022 file |
| `rst_basic` 2024, 2026, 2027 | 817 | 455 | 362: 346 bars, 16 assemblies; both write the 1 space |
| `rac_advanced` 2021 | 5,878 | 5,657 | 221: 192 curtain panels, 25 Areas as spaces, 1 door, 1 element assembly, 2 Masses |
| `rac_advanced` 2022 | 5,685 | 5,682 | 3: 1 element assembly, 2 Masses |
| `rac_advanced` 2023, 2024, 2026, 2027 | 5,878 | 5,682 | 196: 192 curtain panels, 1 door, 1 element assembly, 2 Masses |

Each list adds up to the difference: the sums are over the categories of RvtIO's `ifc_building_by_category` (its `(no Tag)` row is the spaces) set against the classes rvt-rs reads (`decoded.production_class_counts`). That is bookkeeping, not cause. A cause is known for the reinforcing bars, assemblies and Furniture Systems (rvt-rs does not read them) and for the spaces and Areas (section 4.1). It is not known for the roof, the 192 curtain panels, the door, the 4 mullions and the 2 panels, and the Masses are a rule of rvt-rs's whose agreement with Revit's export is not checked here.

### 4.1 What the class tables show

Their category counts (`ifc_building_by_category`) set against rvt-rs's `decoded.production_class_counts` and `by_ifc_type`, on the 2024 files and the 2022 `rac_basic`:

- **Reinforcing bars and framing systems (`rst_basic`, one assembly in `rac_advanced`).** The element-record sweep reads 42 product categories (`PRODUCT_RECORD_CATEGORIES`) and Rebar, Structural Framing Systems and Furniture Systems are not among them, so rvt-rs writes no `IfcReinforcingBar` and no `IfcElementAssembly`. With RvtIO's 346 bars and 16 assemblies taken away, the two agree on every other category of `rst_basic`: 370 beams, 30 columns, 36 foundations, 9 walls, 5 floors, 2 stringers, 1 stair, 1 run.
- **Masses.** rvt-rs reads the Mass elements (2 in `rac_advanced`, 1 in `rac_basic`) and does not write them: a mass is written only where its own "Export to IFC As" names an entity (#864, from the BIM4LCA office's masses). RvtIO writes each as an `IfcBuildingElementProxy`. This is a rule rvt-rs applies, not an element it fails to read. Whether Revit's export writes these Masses is not checked here: 3 of the 75 `IfcBuildingElementProxy` in Revit's public export of `rac_advanced` are not written by rvt-rs (section 4.2), and which elements they are is not shown.
- **Foundations (`IfcSlab` 35 against 11, and 37 against 5 on `rst_basic`).** The elements are the same. RvtIO writes a Structural Foundation family instance as an `IfcSlab` (22 of its 35 on `rac_basic`, by its own account), rvt-rs as an `IfcFooting`: 23 on `rac_basic` and 36 on `rst_basic`, equal to RvtIO's 23 and 36 foundations. rvt-rs's 11 slabs are 8 floors, 2 pads and the landing.
- **Furniture (`IfcFurnishingElement` 35 against 32).** RvtIO's 35 are 30 Furniture, 3 Furniture Systems and 2 Casework. rvt-rs writes the 30 and the 2. Revit's own export of this project is not available, so which is right is not shown; the 3 elements are in the file.
- **Roofs.** RvtIO has 3 elements of the Roofs category in `rac_basic` and rvt-rs reads 2. Not explained.
- **Storeys.** rvt-rs writes 6 for `rac_basic` on every release and 5 for `rac_advanced`; for `rst_basic` 5 on the 2022 file and 2 from 2024. RvtIO writes none for `rst_basic` and 6 for `rac_basic`, so the 3 in their table is rvt-rs's figure at an earlier head.
- **Spaces.** `rac_basic`'s 14 rooms are in the project's second phase, not its last, so `main` writes none and counts 14 as `room_not_in_export_phase`. #1435 writes them (its Measure run 42011174043, at its head `992196c6`, reports 464 entities for the 2016 to 2027 saves of `rac_basic`, 14 more than the 450 here). RvtIO writes none for `rac_basic` either. `rac_advanced`'s 116 spaces are 91 rooms and 25 Areas; the 2021 save writes the rooms only, as it refuses the 25 Areas of that release (`area_without_proven_body`, `boundary_lines_not_readable`). RvtIO's `rst_basic` and rvt-rs's from 2024 each write 1 space; on the 2022 file rvt-rs writes none.
- **Curtain panels and mullions.** In `rac_advanced` RvtIO writes 1,541 panels on its 2021 file and on every file from 2023, and 1,349 on its 2022 file, the figure rvt-rs reads on every release, 2021 to 2027. In `rac_basic` RvtIO's 2022 file has 144 mullions and 44 panels, rvt-rs's figures, and from 2023 148 and 46. The same project gives RvtIO more panels on some saves and rvt-rs the same on every save. Revit's one public export of `rac_advanced` holds 1,349: section 4.2.
- **One door** (101 against 100 in `rac_advanced`). RvtIO's 2022 file has 100, and so do rvt-rs on every release and Revit's export (section 4.2); its 2021 and 2023 to 2027 files have 101.

### 4.2 Revit's own export of `rac_advanced`

Run 42031090233 (`main` at 5988d839, `-f racadv=true -f models="racadv-2020" -f scorers=class_sets_vs_ifc`) pairs rvt-rs's export of the 2020 save with the one public Revit export of the project, an IFC2X3 export by Revit 2021 (RE-330). That export is counted apart everywhere: it confirms a reader and decides nothing between two exporters. It pairs 5,555 elements and spaces of Revit's (83 repeated Tags left out) with 5,631 of rvt-rs's. Per class of Revit's export, elements rvt-rs writes as the same class:

| Revit's class | Revit | rvt-rs writes it as the same class | as another class | not written |
|---|---:|---:|---:|---:|
| `IfcPlate` | 1,349 | 1,349 | 0 | 0 |
| `IfcDoor` | 100 | 100 | 0 | 0 |
| `IfcMember` | 3,281 | 3,276 | 5 (`IfcStair`) | 0 |
| `IfcSpace` | 116 | 116 | 0 | 0 |
| `IfcColumn`, `IfcCovering`, `IfcRailing`, `IfcRoof`, `IfcSlab`, `IfcWindow` | 176, 65, 22, 6, 16, 24 | all | 0 | 0 |
| `IfcBuildingElementProxy` | 75 | 72 | 0 | 3 |

The 34 curtain walls, 140 standard-case walls, 41 flow terminals and 104 furnishing elements of Revit's export are written as the class rvt-rs uses for each (`IfcWall` for the first two, `IfcLightFixture`, `IfcFurniture`), and the other 6 walls as `IfcWall`, 38 beams and 40 proxies and a slab are written that Revit's export does not hold, and Revit's export has no `IfcElementAssembly`. The 1,349 panels and 100 doors are what rvt-rs reads on every release and what RvtIO reads on its 2022 file; RvtIO's 2021 and 2023 to 2027 files give 1,541 and 101. The 2020 export cannot say whether those saves hold 192 panels and a door more, or whether RvtIO reads some where Revit's export of the earlier state has none: they are different files of the sample, and no Revit export of them exists. The causes of the 192 panels, the door and the 4 mullions and 2 panels are not known.

## 5. Measured

Measure run 42039153578 ran the probe at head 37f377d3 on 105 models: Autodesk's 58 sample projects of Revit 2015 to 2027, the 12 family-corpus files of 2016 to 2027 and the 2014 maker family of RE-327, the four BIM4LCA office files, Core Interior, Einhoven, the four RE1 models and the 24 R2025 models. The native reader read 1,284,631 element records of the 92 projects (4,818 had no saved metadata) and the 12 corpus families (10 had one record that did not decode, 2 had seven). The 2014 family is outside the native releases and was refused, as in RE-327. Run 42030865532 ran the first form of the probe (six conversions, a signed distance, a different revolution tally) on the same selection; its counts for 5.1, 5.2 and 5.5 are identical to this run's, and its six-conversion table for 5.3 is withdrawn: the seventh conversion changes it.

Counts of forms are form-and-file pairs. The samples are re-saves of one model per project and the same families recur in every release's files, so the pairs overcount: section 5.4 also gives the count with files of identical per-class form counts taken once.

### 5.1 The sign rule

Of 1,545,257 parameter ids read from typed sets and family slots (903,777 negative, 641,480 positive), none broke the rule:

| store | negative | positive | a positive id resolves to |
|---|---:|---:|---|
| typed sets | 667,463 | 41,908 | `ParamElemExternal` 35,201, `ParamElemProject` 6,655, `ParamElemRefTableEntry` 52 |
| family slots | 236,314 | 599,572 | `ParamElemFamily` 569,353, `ParamElemExternal` 30,219 |

Only the positive side tests the rule: definitions are keyed by positive ElementIds, so a negative id never has one, and "no negative id had a definition" is true by construction. The test is that no positive id lacked a definition, and none did: 0 of 641,480, each resolving to a `ParamElem*` element of its own document. No id was zero. The 4,818 records with no saved metadata, and any record the native reader does not decode, are outside the test. RvtIO found positive ids in the typed sets of one sample project; here they are on all four storage types.

### 5.2 The 2027 Double item

The 2027 schema puts the id first in the Double item. The reader follows the schema, and the 2027 files give the counts the 2026 files give: typed Double items 3,202, 5,070 and 2,480 and stored millimetre candidates 733, 1,787 and 385 in `rac_basic`, `rac_advanced` and `rst_basic` of both releases.

### 5.3 The stored millimetre lengths

RE-310's selection (whole or tenth millimetres, not whole 1/256 inches), the seven conversions of section 3, projects only. Of the values on which the seven conversions give different bits, the share each conversion reproduces and the share none of the seven does:

| store, kind, releases | values where the seven differ | none of the seven | `mul` | `div` | `chain` | `metre` | `cmchain` | only `metre` | only `cmchain` |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| typed Double sets, built-in, 2015 to 2023 | 35,915 | 17% | 45% | 45% | 33% | 58% | 38% | 2,788 | 7,413 |
| typed Double sets, built-in, 2024 to 2027 | 42,935 | 21% | 46% | 45% | 34% | 59% | 28% | 4,489 | 6,195 |
| slots, built-in, 2015 to 2023 | 13,729 | 12% | 80% | 68% | 64% | 59% | 15% | 0 | 487 |
| slots, built-in, 2024 to 2027 | 21,121 | 26% | 69% | 57% | 60% | 46% | 10% | 10 | 407 |
| slots, declared, 2015 to 2023 | 61,371 | 4% | 91% | 75% | 78% | 61% | 10% | 202 | 1,692 |
| slots, declared, 2024 to 2027 | 163,035 | 4% | 93% | 76% | 85% | 49% | 7% | 680 | 1,897 |

- **The slots agree with RE-310.** `mul` (`mm * (1 / 304.8)`) reproduces the most on the slots in every half, so on 2015 to 2023 as on 2024 to 2027. On the office as built it reproduces 90% of the differing slot values that some conversion reproduces (RE-310: 84% of its type value blocks'). The counts cannot be compared: RE-310 reads the type value blocks once per owner, parameter and value, and this run counts every record's slots (the office: 839 built-in and 4,514 declared values where the seven differ, RE-310: 460 and 2,024). That the slots are RE-310's entries is the inference of section 2, not tested here.
- **The typed sets lead with another conversion.** `metre` (`mm / 1000 / 0.3048`) reproduces the most in the typed Double sets, 58% and 59% of the values where the seven differ, `mul` and `div` 45% and 46%. RE-310 did not read these sets.
- **The seven are not exhaustive.** The first run, with six, left 37% of the typed values unmatched. The seventh, `cmchain` (`mm / 10 * (1 / 2.54) * (1 / 12)`), reproduces 7,413 and 6,195 typed values that no other conversion does, and 487 and 407 built-in and 1,692 and 1,897 declared slot values. Whether other chains reproduce more is not known.
- **Many stored values are none of them.** 17% and 21% of the typed values, 12% and 26% of the built-in slots' and 4% of the declared slots' match none of the seven. Their distance, in units of the last place of the magnitude (a value one unit smaller in magnitude than `div` is -1, for either sign), from `div` and from the nearest of the seven:

| store, kind, releases | none of the seven | -1 | -2 | -3 | +1 to +3 | over 3 | nearest of the seven: 1 | 2 | 3 | over 3 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| typed, built-in, 2015 to 2023 | 5,926 | 1,246 | 57 | 608 | 157 | 3,858 | 1,440 | 628 | 5 | 3,853 |
| typed, built-in, 2024 to 2027 | 9,217 | 876 | 134 | 1,359 | 546 | 6,302 | 1,514 | 1,409 | 93 | 6,201 |
| slots, built-in, 2015 to 2023 | 1,650 | 243 | 178 | 0 | 153 | 1,076 | 532 | 24 | 30 | 1,064 |
| slots, built-in, 2024 to 2027 | 5,473 | 113 | 328 | 2 | 158 | 4,872 | 521 | 66 | 68 | 4,818 |
| slots, declared, 2015 to 2023 | 2,204 | 264 | 649 | 0 | 56 | 1,235 | 931 | 18 | 26 | 1,229 |
| slots, declared, 2024 to 2027 | 7,258 | 419 | 1,118 | 12 | 120 | 5,589 | 1,605 | 24 | 90 | 5,539 |

  Of the 31,728 values none of the seven reproduces, 67% of the typed (15,143) and 77% of the slots' (16,585) are more than 3 units from `div`, and 66% and 76% more than 3 from every one of the seven: most are computed values or come from a conversion not tried, and not one unit off one that was. Exactly one unit below `div` are 14% of the typed and 6% of the slots'. A search that takes a miss as absence fails on all of them.
- **The family files** add 520 built-in typed values (344 in 2016 to 2023, 176 in 2024 to 2027): `cmchain` reproduces 436 (84%), 200 of them alone, and the 72 that none of the seven reproduces are all one unit below `div`.
- **Re-saves.** The 2015 to 2023 shares are close to the 2024 to 2027 ones. The samples are re-saves of one model per project, so that shows the stored bits survive re-saving, not that Revit's conversion is the same across releases.

The probe selects numerically and does not know that a value was typed in millimetres: a value computed from others would be among them.

### 5.4 The parameters of a family's forms

| form | form-and-file pairs | at most, files of identical form counts taken once | holds | does not hold |
|---|---:|---:|---|---|
| `ExtrusionElem` | 10,354 | 2,732 | -1001800 and -1001801 on every one (the first is zero on 3,318 pairs) | |
| `RevolutionElem` | 550 | 207 | -1001802 and -1001803 on every one | |
| `BlendElem` | 1,229 | 278 | -1001804 and -1001805 on every one | |
| `SweptBlendElem` | 322 | 87 | no Double set | both ids on every one |
| `SweepElem` | 5,006 | 1,383 | | any of -1001800 to -1001805 on every one |
| `FreeFormElement` | 208 | 208 | | any of them on every one |

86 files hold forms and have 23 distinct per-class counts; the once-counted figures take one file of each. They are upper bounds on distinct forms: files with equal counts are not shown to hold the same forms.

The revolutions' values, the first id and then the second:

- **Every value is a multiple of one 14-decimal number.** The 550 pairs are, in half turns of 3.14159265358979 (`0x400921fb54442d11`, a truncation of π that is seven units in the last place below π, `0x400921fb54442d18`): 517 from 0 to 4 (at most 186 counted once), 28 from 0 to 2 (16), 2 from 1 to 3 (2) and 3 from 2 to 4 (3). So 21 of the at most 207 once-counted pairs are not 0 to 4. The first is below the second on all 550.
- **The seven units in the last place come from that number.** The second id is `401921fb54442d11` on 520 pairs (twice the truncation), `400921fb54442d11` on 28 (the truncation itself, not π, though the probe's first form labelled it "other") and `4012d97c7f3321cd` on 2; the first is `3ff921fb54442d11` on 2 and `400921fb54442d11` on 3, and 0 on 545. Not one of the 555 nonzero values equals a multiple of π/2 by bits; all 555 are within 1e-9 of one. A reader that compares an angle with 2π or π by bits or by exact equality fails on every one of them.
- **Meaning.** The values are those of angles in radians, with the first below the second on every pair. That they are the start and end angle is what RvtIO reports from the Revit API, and nothing here compares them with a mesh: it is reported, not measured. RE-327 does not rebuild revolutions.

### 5.5 `PartAtom`

- The 12 corpus families (Revit 2016 to 2027) hold no string shaped like a GUID in their `PartAtom` (3,429 to 4,164 bytes), and none holds a shared-parameter definition: each has 6 `ParamElemFamily` and 36 `ParamElemElectricalLoadClassification` definitions and no `ParamElemExternal`. RvtIO's negative for its sample families holds on these, for the reason their finding file gives (section 4.3 of `finding-parameters-and-units.md`: its 24 families hold no `ParamElemExternal`): they use no shared parameter.
- The 2014 maker family of RE-327 (`stertil-closed-pit-sp-xp-pp-2014.rfa`, 112 types) holds 2,800 GUID-shaped strings in 425,286 bytes. Every one is the `id` attribute of a parameter element with `type="shared"`: 25 distinct GUIDs on 25 distinct element names, one name to each GUID, in each of the 112 types, and no other element carries an `id`. `rvt-info` lists the same 2,800 parameters as shared. So Discussion #112's shared-parameter GUIDs in `PartAtom` hold on a real family: `PartAtom` names a shared parameter and gives its GUID.
- Not shown: that these GUIDs equal the 16 bytes of the file's own `ParamElemExternal` (RvtIO's in-file chain), since a 2014 file is outside the native releases. That is the one step #155's criterion on shared-parameter names still lacks.

### 5.6 What it changes

Nothing in an export. A reader or probe that searches for a length by its bits has to try `mul`, `div`, `chain`, `metre` and `cmchain` at least, cannot take a miss as absence, and finds many stored values none of them reproduces. One that tests a revolution's angle compares with a tolerance, not with π or 2π by bits.

## 6. Not claimed

- That RvtIO or rvt-rs is right where they differ in section 4. No Revit export of the samples other than `rac_advanced`'s public IFC2X3 one exists in the corpus.
- Anything that rests on the API captures, or on models that are not in the folder: the unit factors beyond the specs rvt-rs converts, the host areas, the volumes. They are cited as leads and recorded as reported, not measured, and nothing that rests on NDA material is used. The values of the revolution ids are measured (5.4); their meaning as angles is not compared with a mesh.
- A name for any built-in parameter.
- Anything before Revit 2015, and the sample files of RvtIO's 2020 and 2025 sets.

## 7. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=5988d8398d17addba8bc67c4525bff69c0971c9f -f base=none \
  -f samples=true -f sample_years="2021 2027" -f scorers=none -f models="sample-*"
gh workflow run measure.yml --ref main -f ref=5988d8398d17addba8bc67c4525bff69c0971c9f -f base=none \
  -f racadv=true -f models="racadv-2020" -f scorers=class_sets_vs_ifc
gh workflow run measure.yml --ref main -f ref=27875749ca622bfd0f81d1ef2ba6e72d00f00dd2 -f base=none \
  -f samples=true -f sample_years="2015 2027" -f families=true -f nordic=true -f r2025=true \
  -f probe=probe_re344_parameter_ids_and_units -f scorers=none
```

The third command, with `-f ref=37f377d35b0e369b43f9ea182d836c9efd35ccc4`, is run 42039153578. Each model's `probe.txt` in the `measure-head` artifact ends in `tally` lines.
