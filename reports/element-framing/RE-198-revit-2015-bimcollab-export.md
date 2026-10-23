# RE-198: Revit 2015 against Revit 2015's own export, on BIMcollab's structural example

**Date:** 2026-10-21
**Issues:** #421 (element records beyond 2024/2025), #408 (held-out validation); follows RE-181 (2015 to 2018 read against their 2019 copies)
**Artefacts:** `02_BIMcollab_Example_STR.rvt`, the structural model of BIMcollab's example project (KUBUS BV). It is a Revit 2015 project: BasicFileInfo reads Autodesk Revit Architecture 2015, build 20141119_0715(x64). 13,979,648 bytes, sha256 `8cd1f122e751c33ea69e6f16c23ed4cd572ded3d0d04dfab50aaf0973e38e193`, the hash STE1200 gave (Discussion #112). The reference export is `02_BIMcollab_Example_STR_optimized.ifc`, 455,050 bytes, sha256 `1d0d047a5519a2ea143b9d07625dcc94bd3a1c3b01cb86607b16d850f0a5081d`: IFC2X3, CoordinationView 2.0 and QuantityTakeOffAddOnView. Its `IfcApplication` is "Autodesk Revit Architecture 2015 (ENU)", version 2015, and its owner history is dated 2014-10-08. Solibri's IFC Optimizer later rewrote it (its `FILE_NAME`), keeping the entities, GlobalIds and Tags. Both are public downloads linked from BIMcollab's help center, with no licence stated, so they are local only and never committed. A Measure run fetches them by hash (`research/bimcollab-models.tsv`, `tools/ci/fetch-bimcollab.sh`, `-f bimcollab=true`).
**Run:** Measure [39580240447](https://github.com/DrunkOnJava/rvt-rs/actions/runs/39580240447), `main` at 2dbc8b8b, scorers from `re/bimcollab-2015-pair` (1f3dc328).
**Result:** the first Revit export of a 2015 to 2022 file scored here. It is mixed, and held out: measured once, before any fix was made for it. Every element of the export is found under Revit's ElementId and class. Wall faces and layer sets come out as Revit's, and storey elevations are equal. Revit's GlobalIds are not rebuilt on 2015 (#878), 3 elements the export leaves out are written, and 16 footings sit on no storey (#879). Names cannot be scored: the model was renamed after the export.

## 1. The pair is one release, but not one moment

The export is Revit 2015's own, but it was written on 2014-10-08. The file was saved later, by the November 2014 build, and the archive dates it 2016-11-07. In between, the model's names changed language. The export names walls "Limestone wall 100", beams "Rafter 71 x 171" and storeys "-01 Foundation" to "03. Roof". The file holds "kalkzandsteen wand 100", "Gordingen 71 x 171" and "KS_PL -01. Fundering" to "KS_PL 03. Dak". rvt-rs writes the file's names. The five storeys' elevations agree to the millimetre (−800, 0, 3,500, 7,000, 10,500 mm), and every ElementId in the export is still in the file. A difference in a name, a type name, a material name or a `Reference` value is therefore the renaming, not a reading, and those scores are left out below.

## 2. Elements

| | |
|---|---:|
| elements in Revit's export with a Tag | 241 |
| of them written by rvt-rs, same ElementId | **241** |
| in Revit's class | 127 (91 IfcBeam, 15 IfcColumn, 16 IfcFooting, 5 IfcSlab) |
| in IFC4's class for Revit's IFC2X3 one | 114 (`IfcWall`, where IFC2X3 writes `IfcWallStandardCase`) |
| written by rvt-rs and not in Revit's export | **3** `IfcAirTerminal` (124771, 148100, 221553; #879) |

The three air terminals are older than every element the export holds (its Tags run from 392844 to 407741), so they are not additions after the export. What made Revit's exporter skip them is not decoded.

## 3. Geometry

| | |
|---|---|
| walls, faces across (113 axis-parallel) | 113 within 0.001 ft of Revit's |
| walls, both ends along | 103 within 0.001 ft, all 113 within 1 ft (worst 0.328 ft) |
| the one angled wall | faces 5.814 ft off, ends within 0.105 ft |
| layer sets, whole | 117 of 117 within 0.001 ft; by Revit's own usage, 115 of 117 (two slabs 0.75 and 0.89 ft) |
| slab outlines | 1 of 5 within 0.01 ft, area within 0.1% |
| bodies that are the record's box | 130 of 244 |

## 4. Identity and containment

- **GlobalIds:** 0 of 241. Every element gets the GlobalId rvt-rs derives from the document, so `revit_global_ids` read nothing from this file (#878). Type objects, through their own path, agree on 113 of 129 type GlobalIds.
- **Storeys:** the five Levels are read with Revit's elevations. 225 elements reach their storey through the Level their record names. The 16 footings sit on no storey, where Revit puts all 16 on one (#879). `storeys_vs_ifc` pairs storeys by name, so after the renaming it counts the other 225 as different storeys. Containment by elevation is not measured here.
- **Types:** all 129 elements Revit types are typed, each with Revit's entity, and 128 of 129 with Revit's type Tag.

## 5. What this measures, and what it does not

This is the first time a 2015 to 2022 release is scored against an export of its own release. Until now, RE-180 and RE-181 measured those releases against the same projects saved by Revit 2019 and 2023. It is one project, a structural model with no rooms, doors or windows. It does not admit Revit 2015: the release stays experimental, and its outputs say so (#871). Against the 1.0 bar (`docs/release-1.0-criteria.md`, W2 and W5) it is one held-out project that fails: on ElementIds, by the 3 extra elements; on identity, by the GlobalIds; and on containment, by the footings. A second 2015 project with Revit's export is needed before any fix made here can be checked on a file that is still held out.

## 6. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=main -f base=none -f bimcollab=true -f keep_ifc=true
gh run download <run id>   # measure-head/bimcollab-str-2015/*.txt
```
