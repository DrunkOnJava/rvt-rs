# RE-431: a housing railing's type is named by the latest copy of its type object

**Date:** 2026-11-11
**Issues:** #1193 (W7: the BIM4LCA models' remaining property values), #1636 (the rules rest on the BIM4LCA models); follows RE-153 (a railing names its type in its own data object), RE-145 and #807 (the latest partition's copy is the current one), RE-231 and RE-254 (the office's railing heights and lengths)
**Artefacts:** the BIM4LCA housing, concrete and timber, as built and at building permit, the two terrain housing files and the office as built (Revit 2024), and Revit's IFC4 exports of them
**Probe and scorer:** `probes/examples/probe_re431_railing_types.rs`, `tools/re/railing_types_vs_ifc.py`; the first look used `probes/examples/probe_re231_record_lengths.rs` and `tools/re/stored_lengths_vs_psets.py`
**Runs:** Measure [43044928311](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43044928311) (the first probe, on `main` at `8ec2ac98`), [43048938486](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43048938486) (the ids and names a railing's copies name) [43050258474](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43050258474) (the name each copy of a type object holds in each partition, with three ways of choosing between them) and the change [43052157947](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43052157947) (unfiltered, `keep_ifc`, head `8298fefd` against `main` at `dae660b8`)
**Status:** Positive. The probe reads the cause on all 198 railings of the four large housing files, and the change gives all 237 railings whose type held two names their type, `Reference` and `Height`, equal to Revit's on every one (4 to 241 of 241), none different.

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

Run 43052157947 (126 exports each side). The property census of the 42 models that have a Revit export, base against head: Revit's values 213,756 each side; equal 199,261 to 199,972 (+711); missing 14,434 to 13,723; **different 61 to 61**; the values rvt-rs writes that Revit does not, 358 to 358. The 711 are 237 railings times three properties:

| property | values | equal before | equal after |
|---|---:|---:|---:|
| `Pset_RailingCommon.Reference` | 241 | 4 | 241 |
| `Pset_RailingCommon.Height` | 241 | 4 | 241 |
| `Pset_EnvironmentalImpactIndicators.Reference` (the railing's type name again, on the railings it is written for) | +237 | | +237 |

By file: the timber housing as built and at permit 56 of 56 each, the concrete housing 43 of 43 each, the timber office at permit 41 of 41 (4 before: the railings whose type already read), and the two terrain office files 1 of 1 each. The office as built and at permit (concrete) and the two terrain housing files, whose railing types hold one name, are byte-identical, as is every other export: **7 of 126 exports move and 119 are byte-identical**, the held-out models, the 24 Revit 2025 models, RE1 and Autodesk's samples among them. `tools/re/ifc_element_diff.py` on the 7 pairs (all keep their exports): no element is only in one file, 237 elements differ, and each is a railing that gains `Pset_RailingCommon` `Height` and `Reference`, `Pset_EnvironmentalImpactIndicators` `Reference`, the `Other` properties `Family`, `Family Name`, `Family and Type`, `Type` and `Type Id`, `Identity Data` `Type Name` and rvt-rs's own `RvtElementRecordGeometry` `FamilyName`, `FamilyNameSource` and `TypeName`, with nothing removed or changed; the entities that are added are 49 `IfcRailingType` objects with their 49 `IfcRelDefinesByType` relations (9 and 9 in the timber housing as built, 7 in the timber office at permit; Revit's housing export has no `IfcRailingType`, as section 2 says, and rvt-rs writes one for every railing type it knows, as it does on the office), 335 `IfcPropertySet`, 237 `IfcRelDefinesByProperties` and 2,517 `IfcPropertySingleValue`. `diagnostics.decoded.parameter_value_count` rises by the values added. `same-project gates` pass on the measured head (the Measure job that makes the verdict failed on `ECONNRESET` in an artifact download after the verdict was posted, #1742).

Held against the rows registered on #1636:

| row | result |
|---|---|
| 1 the housing's railings gain their type and properties | **Held**: 198 of 198 railings, `Reference` and `Height` equal to Revit's, none different |
| 2 where the copies already agree nothing moves | **Held**: the office as built and at permit, the terrain housing files and RE1 are byte-identical; the files that move are those whose railing types hold two names (the housing, the timber office at permit, the two terrain office files), each equal |
| 3 only those properties and names move | **Held**, with the `Other` and provenance properties and the 49 type objects above, and `Pset_EnvironmentalImpactIndicators.Reference`, which the row did not name |
| 4 `Length` | **Not moved**: `Qto_RailingBaseQuantities.Length` is read from a stair railing's path alone (RE-254) and the housing's free railings stay without it (318 missing in the census) |
| 5 no census row gains a `different` | **Held**: 61 to 61 on the 42 models, and no held-out export moves |

## 6. What is not done

- **`Qto_RailingBaseQuantities.Length` on the housing's free railings** (318 missing): RE-254's rule is a stair railing's path in 3D; a free railing's Length is its path's, and nothing read says which are level.
- **A railing whose type object holds no readable name, or whose latest partition's copies disagree,** gets no type, as before.

## 7. Reproduce

```bash
# the probe
gh workflow run measure.yml --ref research/1193-railing-types -f ref=9d2a484f8ecf46ed7961262f09db9190e21f5aaf -f base=none -f nordic=true -f models="nordic-housing-* nordic-office-as-built" -f scorers=none -f probe=probe_re431_railing_types -f probe_scorer=railing_types_vs_ifc
# the change against main
gh workflow run measure.yml --ref feat/1193-railing-type-names -f ref=8298fefd8110f9b16a694b11bd45b09650632110 -f base=dae660b8 -f samples=true -f nordic=true -f r2025=true -f holdout=true -f bimcollab=true -f racadv=true -f keep_ifc=true
```

The commits the runs measured are kept under `refs/archive/2026-11-11/cited/<first 12 hex>`.
