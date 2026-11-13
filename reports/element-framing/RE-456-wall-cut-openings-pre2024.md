# RE-456: a Revit 2015 to 2023 wall's cut steps name their cutter in a 4-byte ElementId

**Date:** 2026-11-12
**Probe:** `probes/examples/probe_re456_wall_cut_steps.rs`
**Files:** Autodesk's `rac_basic` and `rac_advanced` sample projects on every local save from 2015 to 2027 (`research/autodesk-sample-projects.tsv`), Autodesk's `rme_basic` and `rme_advanced` 2015 and 2019, `Revit_IFC5_Einhoven.rvt` (2023, MIT, `magnetar-io/revit-test-datasets`), BIMcollab's Revit 2015 structural example and Revit's 2014 export of it (RE-198), the BIM4LCA office (2024), Snowdon Towers Architectural (2024, 2025), and every other local file that has a rectangular wall opening.

## 1. The question

The Revit 2015 to 2023 coverage warning lists a wall's "profile openings" among what is not read. Exporting the same project from its 2023 and its 2024 save shows what that costs. The two exports hold the same building elements, ElementId for ElementId. They differ in 24 MEP ports and in 8 opening elements:

- `rac_basic` writes 26 `IfcOpeningElement`s from its 2023 save and 33 from its 2024 save;
- `rac_advanced` writes 107 and 108.

The 8 openings missing from 2023 are named after a wall. Their tags name what cuts the wall:

| Project | Tag | Native class of the tag (2023 and 2024) | Wall |
|---|---|---|---|
| `rac_basic` | 504396 | `SWallRectOpening` | 430064 |
| `rac_basic` | 699571 | `SWallRectOpening` | 198749 |
| `rac_advanced` | 148770 | `SWallRectOpening` | 148769 |
| `rac_basic` | 529716, 709879, 705020, 1001895 | `CurveElem` (sketch lines) | 198694, 428745, 845266 |
| `rac_basic` | 906937 | `FamilyInstance` (a door) | 422243 |

The three rectangular wall openings and the door's cut in a second wall are what RE-321 writes from the steps of the wall's native record (`native_wall_cuts`). The four sketch-line openings are the profile openings of RE-151 and RE-380, which this report does not change.

On 2023, RE-321's reader wrote nothing and counted nothing as left out: `exported.wall_cut_openings_left_out` was empty.

## 2. What the records hold

`native_wall_cuts::read_wall_records` reads each wall's `SWall` or `ArcWall` record and takes, from every `WallEmbedWallGStep`, `WallCutoutGStep`, `WallRectOpeningGStep` and `WallShadowCutoutGStep`, the cutter its `m_instId` names. It read that ElementId as `{"m_id":{"m_id64":n}}`, the 2024 shape. The probe counts the shape of every `m_instId` of those steps:

| File | `m_id` (4 bytes) | `m_id64` | Steps | Walls with a step |
|---|---:|---:|---:|---:|
| `rac_basic` 2015 | 33 | 0 | 33 | 13 |
| `rac_basic` 2019 | 34 | 0 | 34 | 13 |
| `rac_basic` 2023 | 34 | 0 | 34 | 13 |
| `rac_basic` 2024 | 0 | 34 | 34 | 13 |
| `rac_advanced` 2015 | 112 | 0 | 112 | 39 |
| `rac_advanced` 2023 | 112 | 0 | 112 | 39 |
| `rac_advanced` 2024 | 0 | 112 | 112 | 39 |
| Einhoven (2023) | 2 | 0 | 2 | 1 |

Every step of 2015 to 2023 saves its cutter as `{"m_id":{"m_id":n}}`, the 4-byte Identifier of RE-280, so none was read. Read in either shape, the steps of the 2023 saves are the 2024 saves' (wall, kind, cutter and `m_bCutSomething`), line for line, on both projects. `rac_basic` 2015 has one step fewer: wall 198749's embed of curtain wall 849032, which is not in that save.

`native_document::Options` reads native records on 2023 to 2027 by default, so before this change 2015 to 2022 were not read at all.

A rectangular opening's rectangle is its `RectOpeningData.m_aPoint`, two 3D corners. 2019 to 2027 save it, and the probe reads the same corners on 2019, 2023 and 2024:

| Opening | Corners (ft), 2019, 2023 and 2024 alike |
|---|---|
| 504396 | (-2.1295, 4.4567, 9.8425) to (1.1514, 4.4567, 17.8425) |
| 699571 | (22.8377, -7.7677, 0) to (22.8377, -4.7165, 7.0538) |
| 148770 | (-32.3123, 19.9063, 0) to (-32.3123, 72.7700, 11.6470) |

2015 to 2018 save no `m_aPoint`, only two corners in the wall's face, `m_lowerLeftCorner` and `m_upperRightCorner`; 2019 on save them beside `m_aPoint`, with the same values on every save:

| Opening | Host wall | `m_lowerLeftCorner`, `m_upperRightCorner` | Host's location line (`GLine` `m_origin`, `m_dirVec`) |
|---|---|---|---|
| 504396 | 430064 | (-5.8727, 0), (-2.5919, 8.0) | (3.7432, 4.4567, 9.8425), (1, 0, 0) |
| 699571 | 198749 | (12.2047, 0), (15.2559, 7.0538) | (22.8377, 7.4882, 0), (0, -1, 0) |

The corners are `[u, v]`: `u` along the line `VWallDriver.m_pCrv` of the opening's `m_hostId` names, from its `m_origin` in its `m_dirVec`, and `v` up from the origin. 504396's corners so placed are (3.7432 - 5.8727, 4.4567, 9.8425 + 0) = (-2.1295, 4.4567, 9.8425) and (1.1514, 4.4567, 17.8425), its `m_aPoint`; 699571's are (22.8377, 7.4882 - 12.2047, 0) and (22.8377, 7.4882 - 15.2559, 7.0538), its `m_aPoint`. 198749's line is not the first `GLine` of its record, which holds 9; `m_pCrv` names it (the pointer RE-360 and #1410 read). Both `MidParam`s of each wall's record hold the opening's two `u` values.

The probe places every rectangle's face corners so and compares them with `m_aPoint` where the save holds both. Run on every local file that has a rectangular wall opening, they agree to 1e-6 ft on all 220 that save both:

| Files | Openings saving both | Agree |
|---|---:|---:|
| `rac_basic` and `rac_advanced`, 2019 to 2027 saves | 27 | 27 |
| BIM4LCA office, as built and building permit (2024) | 163 | 163 |
| Snowdon Towers Architectural (2024 and 2025) | 22 | 22 |
| `Arch Link Model` and `Technical_school-current_m` (2019), `2025_RichTestA` and `2025_RichTestB` (2025) | 8 | 8 |

(BIM4LCA's 4 others are in arc walls, whose `m_pCrv` is a `GArc`; they save `m_aPoint` and are read from it.)

BIMcollab's Revit 2015 structural example saves 19 rectangular openings in the face only. Placed on their walls' lines and compared with the openings of Revit's own export of the model (`02_BIMcollab_Example_STR_optimized.ifc`, written in 2014 and optimised by Solibri, RE-198), each of 15 equals one of that export's along the wall and up, to 0.0005 m, and lies within its depth. One is in a wall at an angle to the axes (406535) and matches within 0.046 m in plan, the box of a turned wall's thickness. The other three, 426282, 426328 and 426366, all in wall 407227, have no counterpart: they are the save's highest-numbered openings, and the export predates the save.

## 3. What changes

- `native_wall_cuts` reads an ElementId in either shape (`native_json::identifier_i64`), so a 2015 to 2023 step names its cutter.
- Its four native reads (the walls' records, the rectangles, and the instances and symbols of a cabinet's cut) run on Revit 2015 to 2027 (`NATIVE_RELEASES`), not on 2023 to 2027.
- `rect_opening_boxes` reads a rectangle with no `m_aPoint` from its face corners, placed on its `m_hostId`'s location line where `VWallDriver.m_pCrv` names one level `GLine` (`native_walls::location_curve_index`, now `pub(crate)`). A rectangle with `m_aPoint` is read from it, as before.
- Nothing else. RE-321's rules (`ifc::wall_cut_openings`) decide what is written, as on 2024.

`walls_with_no_base_join` reads the same records and so now reads 2015 to 2022 too; its caller, the saved-mesh pass, runs on 2023 to 2027 only, so nothing it gives changes.

## 4. Results

`rvt-ifc --profile ci` of the branch's base and of this change, on 58 local files of Revit 2015 to 2023 and on Core Interior (2024) and RE1 Architecture (2025):

- 39 files are identical apart from `FILE_NAME`, among them every 2024 and 2025 file and Einhoven.
- 21 differ, each only by added openings and their placement, shape, voids relation, property sets and quantities. No other entity changes.

| Files | Openings added | Against the same project's 2024 save |
|---|---|---|
| `rac_basic` 2015, 2016, 2017, 2018, 2019, 2020, 2021, 2022, 2023 | 504396 and 699571 (26 → 28) | equal in placement, outline and depth to 0.001 m |
| `rac_advanced` 2015, 2016, 2017, 2018, 2019, 2020, 2021, 2022, 2023, and a renamed 2019 copy of it (`Technical_school-current_m`) | 148770 (107 → 108) | equal |
| BIMcollab's structural example (2015) | 18 (0 → 18) | no 2024 save; 15 equal to Revit's own export of it, below |
| `Arch Link Model` (2019, local only) | 197712, 201549, 201673 (107 → 110) | no later save; their `m_aPoint` and face corners agree (section 2) |

On the 2015 save, 699571's placement relative to its wall differs from 2024's by 0.072 m and its profile by the same amount the other way: wall 198749 is 0.475 ft shorter there (its location line ends at 17.976 ft, not 18.451), so the wall's own placement moves. The opening's box in the model is the same.

The openings' quantities are the 2024 save's: 504396 1.0 by 2.4384 by 0.12 m, 699571 0.93 by 2.15 by 0.202 m. On `rac_basic` 2023 the opening in wall 198749 is named `Wall-198749` and has no property set or quantities, as its wall has none there: the wall is written unnamed on that save (`Wall-198749`), where 2020 and 2024 name it `Basic Wall:Wall - Timber Clad:198749`. That is the wall's own name, not read here.

BIMcollab's 18 written openings, their world boxes computed from the placements and extrusions of both files, against the 51 of Revit's export: the 15 that the export holds are equal to its to 0.0001 m in every coordinate, depth included (the export's 100 mm walls). The other three, 426282, 426328 and 426366, are those the export predates (section 2). The nineteenth, 406535, is in a wall at an angle to the axes and is counted `rectangular_opening_not_in_wall`.

The diagnostics now count, on 2015 to 2023, the steps that write no opening (`exported.wall_cut_openings_left_out`), where before they counted nothing:

- `rectangular_opening_not_in_wall`: BIMcollab's 406535;
- `curtain_wall_box_not_read`: 3 or 4 curtain walls a save (2024 counts them as `curtain_wall_within_half_foot_of_wall_end`);
- `door_opening_box_not_read`: one door of `rac_basic` that cuts a second wall, section 5.

No local file counts `rectangular_opening_not_read`.

## 5. Limits

- A 2015 to 2018 rectangle in a wall whose location line is an arc, or rises, is not placed and is counted `rectangular_opening_not_read`. No local 2015 to 2018 file has one; on BIM4LCA (2024) the 4 in arc walls are read from `m_aPoint`.
- One door of `rac_basic` that cuts a second wall is counted `door_opening_box_not_read` on every save from 2015 to 2023, where the 2024 save writes its opening: a door's own opening box comes from its symbol's cut loop, which is read from 2024 only. Since RE-448 sets each 2015 to 2023 door in the wall its `m_hostId` names, no step counts `door_cuts_a_wall_it_is_not_hosted_in_as_its_own`.
- No Revit export of a 2015 to 2023 save writes any of the openings added. The witness is the same project's 2024 save, as for RE-364 and RE-366.
