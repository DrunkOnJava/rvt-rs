# RE-346: Revit 2026 and 2027 pipes, ports, door and window cuts, and the properties read on 2025

**Date:** 2026-11-05
**Issues:** #1190 (each capability 2024 and 2025 have, on every release from 2015 to 2027); follows RE-341 (beams, stair runs, landings and sketch lines on 2026 and 2027), RE-124 and RE-179 (2026 and 2027 element records are 2025's).
**Artefacts:** Autodesk's own sample projects, pinned in `research/autodesk-sample-projects.tsv` and fetched by `tools/fetch-autodesk-samples.sh`, never stored: `rme_advanced` 2025 (`08457292…886d5`), 2026 (`1f34a8f1…5d7af`) and 2027 (`899ea8a6…fca7a`); `rme_basic` 2025 (`037165e5…c8099`), 2026 (`37127cf5…ead94`) and 2027 (`886bb0d1…ae1a7`); `rac_advanced` 2025 (`512e9188…e25d1`), 2026 (`f5fb3712…83ace`) and 2027 (`a88eb9f8…024ed`); `rac_basic` 2025 (`41f1893a…d4293`), 2026 (`bc547fe9…d99dc`) and 2027 (`c9ce4d01…91a01`); `rst_basic` 2025 (`5089b2c6…31acb`), 2026 (`e7cf23a9…75418`) and 2027 (`01685786…d1453`).
**Scorers:** `tools/re/save_pair_bodies.py` (RE-341), `tools/re/save_pair_props.py` and `tools/re/save_pair_ports.py` (new).
**Status:** positive against the same samples saved by Revit 2025. **No Revit 2026 or 2027 IFC export is available**, so nothing here is measured against Revit's own export of a 2026 or 2027 file. The oracle is the same model saved by Revit 2025, whose export is measured against Revit's (#869, RE-131, RE-193, RE-215, RE-231, RE-254, RE-255, RE-296, #323). It cannot catch an error the 2025 save shares.

## 1. Before

After RE-341, ten more readers are still limited to 2024 and 2025 (one of them to 2024 to 2026) by their release lists, so on 2026 and 2027 a pipe is its record box, a family instance's ports are left unplaced, a wall is not cut by its doors' and windows' outlines, and these properties are not written:

| reader | release list before | what it gives |
|---|---|---|
| a family instance's unjoined ports (`PORT_PLACEMENT_REVIT_VERSIONS`, #869) | 2024, 2025 | each `IfcDistributionPort`'s placement |
| a pipe's axis (`partition_pipe_axes`, RE-131) | 2024, 2025 | `IfcPipeSegment` as its cylinder, its `Qto_PipeSegmentBaseQuantities` |
| an MEP curve's fields (`partition_curve_fields`) | 2025 | duct `Length`, pipe `InvertElevation` |
| a door's or window's cut loop (`CUT_LOOP_REVIT_VERSIONS`, RE-215) | 2024, 2025 | the wall cut to the opening's outline, its `Qto_DoorBaseQuantities` and `Qto_WindowBaseQuantities` |
| `IsExternal`'s last step (`IS_EXTERNAL_LAST_STEP_REVIT_VERSIONS`, RE-193) | 2024, 2025 | `IsExternal` in each common property set |
| a room's height (`ROOM_HEIGHT_REVIT_VERSIONS`, RE-255) | 2024 to 2026 | `Qto_SpaceBaseQuantities.Height` |
| a room as a prism (`ROOM_PRISM_REVIT_VERSIONS`, RE-296) | 2024, 2025 | `Qto_SpaceBaseQuantities.NetVolume` |
| a railing type's top rail height (`RAILING_HEIGHT_REVIT_VERSIONS`, RE-231) | 2024, 2025 | `Pset_RailingCommon.Height` |
| a stair railing's length (`RAILING_LENGTH_REVIT_VERSIONS`, RE-254) | 2024, 2025 | `Qto_RailingBaseQuantities.Length` |
| a stair's and its parts' names (`NATIVE_STAIR_NAME_REVIT_VERSIONS`, #323) | 2025 | the stair's, its runs' and stringers' `Name`, family and type names |

## 2. The change

The ten lists gain 2026 and 2027 (room height, 2027). The records these readers read are the 2025 ones there (RE-124, RE-179), and nothing else in the readers changes.

Three more lists were tried and left alone, since widening them changes no byte of any 2026 export here: a wall type's face angles (`WALL_FACE_ANGLES_SUPPORTED_REVIT_VERSIONS`), a sketch's splines (`SKETCH_SPLINE_SUPPORTED_REVIT_VERSIONS`) and connector pairs (`CONNECTOR_PAIR_SUPPORTED_REVIT_VERSIONS`). Nothing measures them on 2026 or 2027.

Each of the ten changes at least one 2026 export. Leaving one out at a time (on `main` 182ed219, before RE-341), the samples whose 2026 export it changes:

| reader left out | `rac_advanced` | `rac_basic` | `rme_advanced` | `rme_basic` | `rst_basic` |
|---|---|---|---|---|---|
| ports | yes | yes | yes | yes | |
| pipe axis | | | yes | yes | |
| curve fields | | | yes | yes | |
| cut loop | yes | yes | yes | yes | |
| `IsExternal` | yes | yes | yes | yes | yes |
| room prism | yes | | yes | yes | |
| room height | 2027 only: `rac_advanced`, `rme_advanced`, `rme_basic` | | | | |
| railing height | yes | yes | yes | yes | |
| railing length | yes | | | | |
| stair names | yes | yes | | | yes |

## 3. The 2026 and 2027 saves against their 2025 saves

`rvt-ifc FILE -o model.ifc` on each save with the RE-341 head (c8fb8d19) and with this change, each 2026 and 2027 export scored against the 2025 save's export by the RE-341 head. Elements are paired by ElementId (`Tag`), rooms by GlobalId, and ports by their host's `Tag`, `Name` and `FlowDirection`.

**Bodies** (`save_pair_bodies.py`: world box to 3 mm, volume to 0.1%), elements that differ from 2025, RE-341 then this change:

| sample, 2026 and 2027 alike | differ before | differ here | which change |
|---|---:|---:|---|
| `rme_advanced` | 490 of 6,605 | 0 | 454 pipes and 36 walls become 2025's |
| `rme_basic` | 493 of 6,650 | 0 | 457 pipes and 36 walls |
| `rac_advanced` | 56 of 5,527 | 10 | 46 walls |
| `rac_basic` | 22 of 438 | 17 | 5 walls |
| `rst_basic` | 32 of 453 | 32 | none (beams are held to the 2024 save, RE-341) |

No body that was 2025's stops being it (element for element, on each 2026 save). `rac_basic`'s other 16 walls differ before and after. Rooms' bodies are 2025's before and after: 91 of 91 on `rac_advanced` and 76 of 76 on `rme_advanced`.

**Ports** (`save_pair_ports.py`: origin to 3 mm, axis to 1e-3): before, no port of a 2026 or 2027 save is placed; here every one is, and every one is its 2025 save's: 6,959 on `rme_advanced`, 7,046 on `rme_basic`, 30 on `rac_advanced` and 60 on `rac_basic`, on 2026 and 2027.

**Properties and quantities** (`save_pair_props.py`, floats to four places): this change writes no value that differs from 2025's or that 2025 does not write; for every property name, the paired elements that differ, and those that have it on the 2026 or 2027 side only, are as many as on the RE-341 head. The values 2025 writes and the RE-341 head does not, which are now written and equal 2025's:

| sample | 2026 | 2027 |
|---|---:|---:|
| `rme_basic` | 3,454 | 3,530 |
| `rme_advanced` | 3,449 | 3,525 |
| `rac_advanced` | 1,315 | 1,406 |
| `rst_basic` | 440 | 440 |
| `rac_basic` | 169 | 163 |

The properties and quantities that now agree with 2025's on every paired element include, on every sample that has them: `IsExternal` in every common set the export writes, `Pset_RailingCommon.Height`, `Qto_RailingBaseQuantities.Length`, the door and window `Width`, `Height` and `Area`, `Qto_SpaceBaseQuantities.NetVolume` (and `Height` on 2027), the pipe `Length`, `GrossCrossSectionArea`, `OuterSurfaceArea` and `InvertElevation`, the duct `Length`, and the stairs' and stringers' names with their family and type names (`rac_advanced`'s 35, `rac_basic`'s and `rst_basic`'s 4).

What still differs from 2025's is what differs on the RE-341 head, among it `IsExternal` on walls, curtain walls, members and plates, which 2025 writes and 2026 does not on either head, and the beams, landings and outlines RE-341 holds to the 2024 save.

`tools/re/bodies_across_saves.py` (RE-341) passes on both heads for each 2026 and 2027 save of `rac_advanced`, `rac_basic` and `rst_basic`.

## 4. Nothing else moves

120 files were exported with the RE-341 head (c8fb8d19) and with this change, `SOURCE_DATE_EPOCH=1700000000`. 92 are byte for byte the same, IFC and diagnostics: every 2015 to 2025 file here, Autodesk's samples of 2015, 2016 and 2019 to 2025, Core Interior, the four RE1 models, Einhoven, the BIM4LCA office and terrain housing, and puzzbobb's Revit 2025 models. The 28 that change are 2026 and 2027 saves of Autodesk's samples: `rac_advanced`, `rac_basic`, `rme_advanced`, `rme_basic`, `rst_basic` and `rst_advanced`.

## 5. Not measured

- Revit 2026 and 2027 against Revit's own export: none is available.
- `IsExternal` on walls, curtain walls, members and plates on 2026 and 2027: still not written there, on either head.
- A wall type's face angles, sketch splines and connector pairs on 2026 and 2027: no sample here changes, so they keep their lists.
- 2015 to 2023: these readers stay at 2024 and later.
- `bodies_across_saves.py` does not yet hold pipes, ports, cut walls or properties; they are scored here by hand.

## 6. Reproduce

```bash
tools/fetch-autodesk-samples.sh samples 2025 2027
for f in samples/*.rvt; do rvt-ifc "$f" -o "${f%.rvt}.ifc"; done
python3 tools/re/save_pair_bodies.py samples/2026-rmeadvancedsampleproject.ifc samples/2025-rmeadvancedsampleproject.ifc
python3 tools/re/save_pair_ports.py samples/2026-rmeadvancedsampleproject.ifc samples/2025-rmeadvancedsampleproject.ifc
python3 tools/re/save_pair_props.py samples/2026-racadvancedsampleproject.ifc samples/2025-racadvancedsampleproject.ifc
```
