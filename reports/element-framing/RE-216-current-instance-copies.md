# RE-216: An instance whose data copies disagree takes its current copy's transform

**Date:** 2026-10-24
**Issues:** #227 (8 of Core Interior's doors keep their box opening), #548 (which copy of an element's record is current). Follows RE-87 (a family instance's transform), RE-145 (the native route takes the latest copy) and RE-215 §7 (a door's or window's opening from the outline its symbol cuts).
**Artefacts:**

| file | sha256 | licence |
|---|---|---|
| `2024_Core_Interior.rvt` (Revit 2024) | `c805df445d613b408e37337765572021265e3f5dfdc7d1fa53b22ba1600b8014` | MIT, magnetar-io/revit-test-datasets |
| `2024_Core_Interior_slim.ifc` (Revit's IFC4 export) | `bfdf36ffb0bb768f3409d818403990e64d4c262c6780603be87f8077387ad86d` | MIT |
| `ARK_NordicLCA_Office_Concrete_BuildingPermit.rvt` (Revit 2024) | `1c12c2058128315edbd65a8bf97f60da971aee8625f21233b8477d9c6165c1d1` | CC BY-SA 4.0, VTT (`research/bim4lca-measure.tsv`) |
| `ARK_NordicLCA_Office_Concrete_BuildingPermit_Revit.ifc` (Revit's IFC4 export) | `bc3bd3366a3c413ce74840300a590f59102b740118ef37898e906253b6b7c08d` | CC BY-SA 4.0 |

Measure fetches all of them and stores none.
**Probe:** `examples/probe_227_missing_transforms.rs`. **Scorer:** `tools/re/opening_copies_vs_ifc.py`.
**Runs:**
- [40136981922](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40136981922): the probe, on branch `re/227-missing-transforms`;
- [40137774555](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40137774555): each copy against Revit's opening, with the scorer;
- [40138226998](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40138226998): the partition `native_document` reads each instance's current record from;
- [40138537515](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40138537515): this change against `main` (c38a5b0d), with `-f r2025=true -f nordic=true`.

**Status:** positive on the measured copies.

## 1. What was missing

RE-215 §7 cuts a door's or window's opening along the outline its symbol cuts, through the instance's transform (RE-87). On Core Interior 130 of the 138 doors and windows with an outline had a transform; the other 8 doors kept their box opening, within 0.26 ft of Revit's. All 8 have element data. `scan_instance_transforms` read a transform from each of their copies and then dropped them, because an instance whose copies disagree was dropped.

## 2. The two copies

Each of the 8 doors (20820, 29877, 55758, 59338, 60219, 61100, 61981, 62862) has an element-data copy in `Partitions/46` and in `Partitions/59`. They are one door stacked on eight floors, at 46, 61, 76, 91, 106, 121, 136 and 151 ft. In both copies the rotation is the same (X axis along the model's -Y), and the origins differ only along the model's X axis: 48.0833 ft in `Partitions/46` and 48.0 ft in `Partitions/59`, one inch. That axis is the instance's Y, across the host wall, so the X and Z extents of the opening do not tell the copies apart; its band through the host does.

## 3. Which copy Revit drew

`tools/re/opening_copies_vs_ifc.py` moves Revit's `IfcOpeningElement` for each door into each copy's frame and compares it with the cut loop (X and Z) and with the loop's plane and its mirror (Y, the host's band):

| copy | X and Z within 0.001 ft | Y within 0.001 ft | largest difference |
|---|---:|---:|---:|
| `Partitions/46` | 8 | 0 | 0.0833 ft |
| `Partitions/59` | 8 | 8 | 0.0000 ft |

`native_document`, which reads an element's current record from the partition its stored revision routes to, reads all 8 from `Partitions/59`. So Revit's export drew the current copy. The 8 are one stacked pattern, so this is one case confirmed eight times, not eight independent ones; the general rule is RE-145's, where the route takes the latest copy on 1,518 of 1,522 ids whose copies differ, across Core Interior, Einhoven, the RE1 models and Autodesk's 2021 `rac_advanced`.

## 4. What rvt-rs does

`partition_instance_transforms::scan_instance_transforms` keeps every copy with the number of its partition. Where all of an instance's copies agree, it takes that transform, as before. Where they disagree, it takes the copy in the partition the instance's current record is in: the partition its stored revision routes to (`native_document::routed_partitions`, through `native_index::route_episode`), or, where that partition holds no copy, the latest one that does, as `native_document` reads records. An instance whose revision does not route, or whose current partition holds disagreeing copies of its own, is still dropped. Every caller of the scan takes the change: turned boxes and hosted turns, curtain axes, opening facings, window and door openings, beam sections, cut-loop openings and ports.

## 5. Measured

Measure run [40138537515](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40138537515), this change against `main`:

| model | before | after |
|---|---|---|
| Core Interior: doors whose opening is within 0.001 ft of Revit's (`opening_boxes_vs_ifc`) | 124 of 132 (8 within 0.26 ft) | 132 of 132 |
| Core Interior: openings within 0.01 ft (`openings_vs_ifc`) | 130 of 138, largest 0.25 ft | 138 of 138, largest 0.00 ft |
| BIM4LCA office at building permit: door type objects with Revit's GlobalId (`type_objects_vs_ifc`) | 43 differ | 33 differ |

The office's doors take their flip from their facing against the host (B72), which comes from the transform; 10 doors whose copies disagreed now have one, so their `Flipped` type GlobalId is Revit's. Every other model's IFC is byte-identical (Einhoven, the four RE1 models, the 23 R2025 models, the office as built), and all 31 models' diagnostics are the same. Core Interior's export gains 8 placements and 56 property values, so its two witness observations are regenerated.

## 6. What it does not cover

- **Holders.** The partitions holding a copy are the ones whose element data names the instance, not the native channel-101 records `native_document` counts; the fallback to the latest holder applies only where the routed partition holds no copy.
- **Releases.** The element-data header is measured on Revit 2024 to 2027 only (RE-87).
- **Evidence.** The opening check is one stacked pattern on one model, and the office's 10 doors are checked through their type GlobalIds only.

## 7. Reproduce

```bash
gh workflow run measure.yml -f ref=<branch> -f base=none \
  -f probe=probe_227_missing_transforms -f probe_scorer=opening_copies_vs_ifc
gh run download <run id>   # measure-head/core-interior/probe.txt and probe_score.txt
```
