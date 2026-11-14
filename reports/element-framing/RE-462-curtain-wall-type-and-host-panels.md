# RE-462: a wall of a curtain wall type is a curtain wall unless it is the host panel of another (#1960)

**Date:** 2026-11-14
**Issues:** #1960 (the 2023 and 2024 rules disagree on two single-cell curtain walls of `rme_basic`); follows RE-440 (the 2023 rule), RE-46 and RE-72 (the 2024 rule)
**Artefacts:** Autodesk's sample projects of 2023 and 2024 (`rac_basic`, `rac_advanced`, `rme_basic`, `rme_advanced`; the pins are in `research/autodesk-sample-projects.tsv`), Autodesk's Snowdon Towers 2024 architectural sample (local only) with Revit's own IFC4 export of it (`bldrs-ai/test-models`, no licence, never committed), and Revit's own 2018 IFC export of `rac_basic` (public, not committed).
**Probe:** `probes/examples/probe_re462_curtain_wall_structure.rs`
**Runs:** probe run 43542030075 (`3e28f9f4`, 14 models, no base); the change's Measures 43544055702 (unfiltered), 43546540315 (kept exports) and 43546590766 (timing), at `c802574c`.
**Status:** positive for the rule below on every file that has a Revit export; the one case the issue is about, a mullion-free single-cell curtain wall, has no Revit export in any corpus, and what Revit writes for it rests on the source of its exporter.

## 1. The question

The Revit 2023 reader (RE-440, `attach_curtain_walls_2023`) makes a wall a curtain wall by its type, a `NewCurtainWallType`. The 2024 reader (RE-46, RE-72, `attach_curtain_walls`) makes a wall one where a curtain-wall mullion names it or a curtain grid is found, and its doc says a panel alone does not (walls named only by a panel are basic walls used as infill, measured on Snowdon Towers). They disagree on two walls of `rme_basic`, 573660 and 573664: one grid cell, one panel, no mullion. Revit 2023 gives `IfcCurtainWall`, 2024 gives `IfcWall`, and no Revit export of `rme_basic` exists (#1960).

## 2. What Revit's exporter does

Read in `Autodesk/revit-ifc` (CLEANROOM.md source 5), by file and class, nothing copied:

- `CurtainSystemExporter.IsCurtainSystem` is true for a wall that has a Revit curtain grid, which is every wall of a curtain wall type, and `ExporterUtil`'s choice of export type (its fourth step) then gives `IfcCurtainWall`. Nothing in either tests mullions or the number of cells.
- A wall that is the host panel of another curtain wall is not exported as itself: `WallExporter.Export` returns for a wall whose `StackedWallOwnerId` is a curtain wall, and `CurtainSystemExporter.GetVisiblePanelsForGrid` folds a host panel that is itself a curtain wall into its owner's parts.

## 3. What the records hold

The probe prints, for every wall of a curtain wall type (an `SWall` or `ArcWall` whose type is a `NewCurtainWallType`), the objects its own record holds. On the 14 models of run 43542030075:

| Model | Curtain-type walls | One cell | With `HostPanelInfo` | Named by another wall's panel |
|---|---:|---:|---:|---:|
| `rac_basic` 2023, 2024 | 9 | 1 (708088) | 0 | 0 |
| `rac_advanced` 2023, 2024 | 34 | 0 | 0 | 0 |
| `rme_basic` 2023, 2024 | 33 | 2 (573660, 573664) | 0 | 0 |
| `rme_advanced` 2024 | 33 | 2 | 0 | 0 |
| Snowdon Towers architectural 2024 | 65 | 23 | 23 | 23 |

- **The 23 one-cell walls of Snowdon Towers are host panels.** All 23 are of the one type, 1005464 "Solar Wall Window Frame". Each is named by one panel of another curtain wall (seven owners: 1054769, 1054967, 1055165, 1055363, 1055452, 1055522, 1055717) and holds a `HostPanelInfo` (`m_panelId` the owner's placeholder panel, `m_scheduleAsPanel` true). No other wall of the 14 models holds one.
- **708088 has mullions, 573660 and 573664 have none.** The wall's own record holds a `CGDriverMullionData`, whose `m_mullionsByEdge` lists four mullions on 708088 (708090 to 708093) and which the records of 573660 and 573664 do not hold. (The probe's columns for grid-line and `Mullion` records name the wall from outside and read 0 on all 252 walls: they say nothing.)

## 4. What Revit writes, against what rvt-rs wrote

Revit's own exports:

- `rac_basic` 2018: all 9 curtain-type walls are `IFCCURTAINWALL`, 708088 among them with one `IFCPLATE` and four `IFCMEMBER`.
- Snowdon Towers architectural: 42 of the 42 curtain-type walls that are not host panels are `IfcCurtainWall`. None of the 23 host panels is written, nor their panels; for 2339390 the four mullions (2339403 to 2339406) are `IfcMember` parts of the owner 1054967, and neither its panel (2339391) nor the owner's placeholder panel (1054987) is written.

rvt-rs before the change (the kept exports of run 43535235103): the 2023 reader writes 573660 and 573664 as `IfcCurtainWall` with one `IfcPlate` each, the 2024 reader as `IfcWall`; on Snowdon Towers the 23 host panels are `IfcPlate` carrying the wall's own Tag, which Revit's export has nothing for (23 extras, not changed here).

## 5. The rule

A wall of a curtain wall type that is not a host panel (its record holds no `HostPanelInfo`) is a curtain wall: on 2023 as before, on 2024 to 2027 by its type as well as by a mullion or a grid. A host panel is not one on either. The rule is Revit's exporter's, and it is exact on every measured file: 9 of 9 on `rac_basic` (Revit 2018), 42 of 42 on Snowdon Towers, with the 23 host panels left out. It changes the one case no export measures, the mullion-free single cell of `rme_basic` and `rme_advanced`, to what the exporter's source gives.

## 6. Results (2026-11-14)

The rows were registered on #1960 before any run of the change ([comment of 02:02 UTC](https://github.com/DrunkOnJava/rvt-rs/issues/1960#issuecomment-6869851390)). The change is `c802574c`, measured against `main` (`2b4f18b5`) and scored by the scorers of `0ce6c5d6`:

- unfiltered, every switch of run 43525326226 (samples, nordic, r2025, racadv, holdout, bimcollab, families, every scorer): Measure [43544055702](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43544055702);
- both sides' exports kept for the 2023 and 2024 saves of the four samples and Snowdon Towers' five files: Measure [43546540315](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43546540315);
- timing: Measure [43546590766](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43546590766).

1. **Only the files that hold a mullion-free single-cell curtain wall move: holds.** Of the 146 exports 138 are byte-identical to `main`'s and 8 differ: the 2024, 2025, 2026 and 2027 saves of `rme_basic` and `rme_advanced`. Of the 121 diagnostics 113 are equal and the same 8 differ. The 2015 to 2023 saves, the other samples of every year, Core Interior, Einhoven, RE1, the BIM4LCA models, the 24 Revit 2025 models, Snowdon Towers' five files, BIMcollab, the `racadv` pairs and every held-out model (counts only) are identical.
2. **In each moved export the difference is the two walls: holds, with two things the row did not name.** The element diff of the kept 2024 exports (`tools/re/ifc_element_diff.py`, by GlobalId; 6,828 elements on `rme_basic` and 6,783 on `rme_advanced`, none in only one file) finds three elements that differ, the same three on both: 573660 and 573664, and the plate 573665.
   - The two walls go from `IfcWall` to `IfcCurtainWall` named `Curtain Wall:Standard:<id>`. Their own body goes (a curtain wall carries none), as do `Pset_WallCommon` and `Qto_WallBaseQuantities.Height`; `Pset_CurtainWallCommon`, `Qto_CurtainWallQuantities.Length`, the family names and the type name come. Against the 2023 save's export they are the same class and name, and their property sets differ in the three sets that also differ on 574261, a wall both readers already agreed on (`Constraints.Top Constraint`, `Qto_CurtainWallQuantities.Length`, `RvtElementRecordGeometry`).
   - Their panels are aggregated to them, 573661 to 573660 and 573665 to 573664. Not in the row: before the change 573665 was a part of no curtain wall and 573661 was a part of 574261, a wall that does not hold it (52 panels where the 2023 save has 51); each is under its own wall now, and 573665 gains `Pset_PlateCommon.IsExternal` as every plate under a curtain wall has it.
3. **The 2023 and 2024 saves of `rme_basic` agree on every curtain wall: holds.** By Tag the class and name are equal on 33 of 33 curtain walls (31 of 33 before; `tools/re/curtain_classes_across_saves.py`) and so are their aggregated panels, on 33 of 33 (30 of 33 before: the two walls and 574261), with every one of the 629 plates under a curtain wall (628, and one under none, before). The 2026 save of `rme_advanced` against its 2025 save compares `IsExternal` on 33 curtain walls and 629 plates, all equal (31 and 628 before).
4. **Snowdon Towers' export does not move: holds.** All five files are byte-identical, so the 23 host panels are neither written as walls nor as curtain walls by the change. They are still written as `IfcPlate`, which Revit's export has nothing for (#1975).
5. **The gates hold: holds, and the diagnostics move by more than the two walls' counts.** The same-project gates passed on the head (check run 132299172379, from run 43544055702). The diagnostics of the two moved 2024 exports differ in 14 values, all of them consequences of the two walls becoming curtain walls and the plate a part: `CurtainWall` 31 to 33 and `Wall` 135 to 133 (in `production_class_counts` and `by_ifc_type`), `building_elements_carried_by_parts` 31 to 33, `building_elements_with_geometry` down by 2 and the geometry fraction with it, `total_entities` up by 4, `parameter_value_count` up by 1 (the plate's property), one storey warning's counts down by one element (the plate is a part now), and `revit_compound_assemblies_and_walltype_widths` leaves the unsupported features, which only those two walls without layers raised (the 2023 save's list never had it).
6. **No cost worth a rule: holds.** Over the 21 files timed the head's time over the base's is at most 1.025 (median 1.01; 121.2 s in all against 122.8 s), and Snowdon Towers architectural, the largest, takes 27.0 s against 27.7 s. Peak memory is within 1% of the base on 20 of the 21. Snowdon HVAC's median reads 237 MiB on the base and 251 on the head, but each side's rounds are bimodal (about 237 and 251 MiB; two of the head's three rounds fell in the upper mode and one of the base's), and the file holds no curtain wall.

## 7. What is not claimed

- That Revit writes `IfcCurtainWall` for 573660 and 573664: no export of those files exists, and the claim rests on `revit-ifc`'s source. A Revit export of a mullion-free single-cell curtain wall would settle it.
- Anything about the 23 `IfcPlate` extras on Snowdon Towers: they are the host panels written as panels, a finding of its own (#1975).
- The 2015 to 2022 saves, where the 2023 type rule does not run and no curtain wall of this kind is measured.
- That the 2025 to 2027 saves match Revit: none has an export. Their two walls move as the 2024 saves' do, and the 2026 save of `rme_advanced` agrees with its 2025 save (row 3).
