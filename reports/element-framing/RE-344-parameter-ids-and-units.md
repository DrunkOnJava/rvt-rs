# RE-344: Parameter ids, stored lengths and a family's form extents, against RvtIO's findings

**Date:** 2026-11-05
**Issues:** #155 (the parameter store, its ids and units) and #156 (sketch to solid); Discussion #1241; follows RE-77, RE-93, RE-207, RE-310 and RE-327
**Credit:** mahach666, author of RvtIO (a .NET reader of Revit files, MIT), who published findings on both issues in Discussion #1241 on 2026-11-05: the record shapes of the parameter stores, the sign rule of parameter ids, the unit factors, the two millimetre conversions and the form parameters of extrusions and revolutions, with the count tables for Autodesk's samples that section 4 answers. This report restates facts with that credit and copies none of their tables or prose.
**Artefacts:** Autodesk's sample projects of Revit 2015 to 2027 as `research/autodesk-sample-projects.tsv` pins them, the `phi-ag/rvt` family corpus, the BIM4LCA office, Core Interior, the RE1 models and puzzbobb's R2025 models, as Measure fetches them; none is committed.
**Probe:** `probes/examples/probe_re344_parameter_ids_and_units.rs`
**Runs:** Measure 42028541274 (`main` at 5988d839, the samples of 2021 to 2027, section 4), 42031090233 (the same head, the class sets of `racadv-2020` against Revit's public IFC2X3 export, section 4.2), 42030865532 (the probe at 27875749, on `main` at 5988d839: `-f base=none -f samples=true -f sample_years="2015 2027" -f families=true -f nordic=true -f r2025=true -f scorers=none`, 105 models, section 5). The probe has not changed since 27875749.
**Status:** measured. Sections 4 and 5 name their runs. Nothing here changes an export: no value is added.

## 1. What the findings rest on, and what may be used

mahach666 states the sources of every finding: bytes of Revit files (including the class schema each file carries in `Formats/Latest`), the output of the public Revit API (Revit 2022) on Autodesk's sample projects and on their own models, and the IFC specification. They state that no program or library was disassembled or decompiled and no converter's output was used. Their findings carry their own marks: VERIFIED (compared with an independent value), OBSERVED (bytes only) and UNKNOWN.

`CLEANROOM.md` lists public byte observations, `RevitAPI.dll`'s exported symbols, previously published reverse-engineering work, buildingSMART's specifications and `revit-ifc` as accepted sources. The output of the Revit API run on a file is on neither list, accepted or forbidden. This report therefore splits every finding by what it rests on, and takes only the facts that can be stated and proved from bytes of files Measure fetches, against Revit's own IFC export or Revit's saved geometry where one exists. A value that only an API capture confirms is recorded as reported and not measured. The maintainer decided on 2026-11-05 that a third party's API capture is published reverse-engineering work (accepted source 3): it is cited and used to know where to look, and a value lands only when bytes of files CI can fetch, with Revit's own export or saved geometry, confirm it. `CLEANROOM.md` says so in one sentence.

Other handling:

- The folder carries no licence: no licence file, and no licence statement in its README. Facts (counts, layouts, ids) are restated here with credit; no table, prose or script is copied or committed. RvtIO's own repository is MIT.
- None of their scripts was run. Their parameter probe needs RvtIO's command line to dump the schema and the inflated partitions, and the others read the files themselves; this probe proves the same facts on Measure's runners with rvt-rs's own readers. Their finding files, count tables and outputs were read, and only the opening of one probe.
- Their copies of Autodesk's samples were not used (Measure fetches the pinned files from Autodesk's host) and their NDA models were never available.
- RvtIO's README says its built-in name tables are re-verified against copies of Revit's resource DLLs. No BuiltInParameter name is taken from RvtIO here: parameters are named by id in this report and in the probe, and the names rvt-rs already uses come from its own exports (RE-77, RE-93).

## 2. Which findings are new to rvt-rs

"Bytes" means the finding can be stated and proved from bytes of the files Measure fetches; "API" means their evidence is a capture of the Revit API, which is cited and recorded as reported, not confirmed.

| Finding (as RvtIO reports it) | Rests on | rvt-rs before | Here |
|---|---|---|---|
| Each element has four typed value sets (Double, Int, AString, ElementId) in the first four pointer fields of `Element` | bytes | known: RE-207 section 2, the native reader | not new |
| A family parameter is a slot of `FamilyParams.m_params` with one member per storage type, its `m_elemId` and its `m_paramId` | bytes | known: RE-207 section 2 and 7. RE-93's entry `f64 feet · ff×8 · i64 parameter` is one such slot (the value, an unset `m_elemId` of -1, the parameter): the identification is new | not tested by the probe; section 5.3 finds the same leading conversion on the slots as RE-310 on the type value blocks, with counts that cannot be compared |
| The id is negative for a built-in parameter and positive for the ElementId of a definition element (`ParamElem*`) of the same document | bytes | in the code (`ParameterKind`), never counted | measured, section 5.1: no exception in 1,545,257 ids |
| The Double item of the 2027 schema has the id before the value | bytes | read by the schema, so right by construction, not documented | section 5.2: the 2027 samples read as the 2026 ones do |
| The shared parameter's GUID is the 16 bytes of `ParamElemExternal.m_externalParamKey` | bytes, and the API for the captions | known (`Definition::shared_guid`, RE-207) | not new |
| `PartAtom` holds no GUID in the sample families | bytes | #155 recorded the claim as reported | section 5.5: none in the 12 corpus families, as they say; the `PartAtom` of a real family with shared parameters gives each its GUID |
| `ExtrusionElem` holds -1001800 and -1001801 | bytes; the API for which is the start | known: RE-327, proved against the saved meshes | not new |
| `RevolutionElem` holds -1001802 and -1001803, a full turn stored as 2 pi less seven units in the last place | bytes for the ids and the value; the API for "start angle" and "end angle" | not known | the ids and the value are measured in section 5.4, with partial revolutions they did not have; that they are the start and end angle is reported, not confirmed |
| `BlendElem` and `SweptBlendElem` hold -1001804 and -1001805 | bytes (unverified by them) | not known | measured in section 5.4: 1,229 of 1,229 `BlendElem` hold both, no `SweptBlendElem` holds either |
| Unit factors for specs rvt-rs does not convert (mass density, specific heat, thermal conductivity, resistivity, permeability, stress, unit weight, the scale factors): every dimensioned quantity is stored as its SI value times a power of 0.3048 | the API (the display text of `AsValueString`) | length, area, volume, angle, air flow, pressure, power, potential and current are converted, each measured against Revit's exports (RE-207, RE-210) | reported, not confirmed |
| `n / 25.4 / 12` and `n / 304.8` differ for 57,067 of the integers 1 to 100,000 (54,471 by one unit in the last place, 2,596 by two) | arithmetic | RE-310 measured that no one conversion reproduces the stored lengths, without this count | reproduced with an independent program, 42,933 / 54,471 / 2,596; the probe prints it from Rust's `f64` |
| No one conversion explains the stored values; the forms `n / 12 / 25.4` and `n / 1000 / 0.3048` exist | bytes | RE-310 (four forms, 2024 to 2027) | the same finding reached independently; section 5.3 adds the two forms and Revit 2015 to 2023, and the typed sets, where another conversion leads |
| The stored double equals the API's in 1713 of 1872 items | the API | not applicable | reported, not confirmed |
| The element-block walk closes exactly on 859,650 of 859,700 blocks of 19 project files | bytes | the native reader frames every group by its declared counts (RE-280) | a different method; no number to compare |
| Blocks outside the element table of a project (17 `ExtrusionElem`, 10 `SweepElem` in `rst_basic` 2022) were not examined | bytes | the records outside the element table are loaded families' own documents (RE-135, RE-163, RE-204; RE-327 counts 9,587 extrusions in them on the samples); that these 27 are such records is not itself measured | answers their open point in kind |
| Sketch curve classes, `GLine` and `GArc` fields, curves in model coordinates, loops by end points, holes by nesting parity, open chains left out | bytes | known: RE-327 sections 2 and 3, on 12 releases of the family corpus and 6,172 sample extrusions | not new |
| A floor's, ceiling's or roof footprint's area is the net area of its sketch's loops (98 of 105 hosts of four 2022 samples) | the API (the Area parameter) | not applicable: rvt-rs reads these from element records | reported, not confirmed |
| Revolution volume is the angle times the profile's first moment about the axis; a sweep's is the profile area times the path length | the API (volumes of their own models) | revolutions and sweeps not rebuilt (RE-327 section 8) | reported, not confirmed |
| The stored bounding box is bit-identical to the API's (1019 of 1019) | the API | known as bytes: RE-32 | not new |
| `Formats/Latest` grammar, tags counted from 12, class counts by release | bytes | known: `schema_registry`, #255 review | not new |
| Paging segments of 353 bytes every 64,896 bytes; sector size 4096 | bytes | known: `compression.rs` | not new |
| Each `Partitions` member is inflated with the previous member's last 32 KiB as a preset dictionary | bytes | not known: rvt-rs inflates every member on its own and checks each member's CRC-32 and size before it accepts a group (`native_segments`) | a question in the discussion reply, not a finding here |
| The riser height and tread depth of a stair run lie as a pair near its id | bytes, with weak attribution by their own account | not known as a pair | not used: their own attribution is weak |

What is new and can be proved from bytes is therefore small: the sign rule counted, the revolution and blend ids, the full-turn constant, two more conversions, the pre-2024 census and the `PartAtom` count. What is new and rests on the API (the unit factors beyond the converted specs, the angle meaning of the revolution ids, the host areas, the volume rules) is cited and not confirmed: each lands only when bytes and Revit's export or saved mesh confirm it.

## 3. The probe

`probe_re344_parameter_ids_and_units` takes one file and prints, per record:

- `arith`: the unit-in-the-last-place distance of `n / 25.4 / 12`, `n * (1 / 304.8)` and `n * (1 / 25.4) / 12` from `n / 304.8` over the integers 1 to 100,000.
- `sign` tallies, per store (`typed` for the four typed sets, `slot` for the slots of every `FamilyParams`): a negative id (and one that nevertheless has a definition), a positive id by the class of the definition it resolves to (`positive undefined` for one that resolves to none) and a zero id. A project's definitions are those the native reader resolves; a family file's are its `ParamElem*` records.
- `ulp` tallies of the stored doubles that are whole or tenth millimetres and not whole 1/256 inches (RE-310's selection), by store and kind (`builtin`, `declared`, `other`): how many agree under all six conversions, how many differ, and which conversions reproduce the stored bits.
- `form` tallies of a family's forms: which parameter ids each form class holds in each typed set, and for `ExtrusionElem`, `RevolutionElem`, `BlendElem`, `SweptBlendElem` and `SweepElem` the extents, with the revolution's start and end bits and whether the end is 2 pi, 2 pi less seven units in the last place, or something else.
- `partatom`: the stream's size and the count of strings shaped like a GUID.

A positive id with no definition, or a negative id with one, falsifies the sign rule; a sweep that holds an extent id, or a revolution that does not hold both angle ids, falsifies the form claims; a GUID-shaped string in a corpus family's `PartAtom` falsifies their negative for that file.

## 4. Counts on the same files

RvtIO's counts are the ones in the discussion's `counts.csv` for the 21 samples that are byte for byte the pinned files. rvt-rs's are `exported.building_elements` and `exported.by_ifc_type` of each sample's `diagnostics.json` in Measure run 42028541274 (`-f base=none -f samples=true -f sample_years="2021 2027" -f scorers=none -f models="sample-*"`, `main` at 5988d839). RvtIO's table quotes rvt-rs's reports (RE-140, RE-179 to RE-181) at an earlier head, so some of its rvt-rs figures are no longer the head's: rooms follow the export phase since 2026-11-04 (#1328), Areas are written as spaces (#1428), and storeys are bound to the elements they hold.

`building_elements` is the sum of `by_ifc_type`, `IfcSpace` included.

| sample | RvtIO | rvt-rs `main` | what accounts for the difference (section 4.1) |
|---|---:|---:|---|
| `rac_basic` 2022 | 455 | 450 | 5: 3 Furniture Systems, 1 Roof, 1 Mass |
| `rac_basic` 2023, 2024, 2026, 2027 | 461 | 450 | 11: those 5, 4 mullions, 2 curtain panels |
| `rst_basic` 2022 | 817 | 454 | 363: 346 reinforcing bars, 16 element assemblies, 1 space RvtIO writes and rvt-rs does not on a 2022 file |
| `rst_basic` 2024, 2026, 2027 | 817 | 455 | 362: 346 bars, 16 assemblies; both write the 1 space |
| `rac_advanced` 2021 | 5,878 | 5,657 | 221: 192 curtain panels, 25 Areas as spaces, 1 door, 1 element assembly, 2 Masses |
| `rac_advanced` 2022 | 5,685 | 5,682 | 3: 1 element assembly, 2 Masses |
| `rac_advanced` 2023, 2024, 2026, 2027 | 5,878 | 5,682 | 196: 192 curtain panels, 1 door, 1 element assembly, 2 Masses |

Each decomposition adds up to the difference: the sums are over the categories of RvtIO's `ifc_building_by_category` (its `(no Tag)` row is the spaces) set against the classes rvt-rs reads (`decoded.production_class_counts`).

### 4.1 What the class tables show

Their category counts (`ifc_building_by_category`) set against rvt-rs's `decoded.production_class_counts` and `by_ifc_type`, on the 2024 files and the 2022 `rac_basic`:

- **Reinforcing bars and framing systems (`rst_basic`, one assembly in `rac_advanced`).** The element-record sweep reads 42 product categories (`PRODUCT_RECORD_CATEGORIES`) and Rebar, Structural Framing Systems and Furniture Systems are not among them, so rvt-rs writes no `IfcReinforcingBar` and no `IfcElementAssembly`. With RvtIO's 346 bars and 16 assemblies taken away, the two agree on every other category of `rst_basic`: 370 beams, 30 columns, 36 foundations, 9 walls, 5 floors, 2 stringers, 1 stair, 1 run.
- **Masses.** rvt-rs reads the Mass elements (2 in `rac_advanced`, 1 in `rac_basic`) and does not write them: a mass is written only where its own "Export to IFC As" names an entity (#864, from the BIM4LCA office's masses). RvtIO writes each as an `IfcBuildingElementProxy`. This is a rule rvt-rs applies, not an element it fails to read.
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

The 34 curtain walls, 140 standard-case walls, 41 flow terminals and 104 furnishing elements of Revit's export are written as the class rvt-rs uses for each (`IfcWall` for the first two, `IfcLightFixture`, `IfcFurniture`), and the other 6 walls as `IfcWall`, 38 beams and 40 proxies and a slab are written that Revit's export does not hold, and Revit's export has no `IfcElementAssembly`. The 1,349 panels and 100 doors are what rvt-rs reads on every release and what RvtIO reads on its 2022 file; RvtIO's 2021 and 2023 to 2027 files give 1,541 and 101. The 2020 export cannot say whether those saves hold 192 panels and a door more, or whether RvtIO reads some where Revit's export of the earlier state has none: they are different files of the sample, and no Revit export of them exists.

## 5. Measured

Measure run 42030865532 ran the probe at 27875749 on 105 models: Autodesk's 58 sample projects of Revit 2015 to 2027, the 12 family-corpus files of 2016 to 2027 and the 2014 maker family of RE-327, the four BIM4LCA office files, Core Interior, Einhoven, the four RE1 models and the 24 R2025 models. The native reader read 1,284,631 element records of the 92 projects (4,818 had no saved metadata) and the 12 corpus families (10 had one record that did not decode, 2 had seven). The 2014 family is outside the native releases and was refused, as in RE-327. Counts of forms are form-and-file pairs: the same families recur in every release's files.

### 5.1 The sign rule

Of 1,545,257 parameter ids read from typed sets and family slots (903,777 negative, 641,480 positive), none broke the rule:

| store | negative | positive | a positive id resolves to |
|---|---:|---:|---|
| typed sets | 667,463 | 41,908 | `ParamElemExternal` 35,201, `ParamElemProject` 6,655, `ParamElemRefTableEntry` 52 |
| family slots | 236,314 | 599,572 | `ParamElemFamily` 569,353, `ParamElemExternal` 30,219 |

No positive id lacked a definition, no negative id had one, and no id was zero. Every positive id resolves to a `ParamElem*` element of its own document. RvtIO found positive ids in the typed sets of one sample project; here they are on all four storage types.

### 5.2 The 2027 Double item

The 2027 schema puts the id first in the Double item. The reader follows the schema, and the 2027 files give the counts the 2026 files give: typed Double items 3,202, 5,070 and 2,480 and stored millimetre candidates 733, 1,787 and 385 in `rac_basic`, `rac_advanced` and `rst_basic` of both releases.

### 5.3 The stored millimetre lengths

RE-310's selection (whole or tenth millimetres, not whole 1/256 inches), the six conversions of section 3, projects only. Of the values on which the six conversions give different bits, the share each conversion reproduces and the share none of the six does:

| store, kind, releases | values where the six differ | none | `mul` | `div` | `chain` | `metre` | only `metre` |
|---|---:|---:|---:|---:|---:|---:|---:|
| slots, built-in, 2015 to 2023 | 9,787 | 17% | 77% | 61% | 55% | 48% | 96 |
| slots, built-in, 2024 to 2027 | 16,629 | 29% | 67% | 51% | 55% | 37% | 88 |
| slots, declared, 2015 to 2023 | 49,294 | 7% | 91% | 71% | 74% | 53% | 391 |
| slots, declared, 2024 to 2027 | 136,356 | 5% | 93% | 73% | 83% | 40% | 902 |
| typed Double sets, built-in, 2015 to 2023 | 32,998 | 37% | 44% | 43% | 30% | 57% | 5,593 |
| typed Double sets, built-in, 2024 to 2027 | 39,619 | 37% | 44% | 43% | 30% | 58% | 6,475 |

- **The slots agree with RE-310.** `mul` (`mm * (1 / 304.8)`) reproduces the most on the slots in both halves, so on 2015 to 2023 as on 2024 to 2027. On the office as built it reproduces 90% of the differing slot values that some conversion reproduces (RE-310: 84% of its type value blocks'). The counts cannot be compared: RE-310 reads the type value blocks once per owner, parameter and value, and this run counts every record's slots (the office: 651 built-in and 4,084 declared values where the six differ, RE-310: 460 and 2,024).
- **The typed sets lead with another conversion.** `metre` (`mm / 1000 / 0.3048`) reproduces the most in the typed Double sets, 57% to 58% of the values where the six differ, and 5,593 and 6,475 that no other conversion does. `mul` is second at 44%. RE-310 did not read these sets.
- **No one conversion explains the stored values, in either store or half.** 37% of the typed values and 5% to 29% of the slots' match none of the six. Values only `chain` reproduces (247 declared slot values of 2024 to 2027, 81 typed) and values only `div` reproduces (70 and 148) exist beside each other, and `inv12` (`mm / 12 / 25.4`) alone reproduces 219 declared slot and 397 typed values. The finding that a search has to try several conversions holds on 2015 to 2023 as on 2024 to 2027.
- **The family files** add 520 built-in typed values (344 in 2016 to 2023, 176 in 2024 to 2027), 52% of which no conversion reproduces. The probe's sample lines show the extents of extrusions among them, one unit in the last place below `div`.

The probe selects numerically and does not know that a value was typed in millimetres: a value computed from others would be among them.

### 5.4 The parameters of a family's forms

| form | forms | holds | does not hold |
|---|---:|---|---|
| `ExtrusionElem` | 10,354 | -1001800 and -1001801: 10,354 (start zero on 3,318) | |
| `RevolutionElem` | 550 | -1001802 and -1001803: 550 | |
| `BlendElem` | 1,229 | -1001804 and -1001805: 1,229 | |
| `SweptBlendElem` | 322 | no Double set | both ids: 322 |
| `SweepElem` | 5,006 | | any of -1001800 to -1001805: 5,006 |
| `FreeFormElement` | 208 | | any of them: 208 |

The revolutions are not all full turns. Start is 0 on 545, π/2 on 2 and π on 3; end is π on 28, 3π/2 on 2 and the bits `401921fb54442d11` on 520. That value is 2π less seven units in the last place, and it is also the end of the three half turns that run from π to 2π: 517 revolutions are full turns (0 to 2π) and 33 are partial (28 from 0 to π, 2 from π/2 to 3π/2, 3 from π to 2π). The first id is below the second on all 550. RvtIO had no partial revolution. The values are those of angles in radians; that the two ids are the start and the end angle is what RvtIO reports from the API, and nothing here compares them with a mesh, so it is cited and not confirmed. RE-327 does not rebuild revolutions.

### 5.5 `PartAtom`

- The 12 corpus families (Revit 2016 to 2027) hold no string shaped like a GUID in their `PartAtom` (3,429 to 4,164 bytes), and none holds a shared-parameter definition: each has 6 `ParamElemFamily` and 36 `ParamElemElectricalLoadClassification` definitions and no `ParamElemExternal`. RvtIO's negative for its sample families holds on these, for the reason it gives: they use no shared parameter.
- The 2014 maker family of RE-327 (`stertil-closed-pit-sp-xp-pp-2014.rfa`, 112 types) holds 2,800 GUID-shaped strings in 425,286 bytes. Every one is the `id` attribute of a parameter element with `type="shared"`: 25 distinct GUIDs on 25 distinct element names, one name to each GUID, in each of the 112 types, and no other element carries an `id`. `rvt-info` lists the same 2,800 parameters as shared. So Discussion #112's shared-parameter GUIDs in `PartAtom` hold on a real family: `PartAtom` names a shared parameter and gives its GUID.
- Not shown: that these GUIDs equal the 16 bytes of the file's own `ParamElemExternal` (RvtIO's in-file chain), since a 2014 file is outside the native releases. That is the one step #155's criterion on shared-parameter names still lacks.

### 5.6 What it changes

Nothing in an export. The sign rule, the Double item order and the form ids are confirmed as readers already assume them. A reader or probe that searches for a length by its bits has to try `mul`, `div`, `chain` and `metre` at least on the typed sets, and must not take a miss as absence. Revolutions have a measured parameter shape, partial cases included, for a rebuild against the saved meshes as RE-327 did for extrusions.

## 6. Not claimed

- That RvtIO or rvt-rs is right where they differ in section 4. No Revit export of the samples other than `rac_advanced`'s public IFC2X3 one exists in the corpus.
- Anything that rests on the API captures: the unit factors beyond the specs rvt-rs converts, the host areas, the volumes. They are cited as leads and recorded as reported, not confirmed. The values of the revolution ids are measured (5.4); their meaning as angles is not compared with a mesh.
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

Each model's `probe.txt` in the `measure-head` artifact ends in `tally` lines.
