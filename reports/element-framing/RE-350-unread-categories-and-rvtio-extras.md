# RE-350: Furniture Systems are `IfcSystemFurnitureElement` in a Revit export; Rebar and Structural Beam Systems have no export that shows them; the 199 elements RvtIO counts beyond rvt-rs are family instances with no stored geometry that no Revit export of the project writes

**Date:** 2026-11-05
**Issues:** #1444 (reinforcing bars, element assemblies, Furniture Systems) and #1445 (192 curtain panels and a door on `rac_advanced`, 4 mullions and 2 panels on `rac_basic`); Discussion #1241; follows RE-33 (the category census), RE-315, RE-319, RE-326 and RE-344
**Credit:** mahach666, author of RvtIO (a .NET reader of Revit files, MIT), who reported the counts and listed the ElementIds of the 199 extra elements (Discussion #1241, 2026-11-05). Only ElementIds are taken from RvtIO, never its names or tables, and what the ids say is measured here on bytes of files CI can fetch and on Revit's own exports.
**Artefacts:** Autodesk's sample projects of Revit 2018 to 2027 (`research/autodesk-sample-projects.tsv`); 59 models with a Revit export in Measure (section 2); and the public IFC4 export of the 2018 `rac_basic` sample that RE-326 section 5 describes (a forum attachment, sha256 `1114ce918f64885bd87ec3bc670c4c543a90898d1bf7e701fb8fbf95e5b5ea35`, 9,129,271 bytes, no licence: fetched into an empty scratch folder, its hash checked, read locally for counts only and deleted; not in Measure and not committed). None of the samples is committed.
**Probe:** `probes/examples/probe_re350_category_stages.rs`, scored by `tools/re/category_census_vs_ifc.py`. At the commit of run A the probe was named `probe_re348_category_stages` (RE-348 was taken by another pull request while this one was open, so it was renamed before anything cited it); the runs below are evidence of the code at those commits.
**Runs:**
- A: Measure 42097940515, head `32586847` (`-f probe=probe_re348_category_stages -f probe_scorer=category_census_vs_ifc -f scorers=none -f samples=true -f sample_years="2020 2027" -f nordic=true -f r2025=true -f holdout=true -f bimcollab=true -f racadv=true`; 97 directories, every probe exit 0).
- B: Measure 42104379244, head `a78c61ea` (the probe with the declared and chained lists and a beam system's references; `-f samples=true -f sample_years="2020 2027" -f holdout=true -f models="spent-*"`).
- C: Measure 42108271073, head `41528e94` (the probe with the native read; `-f samples=true -f sample_years="2018 2023" -f models="-" -f scorers=none -f probe_scorer=-`).
- D: Measure 42109994761, head `282314ed` (the native read with the family symbol; the 2023 samples only). Its `symbol` lines say nothing (section 4.2).
Runs C and D were dispatched from commits on a side branch, which are in this pull request's history.
**Status:** measured. Nothing is written and `PRODUCT_RECORD_CATEGORIES` is unchanged. Furniture Systems now meet the rule that a category is added only where a Revit export holds it with matching ids and no false positive (section 3.4), and are held for a pull request of their own; Rebar and Structural Beam Systems do not. Nothing here rests on RvtIO's names. Only the five sources of `CLEANROOM.md` are used, and revit-ifc (source 5) is cited by file and class, nothing copied.

## 1. Questions and the rule

#1444: do Revit's own exports write Rebar, Structural Beam Systems (RvtIO's "Structural Framing Systems") and Furniture Systems, and does a rule reproduce their ids with no false positive? The RE-33 rule is the test: a decoded frame that is a standalone placed instance (no container reference, the placed kind), has a 3D body, is outside a non-primary design option and is in the export phase, each a `Tag` of Revit's export. A category is added only where an export holds it. #1445: which of the 199 extra elements are in the files, and are they elements Revit would export?

The categories are named by Autodesk's published `BuiltInCategory` values and by the name Revit's own export gives them (`BUILTIN_CATEGORY_NAMES`): Rebar -2009000 ("Structural Rebar"), Structural Beam Systems -2001327, Furniture Systems -2001100, Assemblies -2000267. That these are RvtIO's three categories is measured, not assumed: the counts agree to the element (section 3.1) and the one beam system of `rac_advanced` is record 201312, the id RvtIO gives its one assembly.

## 2. The instrument and the models

The probe prints, per category and per ElementId, the first test of the exporter that an id's frames fail: a container reference, the type symbol's placement kind, no 3D body (by how many of the three extents are flat), a non-primary design option, outside the export phase. `category_census_vs_ifc.py` scores each set against the `Tag`s of Revit's export and counts the export's elements by class whether or not their `Tag` is a number. The `written` column is the rule, a 3D body, the phase and the design option, not every filter the exporter applies (RE-319 §1), so it can hold ids `rvt-ifc` does not write.

Models with a Revit export in run A: 59 files. By export schema, 43 IFC4, 15 IFC2X3 and 1 IFC4X3. By group: Core Interior; the four RE1 models; the BIM4LCA office's four files; puzzbobb's 24 Revit 2025 models; BIMcollab's Revit 2015 structural example (IFC2X3 through Solibri's optimizer, counted apart); Autodesk's `rac_advanced` of 2015 to 2020 against the three.js IFC2X3 export (`racadv-*`, six files, no licence, counts only); 18 held-out models (counts only) and Vitoria's `ALD26 Before` (spent, CC BY 4.0, measured in full). The 2020 sample of `rac_advanced` is the file `racadv-2020` reads (the same URL and sha256 in the two pins). 37 Autodesk samples of 2020 to 2027 have no export. All 97 probes exited 0 with `census stages` lines. A sixtieth export, the public one of the 2018 `rac_basic`, is outside Measure and is read in section 3.4 and section 4.2.

The held-out, BIMcollab and racadv directories of the artifact hold only `counts.txt` and `ifc.sha256`. Every one of their 3,349 lines was checked against the keep-list's forms (a category id with the name of `BUILTIN_CATEGORY_NAMES` or `unnamed`, counts, IFC class names): none holds an ElementId, a project or family name or a name from a model.

## 3. What Revit's exports hold of the three categories

Of the 59 exports in Measure (run A, the `category census export` lines), and the public 2018 export read beside them:

| IFC class | exports that hold it | elements |
|---|---:|---|
| `IfcReinforcingBar`, `IfcReinforcingMesh`, `IfcTendon` | 0 of 60 | none |
| `IfcSystemFurnitureElement` | 1 of 60: the public IFC4 export of the 2018 `rac_basic` (section 3.4) | 3, `PredefinedType` unset |
| `IfcElementAssembly` | 2 (Vitoria's two Revit 2018 files) | 3 in each |
| `IfcFurnishingElement` (IFC2X3) | 8 | Furniture, Casework: 407 on one held-out file, 104 in each racadv save, 23 on RE1 Architecture |
| `IfcFurniture` (IFC4) | 13 | 4 to 98 per file |

No export holds a reinforcing bar. The only `IfcElementAssembly` in any export are Vitoria's 3. Read in the spent export itself (`ALD26 Before.ifc`, Revit's IFC2X3 export, CC BY 4.0), they are Revit assemblies, not beam systems: each is named `Montaje <family>:<type> <n>` (Spanish for "assembly"), has `PredefinedType` `USERDEFINED`, no representation, and aggregates its members through `IfcRelAggregates`: 7 `IfcColumn`, 7 `IfcColumn`, and 4 `IfcFlowSegment` with 3 `IfcFlowFitting`; their `Tag`s are 737286, 737381 and 770542. Run A finds no frame of any category for these three ids ("no frame 3" in the scorer's missed section), the 3 element assemblies RE-315 already counted among Vitoria's 7 missing elements. They are not in RvtIO's three categories. Run B finds each of the three declared in `Global/ElemTable` of the spent file, with a record in its partition chain and no frame of any category: elements the file holds with no stored model bounding box, the same carrier as the 199 extras of section 4. A reader of Revit assemblies (the record, its members' references) would be a new reader, not an entry of `PRODUCT_RECORD_CATEGORIES`, and is not attempted here.

### 3.1 Per category

Frames the sweep decodes (any category's frame is one record frame; "placed" is the RE-21 rule; "written" adds a 3D body, the phase and the design option):

| category | files with a frame (of 97) | in those, placed / written | with a Revit export |
|---|---|---|---|
| Rebar -2009000 | 9: `rst_basic` 2020 to 2027 (346 on each), Snowdon Towers Structural 2024 (14) | 346 / 346 on each `rst_basic`, 14 / 14 | 0 of 60 |
| Structural Beam Systems -2001327 | 23: `rac_advanced` 2020 to 2027 (1 on each), `rst_basic` 2020 to 2027 (16 on each), Snowdon Structural (76), and the six `racadv` files | 1 / 1; 16 / 15 (one has a flat box); 76 / 5 (71 flat) | 6 (the `racadv` files, one project: 2015 to 2020 saves) |
| Furniture Systems -2001100 | 9: `rac_basic` 2020 to 2027 (6 frames on each: 3 placed, 3 type symbols) and one held-out file (2 type symbols) | 3 / 3 on each `rac_basic`; 0 on the held-out file | 2 of 60: the held-out file holds no placed instance; the public 2018 export of `rac_basic` holds the 3 (section 3.4) |
| Assemblies -2000267 | 0 | none | none |

346 placed rebar frames on `rst_basic` equal RvtIO's 346 bars, 16 beam-system frames its 16 assemblies, 3 placed Furniture Systems frames its 3, and the same frames are on every release 2020 to 2027. The files hold these elements as frames with boxes, and the reader sees them. What decides each is whether a Revit export says what Revit writes for it.

- **Rebar.** No file with a Revit export holds a frame of it. 0 models exercise the rule.
- **Furniture Systems.** One of the 59 exports in Measure holds a frame, 2 type symbols and no placed instance (`unplaced symbol 2`), so the rule selects none there. The public 2018 export holds all 3 placed ones: section 3.4.
- **Structural Beam Systems.** One project has an export, the three.js IFC2X3 export of `rac_advanced`: each of its six saves of 2015 to 2020 holds one placed beam-system frame that the rule writes (on the 2020 to 2027 saves it is record 201312, a box 18.6 by 47.4 by 1.44 ft); its `Tag` is not in the export, and the export has no `IfcElementAssembly`. That is 1 id the rule would write that the export lacks, on each of 6 files of one project. It is no evidence against Revit writing a beam system: the same export holds none of the 38 placed `Structural Framing` instances the same files hold (0 of 38 are a `Tag`, on each of the 6 files, in a class list with no `IfcBeam`), so it leaves out the beams too. The beam system's record names 29 references (run B, the 2023 save), 23 of them placed framing frames, as many as the 23 `IfcBeam` members RvtIO reports for its one assembly (a count: RvtIO gave no member ids, so the sets are not compared), and all 23 are among the 38. With none of its beams exported, the revit-ifc path of section 3.2 writes no assembly, so the export's silence is what that code gives whether or not Revit writes beam systems. This export is IFC2X3 from a third-party build of Revit's exporter, counted apart in every measurement: it confirms a reader and decides nothing between exporters.

### 3.2 What the open-source exporter does (CLEANROOM source 5, read for behaviour, nothing copied)

Autodesk's revit-ifc, `Source/Revit.IFC.Export`:

- `Exporter/Exporter.cs`, `ExportElementImpl`: a `BeamSystem` is exported through `AssemblyInstanceExporter.ExportBeamSystem` the second time the exporter meets it, after the beams (`ExportBeamSystems` runs "after beams are exported"); rebar types (`AreaReinforcement`, `PathReinforcement`, `Rebar`, `RebarContainer`) go to `RebarExporter.Export`.
- `Exporter/AssemblyInstanceExporter.cs`, `ExportBeamSystem` and `ExportAssemblyInstanceWithMembers`: a beam system is written as an `IfcElementAssembly` of type `BEAM_GRID` that aggregates its members, and only when at least one member was exported (its handle is in the element-to-handle cache) and `IfcElementAssembly` is not in the export's exclude list. With no beam exported, no assembly is written.
- `Exporter/RebarExporter.cs`, `Export`: each rebar element is written as individual rebars, grouped where several are made, and only one of the elements created from a Revit element can carry that element's GUID. Whether each bar's `Tag` is the Revit element's ElementId is not read from the source, so even with an export the id test would need to settle it.
- Nothing in revit-ifc names `OST_FurnitureSystems`: the class of a Furniture Systems element comes from Revit's own category table, not in the source, as for the categories of RE-319 §3. The class is measured in section 3.4.

So Revit's exporter does have code for beam systems and rebar. The absence of an export holding one in the corpus is not evidence that Revit never writes them; and the one export that could show a beam system omits all 23 beams in its reference list, which is the condition under which this code writes nothing (section 3.1).

### 3.3 Result

A category is added only where an export holds it and the ids match with no false positive:

- Rebar: no export holds one (0 of 60). Not added.
- Structural Beam Systems: the one export of a project that has one omits it and the beams that belong to it. Not added.
- Furniture Systems: qualifies (section 3.4). Not added here, since it changes an export and the rule puts a new category in a pull request of its own.

`PRODUCT_RECORD_CATEGORIES` is unchanged (42 categories). Rebar and beam systems are held back for want of a Revit export that shows them, not for a defect in the rule. If the maintainer wants either without one, that is an exception for him to make, and what it would write is bounded here: 346 bars on each `rst_basic` save and 14 on Snowdon Towers Structural, and 15 of the 16 beam systems of `rst_basic` (and 1 of `rac_advanced`, 5 of Snowdon's 76), each with the record's box as body.

### 3.4 Furniture Systems against Revit's own export of the 2018 `rac_basic`

The public IFC4 export of the 2018 `rac_basic` sample (RE-326 §5, no licence: read locally for counts only; no tag list and no name from it is quoted or committed) was read beside run C's output on Autodesk's 2018 sample (`sample-2018-rac_basic_sample_project`). It holds 438 elements with a `Tag` (RE-326's 438), and the sweep's written ids cover every one of them: 510 ids written, 438 of them `Tag`s of the export and 72 not, and 0 `Tag`s of the export outside the written ids. For Furniture Systems:

- The export has 3 `IfcSystemFurnitureElement` (IFC4, `PredefinedType` unset on each) and 3 `IfcSystemFurnitureElementType` objects.
- The 2018 save has 6 Furniture Systems frames: 3 placed, written, with a body, and 3 type symbols. The placed ones are 506168, 523531 and 697577 and the symbols 506166, 523529 and 697575, as the sweep reads them from the `.rvt`. All 3 placed frames are in the export as `IfcSystemFurnitureElement` (3 of 3) and its 3 type objects are the 3 symbols (3 of 3), counts of ids the sweep reads, with 0 written that the export lacks. The same 3 and 3 ids are in the 2018 to 2023 saves (runs C and D), and the same counts on 2024 to 2027 (run A).

That is one project, one export and 3 elements, and no other model in Measure places a Furniture System, so a held-out check of the rule is empty. The class Revit writes is `IfcSystemFurnitureElement` with no `PredefinedType`; `category_map` maps `FurnitureSystem` to `IfcFurniture` with `USERDEFINED`, a mapping nothing measured. The same pairing checks the categories this probe also saw there: all 144 mullions, 16 doors, 17 windows and every other written category are `Tag`s of the export, except 72 of the 510 written ids: 6 of the 44 curtain panels, 1 wall, 11 of the 16 planting, and 54 in categories of which the export holds nothing (Mass, wall openings and unnamed categories). They are the rule's over-selection that RE-319 §1 describes, not measured against `rvt-ifc`'s own output here.

## 4. #1445: the sweep reads the same frames on every release

The frames of the three categories of #1445 are the same on every save of Autodesk's samples, 2020 to 2027 (run A):

| sample | curtain panels: frames / placed / written | mullions | doors |
|---|---|---|---|
| `rac_advanced` 2020 to 2027 (8 saves) | 1,508 / 1,349 / 1,349 (159 type symbols) | 3,276 placed and written (345 symbols) | 100 placed and written (64 symbols) |
| `rac_basic` 2020 to 2027 (8 saves) | 66 / 44 / 44 (22 symbols) | 144 placed and written (63 symbols) | 16 placed and written (15 symbols) |

Every placed frame is written: none is left out for a missing body, a design option or the phase, on any of the 16 saves. RvtIO's counts on its 2023 and later files are 1,541 panels and 101 doors on `rac_advanced`, 148 mullions and 46 panels on `rac_basic`; rvt-rs's placed counts are 192 panels and 1 door, and 4 mullions and 2 panels fewer, on every save. Against Revit's own export of the 2015 to 2020 state, the 2020 save's 1,349 placed panels and 100 doors are all `Tag`s (the scorer's line for the `racadv` files, 0 written ids that the export lacks).

### 4.1 The 199 extras are in the files, with no stored geometry

The ids of the extras are those mahach666 attached to Discussion #1241 (the tags in RvtIO's 2023 IFC and not in its 2022 IFC). Runs B and C print, for each save, the ElementIds `Global/ElemTable` declares, those a decoded frame of any category carries, and the records the partitions open with (the chain of RE-35; the declared records of RE-315 on the 32-bit releases) that no frame carries. On every one of the ten saves of each sample, 2018 to 2027 (runs B and C):

| | `rac_advanced` (193 ids) | `rac_basic` (6 ids) |
|---|---:|---:|
| declared in `Global/ElemTable` | 193 | 6 |
| a record in the partition chain | 193 | 6 |
| a frame of any category (the frame carries the stored bounding box and the category) | 0 | 0 |

So the file holds an element record for each id on every save, the 2018 to 2022 saves included (RvtIO's 2022 IFC has none of them), and none has a bounding-box frame the sweep can read. That alone does not set them apart: on the 2023 `rac_advanced`, of 17,108 declared ids 8,896 have a frame and the other 8,212 have a record and none, as types, parameters and every other element without stored geometry do.

### 4.2 What each one is

Run C reads the native record of every id that has a record and no frame, and of every frame of the three categories, in one pass of `native_document::extract_graphs` that decodes no graph. All 199 are class `FamilyInstance`, and the Element base of every one reads the same as the placed panels, mullions and doors the sweep writes: not moribund, not a dummy, no unplaced owner, no owner view, no design option, no demolished phase, created in phase 86961. The placed panels, mullions and doors are the same class with the same flags (`rac_advanced` 2023: 1,349, 3,276 and 100 `FamilyInstance`; their 159, 345 and 64 type symbols are `SysPanelFamSym`, `SysMullionFamSym` and `FamilySymbol`). The Element base names no family for any of the 199 (`m_famId` is -1, run D), and with no frame there is no reference list to name a symbol, so what their family type and category are is not read from the file: the classes RvtIO gives them (below) are its own and not confirmed here.

By the class RvtIO gives each, with its ElementId (Autodesk's sample project ids, from RvtIO's list of the tags in its 2023 IFC and not its 2022 IFC):

| sample | RvtIO's class | n | ElementIds |
|---|---|---:|---|
| `rac_advanced` | `IfcPlate` | 192 | 149013, 182705-182709, 182775, 182777-182779, 182821-182822, 183039-183063, 183173-183175, 183283-183293, 183331, 183336, 183341, 183454-183455, 183573-183581, 183723-183735, 183953-183973, 184063-184064, 184085-184087, 184156-184161, 184222-184225, 184256, 184277-184278, 184302-184303, 188993, 194968, 195036, 195083-195086, 195088-195091, 196237-196240, 196243-196244, 196407-196409, 196469-196471, 196738-196739, 196807-196810, 196813-196814, 196828-196830, 196839-196841, 198055-198064, 198149-198158, 198390-198391, 198393-198395, 216750-216754, 216846, 217077-217079, 217292-217293 |
| `rac_advanced` | `IfcDoor` | 1 | 216845 |
| `rac_basic` | `IfcMember` | 4 | 976937-976938, 976953, 978558 |
| `rac_basic` | `IfcPlate` | 2 | 976935, 978551 |

Whether Revit exports them:

- **`rac_basic`, against an IFC4 export.** The public 2018 export (section 3.4) holds none of the 6 ids, as an occurrence, as a type or anywhere else in the file (a count of 0 of 6). The 2018 save declares all 6, with records and no frame (run C), so the elements the 2023 file holds are in the 2018 save too, and Revit's own export of it does not write them. The 2018 export holds the 144 mullions and 38 of the 44 panels the sweep writes, and 438 of 438 elements are among the sweep's written ids.
- **`rac_advanced`, against an IFC2X3 export (counted apart: no licence, counts only).** The 2020 save is the file `racadv-2020` reads, and it declares all 193 ids and holds their records. The three.js export of the 2015 to 2020 state holds 1,349 `IfcPlate` and 100 `IfcDoor`, every one a `Tag` of a frame the sweep decodes and writes (1,349 of 1,349 panels, 100 of 100 doors, 3,276 of 3,276 mullions), and no `Tag` among its 5,439 distinct numeric ones (5,522 elements, 83 of which repeat another's `Tag`) lies outside the frames the sweep writes: the scorer's "export elements not written" section is empty for each of the six `racadv` files. None of the 193 is written. The 2018 and 2019 saves hold the same 193 records.
- The later saves (2021 and 2023 to 2027) have no Revit export. They hold the same records, with the same carrier, as the 2018 to 2020 saves that do, so nothing in them separates the 199 from the elements those exports leave out; that the later exports would leave them out too is an inference from the same records, not a measurement.

Classification: each of the 199 is a `FamilyInstance` of the main model, placed in a project phase, with no stored bounding box, that no Revit export of its project writes (0 of 6 in the IFC4 export, 0 of 193 in the IFC2X3 one). They are not elements Revit's export writes that rvt-rs misses. That agrees with RvtIO's report that all 199 have a placement and no representation in its own IFC. Their family types and categories are undecided: no carrier the probe reads names them, so the `IfcPlate`, `IfcDoor` and `IfcMember` of the table are RvtIO's and are not used. rvt-rs writes nothing for them. A rule that wrote them would rest on RvtIO's classes, no box and no Revit export, which is a lead only (the maintainer's decision of 2026-11-05).

## 5. What this does not claim

- That Revit writes none of Rebar and Structural Beam Systems. Revit's exporter has code for both (section 3.2), and no export measured holds a bar or a beam system it should have written.
- That the beam system is not an `IfcElementAssembly` in Revit's export. The one export of its project omits the whole `Structural Framing` category as well.
- What family type or category any of the 199 is, or that Revit would not export them from a save that has no export (section 4.2).
- That Furniture Systems are correct beyond 3 elements of one project: the class and the ids match on that export, and nothing else in the corpus places one. The 2018 export is a forum attachment with no licence, used for counts only.
- Anything about files that are not measured. The `written` column over-selects where the exporter filters further: on the BIM4LCA office's two files it holds 2,685 curtain panels each that are not `Tag`s, and on the 2018 `rac_basic` 6 of 44; `rvt-ifc`'s own output is not measured here (RE-319 §1).

## 6. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=41528e94ceb7f33c0b12d052a5fe7b71325da905 -f base=none \
  -f probe=probe_re350_category_stages -f probe_scorer=- -f scorers=none \
  -f samples=true -f sample_years="2018 2023" -f models="-"
```

The probe's `native` lines (run C) and its `declared`, `chained` and `record` lines are in that run's `measure-head-all` artifact for the Autodesk samples. The counts for the 59 models with an export (`-f nordic=true -f r2025=true -f holdout=true -f bimcollab=true -f racadv=true -f probe_scorer=category_census_vs_ifc`) are in run A's `measure-head` artifact; the held-out, BIMcollab and racadv directories there hold counts only. Run A was dispatched from the branch whose `scrub-holdout-results.sh` keeps the `census stages` and `category census` count lines, and used the probe under its earlier name at `32586847`. The 2018 export is fetched from the SourceForge attachment of RE-326 §5 (its hash is above) and paired with `probe.txt` of `sample-2018-rac_basic_sample_project`; the pairing is the scorer's, run on the file's text, and only its counts are quoted here.
