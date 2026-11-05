# RE-350: Rebar, Structural Beam Systems and Furniture Systems are read from the files and written by no Revit export we can check; the 199 elements RvtIO counts beyond rvt-rs have no frame in any category

**Date:** 2026-11-05
**Issues:** #1444 (reinforcing bars, element assemblies, Furniture Systems) and #1445 (192 curtain panels and a door on `rac_advanced`, 4 mullions and 2 panels on `rac_basic`); Discussion #1241; follows RE-33 (the category census), RE-315 §9, RE-319 and RE-344
**Credit:** mahach666, author of RvtIO (a .NET reader of Revit files, MIT), who reported the counts and listed the ElementIds of the 199 extra elements (Discussion #1241, 2026-11-05). Only ElementIds are taken from RvtIO, never its names or tables, and what the ids say is measured here on bytes of files CI can fetch and on Revit's own exports.
**Artefacts:** Autodesk's sample projects of Revit 2020 to 2027 (`research/autodesk-sample-projects.tsv`), 59 models with a Revit export (section 2), none committed.
**Probe:** `probes/examples/probe_re350_category_stages.rs`, scored by `tools/re/category_census_vs_ifc.py`. At the commit of run A the probe was named `probe_re348_category_stages` (the number RE-348 was taken by another pull request while this one was open, so it was renamed before anything cited it); the runs below are evidence of the code at those commits.
**Runs:** A: Measure 42097940515, head `32586847` (`-f probe=probe_re348_category_stages -f probe_scorer=category_census_vs_ifc -f scorers=none -f samples=true -f sample_years="2020 2027" -f nordic=true -f r2025=true -f holdout=true -f bimcollab=true -f racadv=true`; 97 directories, every probe exit 0). B: Measure 42104379244, head `a78c61ea` (the probe with the lists of section 4.1 and a beam system's references; `-f samples=true -f sample_years="2020 2027" -f holdout=true -f models="spent-*"`).
**Status:** measured. No export changes: no category is added and no value is written. Nothing here rests on RvtIO's names, on the Building Coder or on the Revit API Developer's Guide.

## 1. Questions and the rule

#1444: do Revit's own exports write Rebar, Structural Beam Systems (RvtIO's "Structural Framing Systems") and Furniture Systems, and does a rule reproduce their ids with no false positive? The RE-33 rule is the test: a decoded frame that is a standalone placed instance (no container reference, the placed kind), has a 3D body, is outside a non-primary design option and is in the export phase, each a `Tag` of Revit's export. A category is added only where an export holds it. #1445: which of the 199 extra elements are in the files, and are they elements Revit would export?

The categories are named by Autodesk's published `BuiltInCategory` values and by the name Revit's own export gives them (`BUILTIN_CATEGORY_NAMES`): Rebar -2009000 ("Structural Rebar"), Structural Beam Systems -2001327, Furniture Systems -2001100, Assemblies -2000267. That these are RvtIO's three categories is measured, not assumed: the counts agree to the element (section 4) and the one beam system of `rac_advanced` is record 201312, the id RvtIO gives its one assembly.

## 2. The instrument and the models

The probe prints, per category and per ElementId, the first test of the exporter that an id's frames fail: a container reference, the type symbol's placement kind, no 3D body (by how many of the three extents are flat), a non-primary design option, outside the export phase. `category_census_vs_ifc.py` scores each set against the `Tag`s of Revit's export and counts the export's elements by class whether or not their `Tag` is a number. The `written` column is the rule, a 3D body, the phase and the design option, not every filter the exporter applies (RE-319 §1), so it can hold ids `rvt-ifc` does not write.

Models with a Revit export in run A: 59 files. By export schema, 43 IFC4, 15 IFC2X3 and 1 IFC4X3. By group: Core Interior; the four RE1 models; the BIM4LCA office's four files; puzzbobb's 24 Revit 2025 models; BIMcollab's Revit 2015 structural example (IFC2X3 through Solibri's optimizer, counted apart); Autodesk's `rac_advanced` of 2015 to 2020 against the three.js IFC2X3 export (`racadv-*`, six files, no licence, counts only); 18 held-out models (counts only) and Vitoria's `ALD26 Before` (spent, CC BY 4.0, measured in full). The 2020 sample of `rac_advanced` is the file `racadv-2020` reads (the same URL and sha256 in the two pins). 37 Autodesk samples of 2020 to 2027 have no export. All 97 probes exited 0 with `census stages` lines.

The held-out, BIMcollab and racadv directories of the artifact hold only `counts.txt` and `ifc.sha256`. Every one of their 3,349 lines was checked against the keep-list's forms (a category id with the name of `BUILTIN_CATEGORY_NAMES` or `unnamed`, counts, IFC class names): none holds an ElementId, a project or family name or a name from a model.

## 3. What Revit's exports hold of the three categories

Of the 59 exports (run A, the `category census export` lines):

| IFC class | exports that hold it | elements |
|---|---:|---|
| `IfcReinforcingBar`, `IfcReinforcingMesh`, `IfcTendon` | 0 | none |
| `IfcSystemFurnitureElement` | 0 | none |
| `IfcElementAssembly` | 2 (Vitoria's two Revit 2018 files) | 3 in each |
| `IfcFurnishingElement` (IFC2X3) | 8 | Furniture, Casework: 407 on one held-out file, 104 in each racadv save, 23 on RE1 Architecture |
| `IfcFurniture` (IFC4) | 13 | 4 to 98 per file |

No export holds a reinforcing bar. The only `IfcElementAssembly` in any export are Vitoria's 3. Read in the spent export itself (`ALD26 Before.ifc`, Revit's IFC2X3 export, CC BY 4.0), they are Revit assemblies, not beam systems: each is named `Montaje <family>:<type> <n>` (Spanish for "assembly"), has `PredefinedType` `USERDEFINED`, no representation, and aggregates its members through `IfcRelAggregates`: 7 `IfcColumn`, 7 `IfcColumn`, and 4 `IfcFlowSegment` with 3 `IfcFlowFitting`; their `Tag`s are 737286, 737381 and 770542. Run A finds no frame of any category for these three ids ("no frame 3" in the scorer's missed section), the 3 element assemblies RE-315 already counted among Vitoria's 7 missing elements. They are not in RvtIO's three categories. Run B finds each of the three declared in `Global/ElemTable` of the spent file, with a record in its partition chain and no frame of any category: elements the file holds with no stored model bounding box, the same carrier as the 199 extras of section 4.1. A reader of Revit assemblies (the record, its members' references) would be a new reader, not an entry of `PRODUCT_RECORD_CATEGORIES`, and is not attempted here.

### 3.1 Per category

Frames the sweep decodes (any category's frame is one record frame; "placed" is the RE-21 rule; "written" adds a 3D body, the phase and the design option):

| category | files with a frame (of 97) | in those, placed / written | with a Revit export |
|---|---|---|---|
| Rebar -2009000 | 9: `rst_basic` 2020 to 2027 (346 on each), Snowdon Towers Structural 2024 (14) | 346 / 346 on each `rst_basic`, 14 / 14 | 0 of 59 |
| Structural Beam Systems -2001327 | 23: `rac_advanced` 2020 to 2027 (1 on each), `rst_basic` 2020 to 2027 (16 on each), Snowdon Structural (76), and the six `racadv` files | 1 / 1; 16 / 15 (one has a flat box); 76 / 5 (71 flat) | 6 (the `racadv` files, one project: 2015 to 2020 saves) |
| Furniture Systems -2001100 | 9: `rac_basic` 2020 to 2027 (6 frames on each: 3 placed, 3 type symbols) and one held-out file (2 type symbols) | 3 / 3 on each `rac_basic`; 0 on the held-out file | 1 (the held-out file), holding no placed instance |
| Assemblies -2000267 | 0 | none | none |

346 placed rebar frames on `rst_basic` equal RvtIO's 346 bars, 16 beam-system frames its 16 assemblies, 3 placed Furniture Systems frames its 3, and the same frames are on every release 2020 to 2027. The files do hold these elements as frames with boxes, and the reader sees them. What is missing is a Revit export that says what Revit writes for them.

- **Rebar.** No file with a Revit export holds a frame of it. 0 models exercise the rule.
- **Furniture Systems.** The one file with an export that holds a frame holds 2 type symbols and no placed instance (`unplaced symbol 2`), so the rule selects none there. 0 models exercise it.
- **Structural Beam Systems.** One project has an export, the three.js IFC2X3 export of `rac_advanced`: each of its six saves of 2015 to 2020 holds one placed beam-system frame that the rule writes (on the 2020 to 2027 saves it is record 201312, a box 18.6 by 47.4 by 1.44 ft); its `Tag` is not in the export, and the export has no `IfcElementAssembly`. That is 1 id the rule would write that the export lacks, on each of 6 files of one project. It is no evidence against Revit writing a beam system: the same export holds none of the 38 placed `Structural Framing` instances the same files hold (0 of 38 are a `Tag`, on each of the 6 files, in a class list with no `IfcBeam`), so it leaves out the beams too. The beam system's record names 29 references (run B, the 2023 save), 23 of them placed framing frames, as many as the 23 `IfcBeam` members RvtIO reports for its one assembly (a count: RvtIO gave no member ids, so the sets are not compared), and all 23 are among the 38. With none of its beams exported, the revit-ifc path of section 3.2 writes no assembly, so the export's silence is what that code gives whether or not Revit writes beam systems. This export is IFC2X3 from a third-party build of Revit's exporter, counted apart in every measurement: it confirms a reader and decides nothing between exporters.

### 3.2 What the open-source exporter does (CLEANROOM source 5, read for behaviour, nothing copied)

Autodesk's revit-ifc, `Source/Revit.IFC.Export`:

- `Exporter/Exporter.cs`, `ExportElementImpl`: a `BeamSystem` is exported through `AssemblyInstanceExporter.ExportBeamSystem` the second time the exporter meets it, after the beams (`ExportBeamSystems` runs "after beams are exported"); rebar types (`AreaReinforcement`, `PathReinforcement`, `Rebar`, `RebarContainer`) go to `RebarExporter.Export`.
- `Exporter/AssemblyInstanceExporter.cs`, `ExportBeamSystem` and `ExportAssemblyInstanceWithMembers`: a beam system is written as an `IfcElementAssembly` of type `BEAM_GRID` that aggregates its members, and only when at least one member was exported (its handle is in the element-to-handle cache) and `IfcElementAssembly` is not in the export's exclude list. With no beam exported, no assembly is written.
- `Exporter/RebarExporter.cs`, `Export`: each rebar element is written as individual rebars, grouped where several are made, and only one of the elements created from a Revit element can carry that element's GUID. Whether each bar's `Tag` is the Revit element's ElementId is not read from the source, so even with an export the id test would need to settle it.
- Nothing in revit-ifc names `OST_FurnitureSystems`: the class of a Furniture Systems element comes from Revit's own category table, not in the source, as for the categories of RE-319 §3.

So Revit's exporter does have code for beam systems and rebar. The absence of an export holding one in the corpus is not evidence that Revit never writes them; and the one export that could show a beam system omits all 23 beams in its reference list, which is the condition under which this code writes nothing (section 3.1).

### 3.3 Result

A category is added only where an export holds it and the ids match with no false positive. None qualifies:

- Rebar: no export holds one (0 of 59). Not added.
- Furniture Systems: no export holds a placed instance (0 of 59). Not added.
- Structural Beam Systems: the one export of a project that has one omits it and the beams that belong to it. Not added.

Nothing is written; `PRODUCT_RECORD_CATEGORIES` is unchanged (42 categories). Each is held back for want of a Revit export, not for a defect in the rule: if the maintainer wants any of the three without one, that is an exception for him to make, and what it would write is bounded here: 346 bars on each `rst_basic` save and 14 on Snowdon Towers Structural, 15 of the 16 beam systems of `rst_basic` (and 1 of `rac_advanced`, 5 of Snowdon's 76) and the 3 Furniture Systems of `rac_basic`, each with the record's box as body.

## 4. #1445: the sweep reads the same frames on every release

The frames of the three categories of #1445 are the same on every save of Autodesk's samples, 2020 to 2027 (run A):

| sample | curtain panels: frames / placed / written | mullions | doors |
|---|---|---|---|
| `rac_advanced` 2020 to 2027 (8 saves) | 1,508 / 1,349 / 1,349 (159 type symbols) | 3,276 placed and written (345 symbols) | 100 placed and written (64 symbols) |
| `rac_basic` 2020 to 2027 (8 saves) | 66 / 44 / 44 (22 symbols) | 144 placed and written (63 symbols) | 16 placed and written (15 symbols) |

Every placed frame is written: none is left out for a missing body, a design option or the phase, on any of the 16 saves. RvtIO's counts on its 2023 and later files are 1,541 panels and 101 doors on `rac_advanced`, 148 mullions and 46 panels on `rac_basic`; rvt-rs's placed counts are 192 panels and 1 door, and 4 mullions and 2 panels fewer, on every save. Against Revit's own export of the 2015 to 2020 state, the 2020 save's 1,349 placed panels and 100 doors are all `Tag`s (the scorer's line for the `racadv` files, 0 written ids that the export lacks).

### 4.1 The 199 extras (run B)

For each of the eight saves of each sample, run B prints the ElementIds `Global/ElemTable` declares, those a decoded frame of any category carries, and the records the partitions open with (the chain of RE-35; the declared records of RE-315 on the 32-bit releases) that no frame carries. The ids of the extras are those mahach666 attached to Discussion #1241 (the tags in RvtIO's 2023 IFC and not in its 2022 IFC): 192 `IfcPlate` and 1 `IfcDoor` on `rac_advanced`, 4 `IfcMember` and 2 `IfcPlate` on `rac_basic`. They are used as ids and not committed. On every one of the 16 saves, 2020 to 2027:

| | `rac_advanced` (193 ids) | `rac_basic` (6 ids) |
|---|---:|---:|
| declared in `Global/ElemTable` | 193 | 6 |
| a record in the partition chain | 193 | 6 |
| a frame of any category (the frame carries the stored bounding box and the category) | 0 | 0 |

So the file holds an element record for each id on every save, the 2020 to 2022 saves included (RvtIO's 2022 IFC has none of them), and none has a bounding-box frame the sweep can read. That alone does not set them apart: on the 2023 `rac_advanced`, of 17,108 declared ids 8,896 have a frame and the other 8,212 have a record and none, as types, parameters and every other element without stored geometry do. It agrees with what RvtIO reported of its own IFC, that all 199 have a placement and no representation: elements the file holds, with no stored model box.

Revit's own export of the 2015 to 2020 state, the three.js IFC2X3 export of `rac_advanced` (counted apart: IFC2X3, no licence, counts only), is the one check. The 2020 save is the file `racadv-2020` reads, and it declares all 193 ids and holds their records. The export holds 1,349 `IfcPlate` and 100 `IfcDoor`, every one a `Tag` of a frame the sweep decodes and writes (1,349 of 1,349 panels, 100 of 100 doors, 3,276 of 3,276 mullions), and no `Tag` among its 5,439 distinct numeric ones (5,522 elements, 83 of which repeat another's `Tag`) lies outside the frames the sweep writes: the scorer's "export elements not written" section is empty for each of the six `racadv` files. So in the one Revit export of this project none of the 192 panels and the door is written, though their records are in the file. It cannot say what Revit's exporter does with the 2021 and 2023 to 2027 saves, which have no export, and it decides nothing between exporters. For `rac_basic` there is no Revit export of a 2020 to 2027 save; the public IFC4 export of its 2018 save (RE-326 §5, no licence, local use only) was not read for these ids.

Classification, with that limit:

- Not decoded frames of curtain panels, mullions or doors: none has a frame, so the sweep cannot tell their category, and none of the 199 is among the 1,349 + 3,276 + 100 and 44 + 144 + 16 placed frames of section 4.
- Elements the file holds with no stored model bounding box. Whether Revit's export writes them in the 2021 to 2027 saves is not shown. In the one Revit export of the project (2015 to 2020) none of the 193 is written.
- rvt-rs does not read them and writes nothing for them, and nothing here shows it should: a rule that wrote them would rest on RvtIO's report that they are `IfcPlate`, `IfcDoor` and `IfcMember`, and on no box and no Revit export. That is a lead only (the maintainer's decision of 2026-11-05).

What would settle their class is the native record of each id (`native_document` reads a record by member name), which this probe does not read.

## 5. What this does not claim

- That Revit writes none of the three categories. Revit's exporter has code for beam systems and rebar (section 3.2), and no export in the corpus holds a bar or a placed Furniture System.
- That the beam system is not an `IfcElementAssembly` in Revit's export. The one export of its project omits the whole `Structural Framing` category as well.
- That the 199 elements are or are not elements Revit would export: no Revit export of the 2021 or the 2023 to 2027 saves exists (section 4.1).
- Anything about files that are not in the 97 measured. The `written` column over-selects where the exporter filters further: on the BIM4LCA office's two files it holds 2,685 curtain panels each that are not `Tag`s (RE-319 §1); `rvt-ifc`'s own output is not measured here.

## 6. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=a78c61eade5fcd404a0b7c9640f49c54e012a107 -f base=none \
  -f probe=probe_re350_category_stages -f probe_scorer=category_census_vs_ifc -f scorers=none \
  -f samples=true -f sample_years="2020 2027" -f holdout=true -f models="spent-*"
```

The counts for the 59 models with an export (`-f nordic=true -f r2025=true -f holdout=true -f bimcollab=true -f racadv=true`) and the 37 samples are in run A's `measure-head` artifact; the held-out, BIMcollab and racadv directories there hold counts only. Run A was dispatched from the branch whose `scrub-holdout-results.sh` keeps the `census stages` and `category census` count lines, and used the probe under its earlier name at `32586847`.
