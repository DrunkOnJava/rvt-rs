# RE-431: a housing railing's type is named by the latest copy of its type object

**Date:** 2026-11-11
**Issues:** #1193 (W7: the BIM4LCA models' remaining property values), #1636 (the rules rest on the BIM4LCA models); follows RE-153 (a railing names its type in its own data object), RE-145 and #807 (the latest partition's copy is the current one), RE-231 and RE-254 (the office's railing heights and lengths)
**Artefacts:** the BIM4LCA housing, concrete and timber, as built and at building permit, the two terrain housing files and the office as built (Revit 2024), and Revit's IFC4 exports of them
**Probe and scorer:** `probes/examples/probe_re431_railing_types.rs`, `tools/re/railing_types_vs_ifc.py`; the first look used `probes/examples/probe_re231_record_lengths.rs` and `tools/re/stored_lengths_vs_psets.py`
**Runs:** Measure [43044928311](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43044928311) (the first probe, on `main` at `8ec2ac98`), [43048938486](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43048938486) (the ids and names a railing's copies name) and [43050258474](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43050258474) (the name each copy of a type object holds in each partition, with three ways of choosing between them)
**Status:** Positive. The probe reads the cause on all 198 railings of the four large housing files; the change's Measure result is in section 5.

## 1. The question

In the property census of Measure run 43018018669, rvt-rs wrote `Pset_RailingCommon` on the BIM4LCA housing's railings with `IsExternal` alone: `Reference` and `Height` were missing on 237 railings each (56 and 56 in the timber housing as built and at permit, 43 and 43 in the concrete one, and the held-out and terrain files), and `Qto_RailingBaseQuantities.Length` on 318. On the office's 41 railings all three are written (RE-231, RE-254), through the railing's type: its name is `Reference`, and `StairsRailingAttr.m_topRailHeight` of the type's record is `Height`. What differs for the housing's?

## 2. What Revit's export says

Revit's IFC4 export of the housing has no `IfcRailingType` (0 in the timber housing as built): a railing's type is the second part of its Name (`Railing:KAI-7:1347781`), and `Pset_RailingCommon.Reference` is that name on all 56 railings of the timber housing and all 43 of the concrete one. `Pset_RailingCommon.Height` is the type's top rail height, 1219.2 mm on 39 railings and 1000 mm on 17 in the timber file, and 1219.2 mm on all 43 in the concrete one. So the oracle for a railing's type is its name.

## 3. What the probes found

**First probe** (run 43044928311). `probe_re231_record_lengths` prints, per railing, the type rvt-rs gives it and every number at the top level of the first 200 objects of its record and of its type's. The type is none on all 56 railings of the timber housing as built, all 56 at permit, all 43 of the concrete housing as built and all 43 at permit, and it is known on the office's 41 railings and the terrain housing files' 2 each. A housing railing's own record holds only `BaseRailing.m_placementOffset`, `m_registeredLocation`, `m_registeredLocationBackup` and `m_version`: no height. So the height and the name wait on the type.

**What types a railing.** On Revit 2024 and later a railing whose reference list names no type definition is given the one ElementId among the words of its `BaseRailing` data object that is the id of a `StairsRailingAttr` object (`railing_type_picks`, RE-153), and its type is attached only where that type has one name: the names the type's copies hold, over all partitions, must agree, and a copy whose name does not read takes no part.

**Second probe** (run 43048938486). For each railing of Revit's export, the words of each `BaseRailing` copy that are the ElementId of a data object of the file, with the object's class. On all 198 railings of the four large files, and on the 41 of the office and the 2 of each terrain file, the words of the railing's copies together hold **exactly one** `StairsRailingAttr` id. Two attribute ids, or none, would have been the other readings of the failure; neither occurs.

**Third probe** (run 43050258474). The name each copy of the type object holds, with the partition. Every housing railing type has two copies, an older one in an earlier partition with a Finnish name and a newer one, in the partition the stored revision routes to (RE-216), with the English name Revit's export carries:

| housing file | railing types | older copy | newer copy (routed) | examples (older, newer) |
|---|---:|---|---|---|
| timber, as built | 9 | partition 1531 | partition 1611 | `KD-5`, `KAI-5`; `KD-3.1000`, `KAI-2`; `KD-3.käsijohde`, `KJO-2` |
| concrete, as built | 11 | partition 1192 | partition 1295 | `KD-1`, `KAI-1`; `KD-2.suojakaide`, `KAI-9`; `KD-4.luiska`, `KAI-7` |

So the names disagree, and the type has none. Held against Revit's type name (the second part of the railing's Name):

| file | railings | one attribute id | name where copies agree (as written) | name of the latest partition's copy | name of the routed copy |
|---|---:|---:|---:|---:|---:|
| timber housing, as built | 56 | 56 | 0 | 56 | 56 |
| timber housing, permit | 56 | 56 | 0 | 56 | 56 |
| concrete housing, as built | 43 | 43 | 0 | 43 | 43 |
| concrete housing, permit | 43 | 43 | 0 | 43 | 43 |
| terrain housing, timber and concrete | 2 each | 2 | 2 | 2 | 2 |
| office, as built | 41 | 41 | 41 | 41 | 41 |

Every entry is Revit's name: none of the three ways gives a wrong name where it gives one. The latest partition's copy is the way `type_data_names` already takes a type's element data name (#807, RE-145: "the latest partition's, none where its own copies disagree").

## 4. The change

`railing_type_picks` and the loop that reads a railing type's name from its type object (`find_railing_type_names`) take each type's name from the latest partition that holds one (`LatestCopy`), and none where that partition's own copies disagree, instead of from the copies that agree over all partitions. Where a type's copies agree nothing changes. A railing so typed is named `Railing:<type>:<tag>` as in Revit's export, and `Reference` and `Height` follow as they do on the office.

## 5. What Measure found

Not yet read: the unfiltered Measure of the change against `main` is running. The rows it is held to are registered on #1636 (https://github.com/DrunkOnJava/rvt-rs/issues/1636#issuecomment-6800382190, after the probe rows https://github.com/DrunkOnJava/rvt-rs/issues/1636#issuecomment-6799942669 and https://github.com/DrunkOnJava/rvt-rs/issues/1636#issuecomment-6799384065).

## 6. Reproduce

```bash
gh workflow run measure.yml --ref research/1193-railing-types -f ref=9d2a484f8ecf46ed7961262f09db9190e21f5aaf -f base=none -f nordic=true -f models="nordic-housing-* nordic-office-as-built" -f scorers=none -f probe=probe_re431_railing_types -f probe_scorer=railing_types_vs_ifc
```
