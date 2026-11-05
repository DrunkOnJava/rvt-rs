# RE-346: Revit 2026 and 2027 pipes, ports, door and window cuts, and the properties read on 2025

**Date:** 2026-11-05
**Issues:** #1190 (each capability 2024 and 2025 have, on every release from 2015 to 2027); follows RE-341 (beams, stair runs, landings and sketch lines on 2026 and 2027), RE-124 and RE-179 (2026 and 2027 element records are 2025's). Contributed by puzzbobb (#1454); the gate and what section 5 leaves out are the maintainer's.
**Artefacts:** Autodesk's own sample projects, pinned in `research/autodesk-sample-projects.tsv` and fetched by `tools/fetch-autodesk-samples.sh`, never stored: `rme_advanced` 2025 (`08457292…886d5`), 2026 (`1f34a8f1…5d7af`) and 2027 (`899ea8a6…fca7a`); `rme_basic` 2025 (`037165e5…c8099`), 2026 (`37127cf5…ead94`) and 2027 (`886bb0d1…ae1a7`); `rac_advanced` 2025 (`512e9188…e25d1`), 2026 (`f5fb3712…83ace`) and 2027 (`a88eb9f8…024ed`); `rac_basic` 2025 (`41f1893a…d4293`), 2026 (`bc547fe9…d99dc`) and 2027 (`c9ce4d01…91a01`); `rst_basic` 2025 (`5089b2c6…31acb`), 2026 (`e7cf23a9…75418`) and 2027 (`01685786…d1453`). Held-out: the flowbim.ee house (Revit 2026) and hemanth1986's house and office (Revit 2026 and 2027), counts only.
**Scorers:** `tools/re/bodies_across_saves.py` and `tools/re/readers_across_saves.py` (the gate, section 4), `tools/re/save_pair_bodies.py` (RE-341), `tools/re/save_pair_props.py` and `tools/re/save_pair_ports.py` (puzzbobb's), and for the held-out models the W7 census `tools/re/psets_vs_ifc.py`, `qtos_vs_ifc.py` and `names_vs_ifc.py`.
**Status:** positive against the same samples saved by Revit 2025, with two values left out where Revit's own 2026 and 2027 exports disagree: the Area of a door or window on 2027, and the ObjectType of a stair, a flight and a stringer. **No Revit 2026 or 2027 IFC export of an Autodesk sample is available**, so nothing about those samples is measured against Revit's own export. The oracle is the same model saved by Revit 2025, whose export is measured against Revit's (#869, RE-131, RE-193, RE-215, RE-231, RE-254, RE-255, RE-296, #323). It cannot catch an error the 2025 save shares, nor a change in how Revit's exporter writes a value (section 5). **The unfiltered Measure run of the shipped head, with `holdout=true`, is not yet made**: section 3 is Measure run 42063046730 on the first head (3149973f, before #1452 and before the narrowings), and sections 4 and 5 describe what the gate and what is left out do on this one. Measure run 42119142486 (head 95fe042f, the same-project gates only, every 2024 to 2027 sample of the five projects) passed the gate as it ships, 15 samples and the Areas on 4 saves, and the 2026 and 2027 numbers below are what it printed on the IFC files it kept.

## 1. Before

After RE-341, ten more readers are still limited to 2024 and 2025 (one of them to 2024 to 2026) by their release lists, so on 2026 and 2027 a pipe is its record box, a family instance's ports are left unplaced, a wall is not cut by its doors' and windows' outlines, and these properties are not written:

| reader | release list before | what it gives |
|---|---|---|
| a family instance's unjoined ports (`PORT_PLACEMENT_REVIT_VERSIONS`, #869) | 2024, 2025 | each `IfcDistributionPort`'s placement |
| a pipe's axis (`partition_pipe_axes`, RE-131) | 2024, 2025 | `IfcPipeSegment` as its cylinder, its `Qto_PipeSegmentBaseQuantities` |
| an MEP curve's fields (`partition_curve_fields`) | 2025 | duct `Length`, pipe `InvertElevation` |
| a door's or window's cut loop (`CUT_LOOP_REVIT_VERSIONS`, RE-215) | 2024, 2025 | the wall cut to the opening's outline, and its `Qto_DoorBaseQuantities` and `Qto_WindowBaseQuantities` |
| `IsExternal`'s last step (`IS_EXTERNAL_LAST_STEP_REVIT_VERSIONS`, RE-193) | 2024, 2025 | `IsExternal` in each common property set |
| a room's height (`ROOM_HEIGHT_REVIT_VERSIONS`, RE-255) | 2024 to 2026 | `Qto_SpaceBaseQuantities.Height` |
| a room as a prism (`ROOM_PRISM_REVIT_VERSIONS`, RE-296) | 2024, 2025 | `Qto_SpaceBaseQuantities.NetVolume` |
| a railing type's top rail height (`RAILING_HEIGHT_REVIT_VERSIONS`, RE-231) | 2024, 2025 | `Pset_RailingCommon.Height` |
| a stair railing's length (`RAILING_LENGTH_REVIT_VERSIONS`, RE-254) | 2024, 2025 | `Qto_RailingBaseQuantities.Length` |
| a stair's and its parts' names (`NATIVE_STAIR_NAME_REVIT_VERSIONS`, #323) | 2025 | the stair's, its runs' and stringers' `Name`, family and type names |

## 2. The change

The ten lists gain 2026 and 2027 (room height, 2027). The records these readers read are the 2025 ones there (RE-124, RE-179). Two things differ from the contributed change, both from Revit's own exports (section 5): a door's or window's `Area` is left out on 2027 (`DOOR_WINDOW_AREA_REVIT_VERSIONS`), and the `ObjectType` of a stair, a flight and a stringer is left out from 2026 (`STAIR_OBJECT_TYPE_LEFT_OUT_FROM`, #1472). Their type and family are read on 2025 to 2027 as the contributor wrote, which gives the parts their Names, their type objects and the type-derived property sets.

Three more lists were tried by the contributor and left alone, since widening them changes no byte of any 2026 export they held: a wall type's face angles (`WALL_FACE_ANGLES_SUPPORTED_REVIT_VERSIONS`), a sketch's splines (`SKETCH_SPLINE_SUPPORTED_REVIT_VERSIONS`) and connector pairs (`CONNECTOR_PAIR_SUPPORTED_REVIT_VERSIONS`). Nothing measures them on 2026 or 2027.

#1438's hand and facing flips (RE-343) were already read on 2026 and 2027, by a pass of their own, because the door and window cut pass (`attach_cut_loop_sizes`, which gives them on 2024 and 2025) stopped at 2025. With the list widened that pass gives them there too, so each 2026 and 2027 door and window would have carried them twice and the file been read once more: the pass of their own is dropped (`attach_opening_flips`). The flips are decoded fields, not in the IFC, so no gate sees them; the cut pass reads them as it does on 2024 and 2025, where RE-343 measured it, and on 2026 and 2027 that has not been measured.

The contributor's table of the 2026 samples each reader changes (leaving one out at a time, on `main` 182ed219, before RE-341):

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

## 3. The 2026 saves against their 2025 saves

Measure run 42063046730 (head 3149973f, base 0c64196a) exported `rac_advanced`, `rac_basic`, `rme_advanced`, `rme_basic` and `rst_basic` of 2025 and 2026 on each side and kept the IFC files. The figures below are the gate (`bodies_across_saves.py` with `readers_across_saves.py`, at this branch's commit) run on the 2026 and 2025 exports of the head side, and, for what the change moves, `base` against `head` against the 2025 save. That run predates #1452 (RE-345), which makes the 2025 saves draw beams, tread landings and sketch arcs: so the beams, landings, floor and ceiling listed as differing below are, on the head that ships, expected to be equal, as RE-345's run 42091457764 (head 830a4d94) found for the outlines, and **the Measure run of the shipped head has not been made**. 2027 was exported with these lists by Measure runs 42107917715 (head f1460b32) and 42119142486 (head 95fe042f), and the 2027 figures below are the gate's own output on the files the second kept.

**What the change moves, 2026** (the head export against the base export, element by element by `Tag`): `rme_advanced` 454 pipes and 36 walls, `rme_basic` 457 pipes and 36 walls, `rac_advanced` 46 walls, `rac_basic` 11 walls, `rst_basic` none, every moved pipe's `BodySource` (partition_element_record_bbox becomes partition_pipe_axis); ports placed 0 to 6,959, 7,046, 30 and 60 (`rst_basic` has none); cut openings 0 to 107, 107, 108 and 25 (one more opening beside a cut one on `rac_basic`, a hole its door cuts in a wall); `OverallWidth` and `OverallHeight` of 41, 17, 17 and 23 doors and windows (`rac_advanced`, `rme_advanced`, `rme_basic`, `rac_basic`), the Depth of the quantities of 107, 107, 107 and 23 openings (in that order), the type object of 22 doors (`rac_advanced`) and 1 (`rac_basic`), and the properties and quantities of section 4. Every one of them is the 2025 save's at head. The walls are of two kinds: the hosts of the cut openings (36, 36, 34 and 11) and, on `rac_advanced`, 12 walls that held 1 or 2 openings at base and hold none at head or in the 2025 save: the same pass binds each door and window to the wall its native record names (RE-245), which moves openings off the walls they were set in by nearest distance.

**Bodies** against the 2025 save (world box to 3 mm, volume to 0.1%, `save_pair_bodies.py`'s census, in the gate), base then head, on that run:

| sample, 2026 | differ at base | differ at head | the change |
|---|---:|---:|---|
| `rme_advanced` | 490 of 6,605 | 0 | 454 pipes and 36 walls become the 2025 save's |
| `rme_basic` | 493 of 6,650 | 0 | 457 pipes and 36 walls |
| `rac_advanced` | 56 of 5,527 | 10 | 46 walls |
| `rac_basic` | 22 of 438 | 17 | 5 walls; 6 more are moved and still differ (below) |
| `rst_basic` | 32 of 453 | 32 | none |

These are puzzbobb's figures, reproduced on our runners' exports of 2026.

**What still differs, and what the gate admits** (Griffin, 2026-11-05: the list can only shrink, and no entry is ever added; a new difference on 2027 or anywhere else is his to decide):

| sample | differing at head, 2026, before #1452 | by category | what the gate admits |
|---|---:|---|---|
| `rac_advanced` | 10 | 5 tread landings, 2 beams, a ceiling and a floor, which #1452 makes the 2025 save draw as the 2024 save does; a roof whose slope the 2025 save draws and 2026 does not (RE-341 keeps roofs boxes) | the 9 become equal and need no entry; the roof is counted as left as a record box, no list |
| `rac_basic` | 17 | 16 walls (12 `partition_wall_centreline_layer_joins`, 4 `partition_element_record_join_trimmed`, the same source in both saves, their extents differing by up to 3.19 m) and a roof (a box here, `partition_roof_extrusion` in the 2025 save) | the roof as above; the 16 walls pinned (below) |
| `rst_basic` | 32 | 32 beams drawn along their axes (RE-341), boxes in the 2025 save until #1452 | become equal; nothing admitted |
| `rme_advanced`, `rme_basic` | 0 | none | nothing admitted |

None of the 10, 17 and 32 is drawn by this change: each is as on base, except the 6 walls below. The 16 walls are `PINNED_WALLS` in `readers_across_saves.py`: each is pinned, for each release, to its source pair, the volume of each body and the world box of each body (to 1e-6 m and m3), so a wall whose source, volume or box moves fails, and one that heals fails too, which forces its pin out. #1471 is the lane fixing them. The 2027 pins equal the 2026 pins wall for wall, source, volumes and boxes, measured on the IFC files of run 42107917715 and held by the gate of run 42119142486 (16 of 16 on both releases). No release holds more than 16, and none that is not one of these 16: `tools/ci/check_same_project_projects.py` holds the 16 tags in CI, so an entry cannot be added without changing that file too. The roofs a save leaves as record boxes where the 2025 save draws them are counted exactly: `rac_advanced`'s 201262 and `rac_basic`'s 243274, one each on 2026 and 2027 and none on any other sample; one more or one fewer fails.

Six of the 16 walls (428588, 428745, 429964, 430064, 430318 and 430412) host cut openings and are the walls this change moves that still differ. Their own bodies, without their openings, differ from the 2025 save's by 0.56 to 2.35 m3 (12.0388 against 12.9987 m3 on the first), exactly as on base. What the change moves is what their openings take out of them: on all six that volume is now exactly the 2025 save's (2.4543, 3.2724, 0.2016, 1.6153, 0.2016 and 0.2016 m3 on head against 2.5663, 3.4903, 0.2511, 1.7143, 0.2511 and 0.2511 on base), so the corrected cuts land and the walls still differ by what they differed by before. The gate fails on a cut that takes out another volume, wherever the host itself differs.

**Ports** (`readers_across_saves.py`: origin to 3 mm, axis to 1e-3, paired by the host's `Tag`, `Name` and `FlowDirection`): at base none of a 2026 save is placed; at head every one is the 2025 save's: 6,959 on `rme_advanced`, 7,046 on `rme_basic`, 30 on `rac_advanced` and 60 on `rac_basic` (28 ports of `rme_basic` and 27 of `rme_advanced` have no placement in either save).

**Properties and quantities** (floats to four places): at head every value of the names the release lists add is equal to the 2025 save's, with none differing and none written that the 2025 save does not write: 5,015 (`rme_advanced`), 5,020 (`rme_basic`), 4,448 (`rac_advanced`), 329 (`rac_basic`) and 413 (`rst_basic`) values, and 107, 107, 107, 13 and 0 doors' and windows' Areas. What 2025 writes and 2026 does not, among the names the lists add: `IsExternal` on walls, curtain walls, members and plates (on `rme_basic`, 135, 31, 1,450 and 628), which RE-346 does not change.

**Door and window sizes** (`OverallWidth`, `OverallHeight` and the type object's GlobalId, class and Name): 124, 33, 124, 124 and 0 compared on `rac_advanced`, `rac_basic`, `rme_advanced`, `rme_basic` and `rst_basic`, none differing.

**2027 against 2026** (run 42119142486, head 95fe042f, the gate's own output on its kept files; run 42107917715 gave the same but for the stairs' values, which the gate did not score then): the ports, cut openings, door and window sizes, properties and Areas of the five projects are the same numbers on 2027 as on 2026, and the 16 pinned walls are the same walls with the same sources, volumes and boxes. Two things differ: (1) a door's or window's Area, which 2027 leaves out by design; (2) two doors of `rac_basic`, 431064 and 485679, write no `Qto_DoorBaseQuantities` Width and Height on 2027 where the 2025 and 2026 saves write 0.8 and 2.1: 9 of its 11 doors, 4 values fewer (414 properties equal on 2027 against 418 on 2026). Both doors keep `OverallWidth` 0.8 and `OverallHeight` 2.1 on every save, so their cut loops are read; the quantities are written where the symbol that draws the instance and the instance's type hold the same Width and Height, so on 2027 that pair is not read for these two, or it differs. Which is not known from the bytes, and is #1489 (it may be the kind of gap #1475 is, a native row a 2027 save does not return). The gate holds `rac_basic`'s properties per release for that reason (418 on 2026, 414 on 2027). A stair's, its flights' type objects and type-derived values (`NosingLength`, `NumberOfRiser`, `NumberOfTreads`, `Reference` and the rest) are equal to the 2025 save's on every stair and flight of `rac_advanced` (5 and 10) and `rac_basic` (3 and 3), none differing and none only here. Also differing on 2027 and not scored, since the lists do not add them: 21 of `rac_basic`'s 144 members' `LevelElementId` (123 equal) and one `rac_advanced` curtain wall's `Qto_CurtainWallQuantities.Length`, which the 2025 save writes and the 2027 save does not (33 equal); whether `main` writes them the same was not measured here.

## 4. The gate

Griffin's decision (2026-11-05): a body or category on a release with no Revit export is held per element to the same project's earlier save before it lands, anything that differs stays a box, and a difference blocks landing. The contributed change was not seen by the existing gate: it held `rac_advanced`, `rac_basic` and `rst_basic` only, and only their beams, landings, stair runs and sketch outlines, and the PR's largest effect is on `rme_advanced` and `rme_basic`. The gate now also holds, for each 2026 and 2027 save of the five projects, against the 2025 save:

| check | what it compares | fails on | least it compares |
|---|---|---|---|
| pipes | each `IfcPipeSegment` drawn as `partition_pipe_axis`: class, GlobalId, storey, `BodySource`, representation, placement, volume, and the cylinder itself where the pipe is a circular extrusion in both saves (the two ends of its axis in either order, its radius and its length, each to 0.001 ft), else the vertices (the RE-341 group) | any difference, a pipe the 2025 save does not draw so | 454 (`rme_advanced`), 457 (`rme_basic`), each compared as a cylinder |
| cut openings | each opening that carries `Qto_OpeningElementBaseQuantities` (the extrusion of a cut loop, RE-244), and each opening that shares its Tag, with the same fields, GlobalId excepted (derived from the export, it differs between two saves of one door), its host's Tag and its Depth; and each host with its openings and without them | any difference; a host the 2025 save's without its openings and not with them; a host whose openings take out another volume than the 2025 save's; a cut the 2025 save has and this save lacks | 108, 25, 107, 107 (`rac_advanced`, `rac_basic`, `rme_advanced`, `rme_basic`) |
| door and window sizes | `OverallWidth`, `OverallHeight` and the type object's GlobalId, class and Name | any difference | 124, 33, 124, 124 |
| ports | each placed `IfcDistributionPort` | one that differs, one only this save places, one only the 2025 save places | 30, 60, 6,959, 7,046 |
| properties | the names the release lists add (`ADDED` in the script, a stair's and flight's type-derived property sets and names too), element by `Tag`, rooms by GlobalId | a value that differs, or that this save writes and the 2025 save does not; a door's or window's Area on a save of 2027 or later | 4,733, 418 (2026) and 414 (2027), 5,023, 5,028, 451 values; Areas 107, 13, 107, 107 |
| census | every body of the save with the 2025 save's, box and volume | a body that differs, unless it is one of the roofs left as record boxes (exactly those, counted) or one of the 16 pinned walls, which must differ exactly as pinned in source, volume and box | |

Each prints how many it compared, and the run summary of Measure lists them under each sample. A check whose least number is not reached fails, so one that compares nothing is never a pass; `rst_basic` holds no pipe, port or cut opening, and its least numbers there are 0. The 2025 save of the project must be in the run: otherwise the sample reads `PARTIAL`, never `PASS`. The checks run on `rme_advanced` and `rme_basic` as well, so those samples are now in the same-project verdict (`tools/ci/same_project_verdict.py`) and the check script, and a 2025 save is held to its 2024 save alone for the beams, landings and outlines (RE-345), which the MEP samples have never been through.

The MEP samples exposed two weaknesses of the gate that RE-341 and RE-345 had no sample to show. A floor Revit writes as several products under one Tag (`rme_advanced`'s slabs 577552, 577724 and the like: 6 under one Tag, in one storey and at one height) carries a GlobalId derived from the export on all but the first, which differs between two saves of one project; pairing a Tag's products by GlobalId crossed them, and comparing the derived GlobalIds failed every one. The gate now pairs the products of a Tag by storey, then the volume and the centre of the body, and does not compare the GlobalId of a Tag that holds several, except that the two saves must share exactly one GlobalId under such a Tag, and that one must be on the two products paired by what they are: an identity swapped between two pieces, or none kept, fails (every Tag of the MEP samples holds exactly one). With that, the 78 sketch outlines of each MEP sample (42 `IfcCovering`, 36 `IfcSlab`) of 2025, 2026 and 2027 are equal to their 2024 saves', scored with the gate on the IFC files that run 42091457764 (head 830a4d94, #1452's) kept, whose 2026 and 2027 outlines this change does not move; the 2025 samples of `rac_advanced`, `rac_basic` and `rst_basic` still pass as RE-345 found.

What the gate covers was checked by comparing the base and head exports of run 42063046730 for 2026: the bodies that moved (pipes, hosts, 12 walls that lost openings), the ports, the openings, the doors' and windows' sizes and types, the openings' quantities and the property values that appeared or changed are each in a check above, and the counts of IFC entities by class differ between the two only in those classes. The stair types that head wrote (`IfcStairType`, `IfcStairFlightType`, one more `IfcMemberType`) are written again; the one value of them left out from 2026 is the `ObjectType` of a stair, a flight and a stringer (section 5). Anything a later change moves that no check names, the gate does not see: a name `ADDED` does not cover needs adding there.

The minimums are the numbers run 42119142486 printed on the 2026 and 2027 saves, the same on both except `rac_basic`'s properties (418 on 2026, 414 on 2027, #1489); the stairs' type-derived values are among them. `tools/ci/check_same_project_projects.py` (a CI step) fails when the verdict, the check script, the measuring script's two lists or either scorer's `MINIMUMS` name different samples.

**The doors and windows group of #1456 (RE-347)** holds each door and window placed on Revit's axes by the placement rule to the 2024 save, per element, with a floor per class and a ceiling on those the later save leaves on the default axes. #1456 set it on `rac_advanced` and `rac_basic` while the cut pass, which reads a door's flips and rebinds its host (RE-245), ran on 2024 and 2025 only: it left 28 elements of `rac_advanced`'s 2026 save and 42 of its 2027 save on the default axes, 28 of them for that reason and 14 on 2027 because 8 host walls return no native straight-line row (#1475). This change widens that pass to 2026 and 2027, which removes a pass of its own that read the flips there, so the meaning of those floors and ceilings changes: they were what the readers of the day placed, and are now what the 2024 save places. They were set for the merged head from that diagnosis, and Measure run 42124273032 (head ac922cd9) counted: `rac_advanced` 2026 83 doors and 24 windows compared, none left out, as predicted; 2027 68 doors and 24 windows compared and 15 doors left out, all in 8 host walls, 14 of them with a frame that differs from the 2024 save's and one with the same frame, so the ceiling is 15 and the doors floor 68, where 14 and 69 were predicted. `rme_advanced` and `rme_basic` join the group, as #1456 said they would: 107 placed doors per file (59 with unequal flips), a floor of 107 doors and a ceiling of 0 for each of 2025, 2026 and 2027, windows not floored until a run counts them. `rme_advanced` and `rme_basic` compared 107 doors and left none out on each of 2025, 2026 and 2027, 75 of them off the default axes, as predicted. A 2026 or 2027 door whose flips no longer read keeps the default axes, is counted as left out and fails the ceiling.

## 5. Where Revit's own exports disagree

The held-out models are the only 2026 and 2027 files with a Revit export: the flowbim.ee house (Revit 2026, its 26.4 IFC4X3 export) and hemanth1986's house and office, saved and exported by each of Revit 2026 and 2027 (IFC4 Reference View). Counts only. Measure run 42062972018 (head 3149973f, base 0c64196a), `psets_vs_ifc`, `qtos_vs_ifc` and `names_vs_ifc`, base then head:

| model | values equal | values different | quantities equal | quantities different |
|---|---:|---:|---:|---:|
| flowbim house, 2026 | 378 to 396 | 0 to 0 | 135 to 200 | 0 to 0 |
| hemanth house, 2026 | 224 to 289 | 0 to 0 | 23 to 89 | 1 to 1 |
| hemanth house, 2027 | 224 to 289 | 0 to 0 | 19 to 74 | 1 to **16** |
| hemanth office, 2026 | 186 to 230 | 0 to 0 | 14 to 74 | 3 to 3 |
| hemanth office, 2027 | 186 to 230 | 0 to 0 | 10 to 60 | 3 to **17** |

Values Revit does not write: 0 to 0 on every model. No held-out model holds a pipe, a duct or a port, so the pipes, ports, duct and pipe quantities are measured only against the 2025 saves. Two things the contributed change wrote were not Revit's:

- **A door's or window's Area on 2027.** On both 2027 models the Width and Height equal Revit's (29 doors and windows in all) and the Area differs on every one (rvt-rs over Revit 0.2558 on every window of both models, 0.3012 and 0.3879 on the doors): Revit 2027's export does not write Area as Width times Height, which its 2026 export and rvt-rs on 2024 to 2026 do (equal on every door and window of the three 2026 models). Same-project checking could not see this: both saves of an Autodesk sample read the same Area. Area is not written on 2027 (`DOOR_WINDOW_AREA_REVIT_VERSIONS`, 2024 to 2026), with Width and Height as they were.
- **A stair's ObjectType on 2026 and 2027.** The stair, its flight and its two stringers on the 2026 and 2027 houses went from Name different and ObjectType equal to Name equal and ObjectType different: the `Family:Type` rvt-rs writes as ObjectType is not what Revit's 2026 and 2027 exports write on any class with a type in these models (48 proxies, 12 windows, 10 walls, 4 furniture, 3 doors and the rest measure ObjectType different on `main` already), where on the 2025 pair every class is equal. What Revit's 2026 and 2027 exports write instead is not in a count. rvt-rs writes `Family:Type` on 2026 and 2027 whenever it has the names, so it differs on every class there; that predates this change, belongs to every class and is #1472. Here the stair, its flights and its stringers keep their family and type on 2026 and 2027 as on 2025 (an earlier head of this change read them on 2025 only, which cost two monolithic flights of `rac_basic` their Names, found by run 42107917715), and only their `ObjectType` is left out from 2026 (`STAIR_OBJECT_TYPE_LEFT_OUT_FROM`), as it was on `main`: `$`. That gives them back their Names and the type-derived property sets (`Pset_StairCommon.NosingLength`, `NumberOfTreads` and `Reference`, `Pset_StairFlightCommon.Reference`); each of those is kept only where it measures equal to Revit's on the held-out pairs, which the Measure run of this head with `holdout=true` decides, and left out where it does not (not yet measured).

The rest of the held-out diff: all new values equal, none different, on every model.

## 6. Nothing else moves

Contributor's local run, `SOURCE_DATE_EPOCH=1700000000`, 120 files exported with the RE-341 head (c8fb8d19) and with their change: 92 are byte for byte the same, IFC and diagnostics: every 2015 to 2025 file there, Autodesk's samples of 2015, 2016 and 2019 to 2025, Core Interior, the four RE1 models, Einhoven, the BIM4LCA office and terrain housing, and puzzbobb's Revit 2025 models. The 28 that change are 2026 and 2027 saves of Autodesk's samples. That is theirs, on their machine; the unfiltered Measure run of this head, which scores the same on our runners, is not yet made.

## 7. Not measured

- Revit 2026 and 2027 Autodesk samples against Revit's own export: none is available.
- Pipes, ducts and ports against any Revit export of a 2026 or 2027 file: no held-out model holds one.
- The unfiltered Measure run of this head with `holdout=true` (every output change, the held-out scorers): not made. Run 42119142486 held the 2026 and 2027 saves of the five projects to the gate as it ships, and passed.
- The stairs' type objects (`IfcStairType`, `IfcStairFlightType` and a stringer's `IfcMemberType`, which `main` wrote on no 2026 or 2027 save) and their type-derived property sets against Revit's 2026 and 2027 exports (flowbim's and hemanth1986's pairs), which `holdout=true` on this head decides.
- A stringer's type-derived values (`Pset_MemberCommon.Reference` and the like) on 2026 and 2027: the gate scores a stair's and a flight's, and a stringer's `Name`, not these, since `IfcMember` also holds 144 non-stair members on `rac_basic`.
- #1476 draws `rac_basic`'s roof 243274: when it lands, it removes `rac_basic` from `ROOFS_LEFT_AS_BOXES` as well as emptying `PINNED_WALLS` and the exception in `docs/maintaining.md`.
- Why two doors of `rac_basic`'s 2027 save write no Width and Height (#1489).
- The sketch outlines, beams and landings of `rme_advanced` and `rme_basic` 2025 to 2027 against their 2024 saves, which the gate holds for the first time here.
- #1438's hand and facing flips on 2026 and 2027, now read by the cut pass instead of a pass of their own: they are not in the IFC, so nothing here compares them, and RE-343 measured that pass on 2024 and 2025.
- `IsExternal` on walls, curtain walls, members and plates on 2026 and 2027: 2025 writes it, 2026 and 2027 do not, on either head. Some other release list or rule gates it.
- A wall type's face angles, sketch splines and connector pairs on 2026 and 2027: no sample here changes, so they keep their lists.
- 2015 to 2023: these readers stay at 2024 and later.
- A same-project check cannot catch an error the 2025 save shares, nor a value Revit's exporter writes differently on 2026 or 2027 (section 5).

## 8. Reproduce

```bash
tools/fetch-autodesk-samples.sh samples 2025 2027
for f in samples/*.rvt; do rvt-ifc "$f" -o "${f%.rvt}.ifc"; done
python3 tools/re/save_pair_bodies.py samples/2026-rmeadvancedsampleproject.ifc samples/2025-rmeadvancedsampleproject.ifc
python3 tools/re/save_pair_ports.py samples/2026-rmeadvancedsampleproject.ifc samples/2025-rmeadvancedsampleproject.ifc
python3 tools/re/save_pair_props.py samples/2026-racadvancedsampleproject.ifc samples/2025-racadvancedsampleproject.ifc
```

The gate is Measure's: `gh workflow run measure.yml --ref main -f ref=<head> -f samples=true -f sample_years="2024 2027" -f models="-" -f scorers=bodies_across_saves -f base=none`, which writes the `same-project gates` verdict on the measured commit.
