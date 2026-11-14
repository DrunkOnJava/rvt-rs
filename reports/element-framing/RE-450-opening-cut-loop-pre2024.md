# RE-450: a Revit 2015 to 2023 door or window cuts the outline its symbol cuts

**Date:** 2026-11-13
**Probe:** `probes/examples/probe_re450_door_cut_loops.rs`
**Follows:** RE-215 (the symbol's cut loop, 2024 to 2027), #227 (the opening from the loop and the instance's transform), RE-244 (the opening's `Depth`), RE-280 (the 4-byte ElementId of 2015 to 2023), RE-448 (the host from the native record on 2015 to 2023)
**Files:** Autodesk's `rac_basic` and `rac_advanced` sample projects on every local save from 2015 to 2024 (`research/autodesk-sample-projects.tsv`), Einhoven (2023, local only), and the other local 2015 to 2023 files for the comparison of section 4.
**Result:** positive: on 2015 to 2023 a door's or window's symbol saves its cut loop and the instance its transform as on 2024, and the opening #227 places from them is, on every save of both projects, the 2024 save's opening in size, and in place on all but 24 windows of `rac_advanced`'s 2015 to 2020 saves, which sit 0.0075 m off it as they did before.

## 1. Before

On Revit 2024 to 2027 a door's or window's opening is the outline its symbol cuts in its host (`m_pCutLoops`, RE-215), placed by the instance's transform (#227), or its type's rough opening where that is read (RE-93, RE-94); the loop's extents are its `OverallWidth` and `OverallHeight`. The 2015 to 2023 path runs none of that: the cut-loop pass (`attach_cut_loop_sizes`) is gated to `CUT_LOOP_REVIT_VERSIONS`, its reader read records on 2023 to 2027 and took an ElementId only in 2024's `{"m_id":{"m_id64":n}}` shape, and the transform comes from `scan_instance_transforms`, whose element-data header is known from 2024 only. The opening was the element's own body, its record box, frame and trim included, and `OverallWidth` and `OverallHeight` were not written.

Exported from their 2023 and 2024 saves, the same projects cut different openings:

| Project | Openings | Equal to the 2024 save's within 0.001 m, on 2023 |
|---|---:|---:|
| `rac_advanced` | 107 | 24 |
| `rac_basic` | 26 | 3 |

Door 154488's opening is 1.067 by 2.21 m on 2023 and 0.915 by 2.134 m on 2024; `rac_basic`'s windows are 1.594 by 2.71 m on 2023 and 1.5 by 2.7 m on 2024.

## 2. What the records hold

A door's or window's `FamilyInstance` record holds an `InstanceInfo` on 2015 to 2023 as on 2024: the symbol that draws it in `m_symbolId`, saved as the 4-byte `{"m_id":{"m_id":n}}`, and its transform in `m_Trf` (the rotation `m_3x3` applied to the symbol's coordinates, then `m_or` added, as RE-321 reads it for a symbol's cut solid). The symbol's `FamilySymbol` record holds `m_pCutLoops`. The probe reads them on Revit 2015 to 2027 (`native_symbol_cuts::instance_cuts_and_symbols`):

| File | Doors and windows | Transform read | Cut loop read |
|---|---:|---:|---:|
| `rac_advanced` 2015, 2023, 2024 | 124 | 124 | 107 |
| `rac_basic` 2015, 2023, 2024 | 33 | 33 | 23 |
| Einhoven 2023 | 2 | 2 | 2 |

Every loop read on 2015 and 2023 has the 2024 save's extents for the same ElementId: 107 of 107 and 23 of 23. The other doors and windows are openings alone, cut nothing (#810) or are set in no wall, on 2024 as well.

## 3. What changes

- `native_symbol_cuts::instance_cuts_and_symbols` reads records on Revit 2015 to 2027 (`NATIVE_RELEASES`), takes `m_symbolId` and `m_hostId` in either ElementId shape (`identifier_i64`), and returns each instance's `InstanceInfo.m_Trf` as an `InstanceTransform`.
- On the 2015 to 2023 path, after the hosts are final (RE-448), `attach_cut_loop_openings_32` gives each door and window that draws geometry and whose symbol's loop is read the loop's extents (`CUT_LOOP_SIZE_FIELDS`) and the opening #227 places from the loop and the transform (`push_filler_opening`), on `CUT_LOOP_32BIT_REVIT_VERSIONS` (2015 to 2023).
- Nothing else of the 2024 cut-loop pass is run before 2024: not the flips (RE-343), the cut solids (RE-225), the drawn size (#1016) or the record-box size of a door that cuts no loop (#810).
- The 2024 to 2027 path reads the same values as before.

The probe, run on this change, finds 107 of 107 and 23 of 23 openings placed on 2015, 2023 and 2024, each the same size on each save; their centres and bottoms are the 2024 save's within 0.001 m, except 24 windows of `rac_advanced` 2015 (section 5).

## 4. Results

`rvt-ifc --profile ci` of RE-448 and of this change, on 58 local files of Revit 2015 to 2023 and on Core Interior (2024) and RE1 Architecture (2025):

- 34 files are identical apart from `FILE_NAME`, among them every 2024 and 2025 file.
- 26 differ. In each, the loop-cut openings change their outline and placement and gain `Qto_OpeningElementBaseQuantities.Depth` (RE-244), and their doors and windows gain `OverallWidth` and `OverallHeight`. No element, class or relation is added or removed.

| Files | Openings | Equal to the 2024 save's within 0.001 m, before → after |
|---|---:|---|
| `rac_basic` 2015, 2016, 2017, 2018, 2019, 2020, 2021, 2022, 2023 | 26 | 3 → 26 on each |
| `rac_advanced` 2021, 2022, 2023 | 107 | 24 → 107 on each |
| `rac_advanced` 2015, 2016, 2017, 2018, 2019, 2020, and a renamed 2019 copy of it | 107 | 0 → 83 on each; the other 24 are the 2024 save's size, 0.0075 m off it |
| `rme_basic` and `rme_advanced` 2015 and 2019, `Arch Link Model` (2019) | 107 each | no 2024 save local |
| Einhoven (2023) | 2 | no 2024 save; each 0.6096 by 1.2192 m, the 24 by 48 inch window its type names |
| `revit_sample_file` (`revit_files4`) | 1 | no 2024 save |

On every save of both projects each loop-cut door's and window's `OverallWidth` and `OverallHeight` are the 2024 save's: 107 of 107 and 23 of 23, none written before.

On the 2023 saves, walls whose oversized openings had left them without some base quantities now get them: 21 walls of `rac_advanced` and wall 424922 of `rac_basic`. 21 of those 22 equal the 2024 save's quantities; wall 144180 of `rac_advanced` lacks the 2024 save's `Height` and has its other five. The walls of the 2015 saves' exports keep the quantities they had.

## 5. Limits

- 24 windows of `rac_advanced` (145607 among them) lie 0.0075 m across their wall from the 2024 save's openings on its 2015 to 2020 saves, before and after this change, with the 2024 save's size. Their transforms' origins differ by that much between those saves and 2024 (145607: `m_or` y 100.4841 ft on 2015, 100.4595 ft on 2023 and 2024); 2021 to 2023 have 2024's.
- A door or window whose symbol cuts no loop, several loops or an arc keeps its body as its opening, as before. The 2024 path's rules for those (RE-225, #810, RE-318) are not run before 2024.
- The type's rough opening (RE-93, RE-94) is not read before 2024; where it is read on 2024 the two projects' openings equal the loop's.
- No Revit export of a 2015 to 2023 save is local. The witness is the same project's 2024 save, as for RE-364, RE-366, RE-447 and RE-448.
