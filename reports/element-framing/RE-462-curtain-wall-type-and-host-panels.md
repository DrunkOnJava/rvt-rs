# RE-462: a wall of a curtain wall type is a curtain wall unless it is the host panel of another (#1960)

**Date:** 2026-11-14
**Issues:** #1960 (the 2023 and 2024 rules disagree on two single-cell curtain walls of `rme_basic`); follows RE-440 (the 2023 rule), RE-46 and RE-72 (the 2024 rule)
**Artefacts:** Autodesk's sample projects of 2023 and 2024 (`rac_basic`, `rac_advanced`, `rme_basic`, `rme_advanced`; the pins are in `research/autodesk-sample-projects.tsv`), Autodesk's Snowdon Towers 2024 architectural sample (local only) with Revit's own IFC4 export of it (`bldrs-ai/test-models`, no licence, never committed), and Revit's own 2018 IFC export of `rac_basic` (public, not committed).
**Probe:** `probes/examples/probe_re462_curtain_wall_structure.rs`
**Runs:** probe run 43542030075 (`3e28f9f4`, 14 models, no base); the change's Measure is named in section 6.
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

## 6. Results

To be filled from the change's Measure.

## 7. What is not claimed

- That Revit writes `IfcCurtainWall` for 573660 and 573664: no export of those files exists, and the claim rests on `revit-ifc`'s source. A Revit export of a mullion-free single-cell curtain wall would settle it.
- Anything about the 23 `IfcPlate` extras on Snowdon Towers: they are the host panels written as panels, a finding of its own.
- The 2015 to 2022 saves, where the 2023 type rule does not run and no curtain wall of this kind is measured.
