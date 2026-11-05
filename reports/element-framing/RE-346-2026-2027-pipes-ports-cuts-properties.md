# RE-346: Revit 2026 and 2027 pipes, ports, door and window cuts, and the properties read on 2025

**Date:** 2026-11-05
**Issues:** #1190 (each capability 2024 and 2025 have, on every release from 2015 to 2027); follows RE-341 (beams, stair runs, landings and sketch lines on 2026 and 2027), RE-124 and RE-179 (2026 and 2027 element records are 2025's). Contributed by puzzbobb (#1454); the gate and the two narrowings in section 5 are the maintainer's.
**Artefacts:** Autodesk's own sample projects, pinned in `research/autodesk-sample-projects.tsv` and fetched by `tools/fetch-autodesk-samples.sh`, never stored: `rme_advanced` 2025 (`08457292…886d5`), 2026 (`1f34a8f1…5d7af`) and 2027 (`899ea8a6…fca7a`); `rme_basic` 2025 (`037165e5…c8099`), 2026 (`37127cf5…ead94`) and 2027 (`886bb0d1…ae1a7`); `rac_advanced` 2025 (`512e9188…e25d1`), 2026 (`f5fb3712…83ace`) and 2027 (`a88eb9f8…024ed`); `rac_basic` 2025 (`41f1893a…d4293`), 2026 (`bc547fe9…d99dc`) and 2027 (`c9ce4d01…91a01`); `rst_basic` 2025 (`5089b2c6…31acb`), 2026 (`e7cf23a9…75418`) and 2027 (`01685786…d1453`). Held-out: the flowbim.ee house (Revit 2026) and hemanth1986's house and office (Revit 2026 and 2027), counts only.
**Scorers:** `tools/re/bodies_across_saves.py` and `tools/re/readers_across_saves.py` (the gate, section 4), `tools/re/save_pair_bodies.py` (RE-341), `tools/re/save_pair_props.py` and `tools/re/save_pair_ports.py` (puzzbobb's), and for the held-out models the W7 census `tools/re/psets_vs_ifc.py`, `qtos_vs_ifc.py` and `names_vs_ifc.py`.
**Status:** positive against the same samples saved by Revit 2025, with two readers narrowed where Revit's own 2026 and 2027 exports disagree. **No Revit 2026 or 2027 IFC export of an Autodesk sample is available**, so nothing about those samples is measured against Revit's own export. The oracle is the same model saved by Revit 2025, whose export is measured against Revit's (#869, RE-131, RE-193, RE-215, RE-231, RE-254, RE-255, RE-296, #323). It cannot catch an error the 2025 save shares, nor a change in how Revit's exporter writes a value (section 5). **The Measure run that scores this head is not yet made**: section 3 is Measure run 42063046730 on the first head (3149973f), and sections 4 and 5 describe what the gate and the narrowings do on this one.

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

The ten lists gain 2026 and 2027 (room height, 2027). The records these readers read are the 2025 ones there (RE-124, RE-179). Two things differ from the contributed change, both from Revit's own exports (section 5): a door's or window's `Area` is left out on 2027 (`DOOR_WINDOW_AREA_REVIT_VERSIONS`), and a stair's and its parts' type and family are read on 2025 only (`NATIVE_STAIR_TYPE_REVIT_VERSIONS`), their names on 2025 to 2027.

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

Measure run 42063046730 (head 3149973f, base 0c64196a) exported `rac_advanced`, `rac_basic`, `rme_advanced`, `rme_basic` and `rst_basic` of 2025 and 2026 on each side and kept the IFC files. The figures below are the gate (`bodies_across_saves.py` with `readers_across_saves.py`, at this branch's commit) run on the 2026 and 2025 exports of the head side, and, for what the change moves, `base` against `head` against the 2025 save. 2027 was not exported in that run: the 2027 figures here are puzzbobb's local ones, labelled.

**What the change moves, 2026** (the head export against the base export, element by element by `Tag`; every one of the moved things is in a check of section 4): `rme_advanced` 454 pipes and 36 walls, `rme_basic` 457 pipes and 36 walls, `rac_advanced` 46 walls, `rac_basic` 11 walls, `rst_basic` none, every moved pipe's `BodySource` (partition_element_record_bbox becomes partition_pipe_axis); ports placed 0 to 6,959, 7,046, 30 and 60 (`rst_basic` has none); cut openings 0 to 107, 107, 108 and 25, and the properties and quantities of section 4. Nothing else changed in a body. The walls are of two kinds: the hosts of the cut openings (36, 36, 34 and 11) and, on `rac_advanced`, 12 walls that held 1 or 2 openings at base and hold none at head or in the 2025 save: the same pass binds each door and window to the wall its native record names (RE-245), which moves openings off the walls they were set in by nearest distance. The census of section 4 holds those 12.

**Bodies** against the 2025 save (world box to 3 mm, volume to 0.1%, `save_pair_bodies.py`'s census, in the gate), base then head:

| sample, 2026 | differ at base | differ at head | the change |
|---|---:|---:|---|
| `rme_advanced` | 490 of 6,605 | 0 | 454 pipes and 36 walls become the 2025 save's |
| `rme_basic` | 493 of 6,650 | 0 | 457 pipes and 36 walls |
| `rac_advanced` | 56 of 5,527 | 10 | 46 walls |
| `rac_basic` | 22 of 438 | 17 | 5 walls; 6 more are moved and still differ (below) |
| `rst_basic` | 32 of 453 | 32 | none |

These are puzzbobb's figures, reproduced on our runners' exports of 2026; they hold on 2027 as the contributor measured them locally and as Measure has not yet scored.

**What the 2026 bodies that still differ are** (head, 2026; 2027 is as the contributor measured it, "alike"):

| sample | differing | by category | drawn by this change |
|---|---:|---|---|
| `rac_advanced` | 10 | 5 tread landings and 2 beams that RE-341 draws and the 2025 save leaves as boxes (held to the 2024 save there), a roof whose slope the 2025 save draws and 2026 does not (RE-341 keeps roofs boxes), a ceiling and a floor that both saves draw as boxes with different volumes (RE-341's named elements) | none: all 10 are as on base |
| `rac_basic` | 17 | 16 walls (12 `partition_wall_centreline_layer_joins`, 4 `partition_element_record_join_trimmed`, the same source on both saves, their extents differing by up to 3.19 m) and a roof (box here, `partition_roof_extrusion` in the 2025 save) | 6 of the 16 walls are moved by the change, below; the other 10 walls and the roof are as on base |
| `rst_basic` | 32 | 32 beams drawn along their axes (RE-341), boxes in the 2025 save | none: as on base |

The 6 `rac_basic` walls the change moves each host cut openings, and each differs from the 2025 save without its openings too (its whole body, with the openings left out, is 12.0388 against 12.9987 m3 on the first, and the same on base). What the change moves is what its openings take out of it: on all six, the volume the openings cut is exactly the 2025 save's (2.4543, 3.2724, 0.2016, 1.6153, 0.2016 and 0.2016 m3 on head against 2.5663, 3.4903, 0.2511, 1.7143, 0.2511 and 0.2511 on base). The difference of these six walls from the 2025 save is the wall's own, not the cut's. Whether a wall already unlike the 2025 save should also keep its old cut is the maintainer's call; the gate fails on a cut that takes out another volume, and prints these.

**Ports** (`readers_across_saves.py`: origin to 3 mm, axis to 1e-3, paired by the host's `Tag`, `Name` and `FlowDirection`): at base none of a 2026 save is placed; at head every one is the 2025 save's: 6,959 on `rme_advanced`, 7,046 on `rme_basic`, 30 on `rac_advanced` and 60 on `rac_basic` (28 ports of `rme_basic` and 27 of `rme_advanced` have no placement in either save).

**Properties and quantities** (floats to four places): at head every value of the names the release lists add is equal to the 2025 save's, with none differing and none written that the 2025 save does not write: 5,015 (`rme_advanced`), 5,020 (`rme_basic`), 4,448 (`rac_advanced`), 329 (`rac_basic`) and 413 (`rst_basic`) values, and 107, 107, 107, 13 and 0 doors' and windows' Areas. What 2025 writes and 2026 does not, among the names the lists add: `IsExternal` on walls, curtain walls, members and plates (on `rme_basic`, 135, 31, 1,450 and 628), which RE-346 does not change.

## 4. The gate

Griffin's decision (2026-11-05): a body or category on a release with no Revit export is held per element to the same project's earlier save before it lands, anything that differs stays a box, and a difference blocks landing. The contributed change was not seen by the existing gate: it held `rac_advanced`, `rac_basic` and `rst_basic` only, and only their beams, landings, stair runs and sketch outlines, and the PR's largest effect is on `rme_advanced` and `rme_basic`. The gate now also holds, for each 2026 and 2027 save of the five projects, against the 2025 save:

| check | what it compares | fails on | least it compares |
|---|---|---|---|
| pipes | each `IfcPipeSegment` drawn as `partition_pipe_axis`: class, GlobalId, storey, `BodySource`, representation, placement, volume and vertices (the RE-341 group) | any difference, a pipe the 2025 save does not draw so | 454 (`rme_advanced`), 457 (`rme_basic`) |
| cut openings | each opening that carries `Qto_OpeningElementBaseQuantities` (the extrusion of a cut loop, RE-244), with the same fields, GlobalId excepted (derived from the export, it differs between two saves of one door); and each host with its openings and without them | any difference; a host the 2025 save's without its openings and not with them; a host whose openings take out another volume than the 2025 save's | 108, 25, 107, 107 (`rac_advanced`, `rac_basic`, `rme_advanced`, `rme_basic`) |
| ports | each placed `IfcDistributionPort` | one that differs, one only this save places, one only the 2025 save places | 30, 60, 6,959, 7,046 |
| properties | the names the release lists add (`ADDED` in the script), element by `Tag`, rooms by GlobalId | a value that differs, or that this save writes and the 2025 save does not; a door's or window's Area on a 2027 save | 4,448, 329, 5,015, 5,020, 413 values; Areas 107, 13, 107, 107 |
| census | every body of the save with the 2025 save's, box and volume | a body that differs and is not one of the 59 known differences of section 3 (10, 17 and 32 on `rac_advanced`, `rac_basic` and `rst_basic`, none on the MEP samples), or has none in the 2025 save; a known one that no longer differs is counted, not a failure | |

Each prints how many it compared, and the run summary of Measure lists them under each sample. A check whose least number is not reached fails, so one that compares nothing is never a pass; `rst_basic` holds no pipe, port or cut opening, and its least numbers there are 0. The 2025 save of the project must be in the run: otherwise the sample reads `PARTIAL`, never `PASS`. The checks run on `rme_advanced` and `rme_basic` as well, so those samples are now in the same-project verdict (`tools/ci/same_project_verdict.py`) and the check script. A change to what the lists add (a name `ADDED` does not cover) still needs a name added there: the scorer compared against the base export moves nothing outside it for the 2026 samples (every added value is in `ADDED`, and every moved body in the pipes group or the cut hosts).

The minimums are from the 2026 saves (section 3); the 2027 saves are held to the same numbers, and were not exported in run 42063046730. They hold if the 2027 samples carry what the 2026 ones do, which the contributor's local figures say they do for bodies and ports.

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
- **A stair's family and type on 2026 and 2027.** The stair, its flight and its two stringers on the 2026 and 2027 houses went from Name different and ObjectType equal to Name equal and ObjectType different: the `Family:Type` rvt-rs writes as ObjectType is not what Revit's 2026 and 2027 exports write on any class with a type in these models (48 proxies, 12 windows, 10 walls, 4 furniture, 3 doors and the rest measure ObjectType different on `main` already), where on the 2025 pair every class is equal. What Revit's 2026 and 2027 exports write instead is not in a count. rvt-rs writes `Family:Type` on 2026 and 2027 whenever it has the names, so it differs on every class there; that predates this change and is left alone. The stairs' types are read on 2025 only (`NATIVE_STAIR_TYPE_REVIT_VERSIONS`), so their ObjectType stays as on `main`, `$`; their names are read on 2026 and 2027 and measure equal. This also leaves the type-derived stair property sets (`Pset_StairCommon.Reference`, `NosingLength` and the like) unwritten there, as on `main`.

The rest of the held-out diff: all new values equal, none different, on every model.

## 6. Nothing else moves

Contributor's local run, `SOURCE_DATE_EPOCH=1700000000`, 120 files exported with the RE-341 head (c8fb8d19) and with their change: 92 are byte for byte the same, IFC and diagnostics: every 2015 to 2025 file there, Autodesk's samples of 2015, 2016 and 2019 to 2025, Core Interior, the four RE1 models, Einhoven, the BIM4LCA office and terrain housing, and puzzbobb's Revit 2025 models. The 28 that change are 2026 and 2027 saves of Autodesk's samples. That is theirs, on their machine; the unfiltered Measure run of this head, which scores the same on our runners, is not yet made.

## 7. Not measured

- Revit 2026 and 2027 Autodesk samples against Revit's own export: none is available.
- Pipes, ducts and ports against any Revit export of a 2026 or 2027 file: no held-out model holds one.
- The 2027 saves with this gate and these minimums, and the figures of puzzbobb's section 3 on our runners for 2027.
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
