# RE-402: the first measurement of the shipped rules on a second project: 5 of the 15 census lines hold on the BIM4LCA housing, 9 do not, and RE-400 is wrong on a file it was never measured on

**Date:** 2026-11-10
**Issues:** #1636 (the shipped rules that rest on one project, and the second model that could test each); follows RE-399 and RE-400, and the pre-registration of [#1636 comment 6777884539](https://github.com/DrunkOnJava/rvt-rs/issues/1636#issuecomment-6777884539), posted 2026-11-10 at 20:02 UTC before any run on these models
**Artefacts:** eight Revit 2024 pairs of VTT's `ARCH.zip` with Revit 24.0.5's own IFC4 exports (CC BY-SA 4.0), pinned in `research/bim4lca-measure.tsv` by #1653: the housing in concrete and in timber, each as built and at building permit; the two small housing terrain files (concrete and timber, at permit); the timber office as built and at building permit. The housing is a second 2024 project. The timber office is the concrete office's project in another state, as built and at permit are, and counts as the same project.
**Runs:** seven Measure runs, listed in section 1; the first, [42892175045](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42892175045), is recorded as it printed, and the later ones are labelled as later.
**Status:** Measured as pre-registered, on `main` at `9066e124`. Of the 15 census lines the pre-registration reads, the concrete office, the project the rules were developed on, holds 13 and fails 2 (both failures are values rvt-rs writes that Revit omits, already counted there); the housing, a second project, holds 5, fails 9 and does not exercise 1; the timber office, the concrete office's project in a state no earlier run measured, holds 7 and fails 8. Three readings matter most. (1) Every wrong mullion value on the housing that the scorers list (all 48 on two files, 30 on the others) is one type, a 50 by 100 mm top profile, and is 0.005429 ft² above Revit's whatever its ends, so the ends are not the cause (section 8). (2) RE-400, merged at 20:39 UTC while its first measurement on these files was running, writes 24 wrong `NetSurfaceArea` values on the timber office at permit, 22 of them on a mullion with another section or type lying over it and 2 beside a short one; the repair narrows it (section 8). (3) The first run's roof and duct readings, 85 to 213 ft from Revit's, were the scorers': neither undid Revit's rotated IfcSite placement. They are repaired in #1653, and re-scored the roofs read 3 of 4 and 4 of 5 on the housing, one roof per file 0.033 ft and 0.2% off (section 7). Rows the outputs cannot read, the ones not exercised, and the repairs are listed in sections 6, 9 and 10; none of the repairs is made in the pull request that records this.

## 1. What was run

The pre-registration fixed, before any run on these models, what "holds" means for each rule that a report says rests on one project. A census line (a `Set.Name` line of `qtos_vs_ifc` or `psets_vs_ifc`) holds on a file when it reads `different 0` and none of its values is one rvt-rs writes that Revit's export omits; it is exercised by its `equal` count, and a line with `equal 0` is not exercised, never "holds". A rule on a body or a structure holds when its scorer finds no object the rule changes outside the bar its report states, and is exercised by the objects that carry its marker in the file's `diagnostics.json`. Any `different` value or any value rvt-rs writes that Revit omits, on any census line, is a finding whether or not a row names it. A rule is not tuned to these models in the pull request that adds them. No row was added after the first run: what the run turned up is a behaviour that is missing (section 5, section 8), not a shipped rule the list had missed.

The first run is the pre-registered measurement: `main` at `9066e124711917edfc8aa5515d0a05ba945b22d5` (committed at 20:01 UTC and not moved when the run was dispatched at 20:07 UTC), which holds RE-268's mullion rule and not RE-400, every scorer of Measure's default lists, one shard per large model. The later runs add what the pre-registration named but a default run does not hold (the scorers that are run by name), a baseline on the concrete office at the same commit, and the checks the first run's results called for.

| Run | What | Code under test | Scorers |
|---|---|---|---|
| [42892175045](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42892175045) | the first measurement: the eight models | `main` at `9066e124` | the default lists |
| [42897062282](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42897062282) | the concrete office as built and at permit, the censuses | `9066e124` | `qtos_vs_ifc`, `psets_vs_ifc` |
| [42899069462](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42899069462) | the concrete office as built and at permit, the body scorers | `9066e124` | the body scorers of section 6, the roof and duct scorers repaired (section 7) |
| [42896829177](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42896829177) | the eight models again, after the repair of section 7 | `9066e124` | `roof_bodies_vs_ifc`, `duct_bodies_vs_ifc`, and the two named ones, `slab_bodies_vs_ifc` and `wall_top_cover_vs_ifc` |
| [42896834169](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42896834169) | the eight models, saved meshes off | `9066e124` | `mullion_ends_vs_boxes` |
| [42896838588](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42896838588) | the eight models, RE-400's head against `main` | `03b345552c52` against `9066e124` | `qtos_vs_ifc`, `psets_vs_ifc` |
| [42898163164](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42898163164) | the timber office and the housing as built, saved meshes off, per mullion | `0bb8067f5399` (`main` with RE-400) | `mullion_ends_vs_boxes` and a scratch scorer, `mullion_wrong_detail.py`, commit `7b5bdc87e6ae`, never merged |

The first run took 15.1 minutes from its dispatch to the end of its compare job: a build of 3.0 minutes and seven shards of 2.6 to 11.2 minutes (46.1 runner minutes; the timber office as built sets the run). The 16 pinned files are 1,121 MB; a shard fetches and caches only its own pair, 144 to 200 MB for each of the six large pairs and 59 MB for the two terrain files together.

## 2. The census, per file

What Revit's export writes on the objects rvt-rs also writes, as the two censuses count it, at the same commit for every row of the table:

| File | Quantities Revit writes on objects rvt-rs writes | equal | different | missing | rvt-rs writes, Revit omits | Properties | equal | different | missing | rvt-rs writes, Revit omits |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Housing C AB | 5,274 | 3,552 | 131 | 1,591 | 42 | 7,189 | 5,766 | 1 | 1,422 | 167 |
| Housing C BP | 4,978 | 3,492 | 169 | 1,317 | 2 | 6,969 | 5,554 | 1 | 1,414 | 163 |
| Housing T AB | 5,788 | 3,692 | 170 | 1,926 | 28 | 8,100 | 6,609 | 1 | 1,490 | 168 |
| Housing T BP | 5,580 | 3,672 | 109 | 1,799 | 15 | 7,868 | 6,396 | 1 | 1,471 | 163 |
| Housing terr C | 1,464 | 1,379 | 0 | 85 | 6 | 1,621 | 1,559 | 1 | 61 | 47 |
| Housing terr T | 1,462 | 1,371 | 0 | 91 | 6 | 1,630 | 1,562 | 1 | 67 | 47 |
| Office C AB | 39,754 | 33,181 | 1 | 6,572 | 0 | 45,798 | 44,492 | 1 | 1,305 | 7 |
| Office C BP | 40,006 | 33,509 | 1 | 6,496 | 0 | 45,957 | 44,648 | 1 | 1,308 | 15 |
| Office T AB | 33,800 | 26,867 | 8 | 6,925 | 9 | 39,953 | 38,430 | 1 | 1,522 | 6 |
| Office T BP | 33,663 | 26,355 | 145 | 7,163 | 10 | 39,617 | 37,791 | 101 | 1,725 | 50 |

At this commit the concrete office, which the rules were developed on, reads 33,181 of 39,754 quantities equal as built (33,509 of 40,006 at permit), 1 different and none written that Revit omits, and 44,492 of 45,798 property values equal (44,648 of 45,957), 1 different and 7 (15) written that Revit omits. The housing as built in concrete reads 3,552 of 5,274 quantities equal, 131 different and 42 written that Revit omits, and 5,766 of 7,189 property values equal, 1 different and 167 written that Revit omits. The timber office reads like the concrete one as built (26,867 of 33,800 quantities equal, 8 different) and not at permit, where 145 quantities and 101 property values differ. One property value differs on every housing file: a stair's `IsExternal` (section 4).

## 3. The pre-registered census lines

Fifteen census lines carry the pre-registration's rows for quantities and properties (door and window sizes, and a door's `FireRating` with a family's `Manufacturer`, are two lines each). The door and window rows are also read by `door_window_sizes_vs_ifc`, which compares `OverallWidth` and `OverallHeight`, so a file fails them if either reads a difference; the scorer's own readings are in section 6.

| Rule (report) | Office Concrete, 2 files (developed on) | Office Timber, 2 files (same project, first measured here) | Housing, 6 files (second project) |
|---|---|---|---|
| RE-396 plate Width | holds on 2 | holds on 2 | holds on 4, not exercised on 2 |
| RE-387 volumes of meshes | holds on 2 | does not hold on 1, holds on 1 | does not hold on 4, holds on 2 |
| RE-268 mullion NetSurfaceArea | holds on 2 | does not hold on 2 | does not hold on 4, holds on 2 |
| RE-291/353 columns | holds on 2 | does not hold on 1, holds on 1 | holds on 6 |
| RE-266/293 wall Height | holds on 2 | does not hold on 2 | does not hold on 3, holds on 3 |
| RE-270/388 wall volumes and areas | holds on 2 | does not hold on 2 | does not hold on 4, holds on 2 |
| RE-296 space NetVolume | holds on 2 | holds on 2 | does not hold on 1, holds on 5 |
| RE-254 railing Length | holds on 2 | holds on 2 | holds on 4, not exercised on 2 |
| RE-215/373 door sizes | holds on 2 | does not hold on 1, holds on 1 | holds on 4, not exercised on 2 |
| RE-215/373 window sizes | holds on 2 | holds on 2 | does not hold on 4, not exercised on 2 |
| RE-386 opening FireExit | does not hold on 1, holds on 1 | does not hold on 2 | not exercised on 6 |
| RE-381/382 opening Reference | does not hold on 2 | does not hold on 2 | does not hold on 4, not exercised on 2 |
| RE-384 PitchAngle | holds on 2 | holds on 2 | does not hold on 3, holds on 3 |
| RE-354 door FireRating | holds on 2 | holds on 2 | does not hold on 1, holds on 3, not exercised on 2 |
| RE-354 Manufacturer | holds on 2 | holds on 2 | holds on 6 |

On the concrete office 13 of the 15 lines hold on both files. The two that do not are the opening `Reference` (3 values rvt-rs writes that Revit omits as built, 4 at permit) and `FireExit` (1, at permit): they are values rvt-rs writes that Revit omits, which the office's own census already counts, not values that differ.

On the housing, 5 lines hold where they are exercised, 9 do not and 1 is not exercised.

- **Hold:** the plate `Width` (RE-396: 78 equal on each of the four large files); the columns (RE-291, RE-353: `Length` only, 5, 5, 25, 25, 5 and 5 equal, since `CrossSectionArea`, `GrossVolume` and `OuterSurfaceArea` are written on none of the housing's columns and the criterion scores that as missing); the railing `Length` (RE-254: 4, 4, 13, 13); the door sizes (RE-215, RE-373: 195, 198, 201 and 204 equal in the census, none different in `door_window_sizes_vs_ifc`); the family types' `Manufacturer` (RE-354: 9, 10, 11, 11, 5, 5).
- **Do not hold:** the volumes of meshes (RE-387: 48, 40, 48 and 48 different) and the mullions' `NetSurfaceArea` (RE-268: the same 48, 40, 48 and 48, section 8); the wall `Height` (RE-266, RE-293: 3, 22 and 2 walls different on three of the four large files) and the walls' volumes and side areas (RE-270, RE-388: 24, 58, 65 and 10); the prism room's `NetVolume` (RE-296: 1 different at concrete permit); the window sizes (RE-215, RE-373: the census agrees on every window, while `door_window_sizes_vs_ifc` finds 17, 11, 11 and 11 windows whose `OverallWidth` or `OverallHeight` differ); the opening `Reference` (RE-381, RE-382: 1 written that Revit omits on each large file); the sloped floors' `PitchAngle` (RE-384: 1 at timber as built, 2 on each terrain file); a door's `FireRating` (RE-354: 1 at concrete permit).
- **Not exercised:** the opening `FireExit` (RE-386). Revit's export writes it on openings of the kind the census counts apart (a door or wall cut in a further host, "not scored"), so no scored value stands for it.

The timber office, a further state of the first project that no run measured before, fails 8 of the 15 lines: the volumes (49 different at permit), `NetSurfaceArea` (2 and 25), the columns' `Length` (2 at permit), the wall `Height` (1 and 1), the walls' volumes and areas (4 and 4), the door sizes (the census agrees, and `door_window_sizes_vs_ifc` finds 65 of 266 doors different in `OverallWidth` or `OverallHeight` at permit), and the opening `Reference` and `FireExit` (values rvt-rs writes that Revit omits, as on the concrete office).

Each cell below is `equal/different`, then `+n` for values rvt-rs writes that Revit omits on the row's lines, and for the two size rows `; scorer equal/differ` from `door_window_sizes_vs_ifc`; `-` is not exercised. A row with several lines adds them (the volumes row holds the member, plate and column `NetVolume` lines).

The housing, per file:

| Rule (report) | C AB | C BP | T AB | T BP | terr C | terr T |
|---|---:|---:|---:|---:|---:|---:|
| RE-396 plate Width | 78/0 | 78/0 | 78/0 | 78/0 | - | - |
| RE-387 volumes of meshes | 389/48 | 397/40 | 486/48 | 486/48 | 415/0 | 415/0 |
| RE-268 mullion NetSurfaceArea | 306/48 | 314/40 | 396/48 | 396/48 | 414/0 | 414/0 |
| RE-291/353 columns | 5/0 | 5/0 | 25/0 | 25/0 | 5/0 | 5/0 |
| RE-266/293 wall Height | 198/3 | 179/22 | 85/2 | 81/0 | 14/0 | 12/0 |
| RE-270/388 wall volumes and areas | 984/24 | 933/58 | 990/65 | 985/10 | 56/0 | 52/0 |
| RE-296 space NetVolume | 76/0 | 75/1 | 76/0 | 74/0 | 7/0 | 7/0 |
| RE-254 railing Length | 4/0 | 4/0 | 13/0 | 13/0 | - | - |
| RE-215/373 door sizes | 195/0; scorer 65/0 | 198/0; scorer 65/0 | 201/0; scorer 67/0 | 204/0; scorer 67/0 | - | - |
| RE-215/373 window sizes | 174/0; scorer 47/17 | 195/0; scorer 54/11 | 192/0; scorer 53/11 | 195/0; scorer 54/11 | - | - |
| RE-386 opening FireExit | - | - | - | - | - | - |
| RE-381/382 opening Reference | 35/0 +1 | 20/0 +1 | 19/0 +1 | 19/0 +1 | - | - |
| RE-384 PitchAngle | 26/0 | 32/0 | 34/0 +1 | 34/0 | 1/0 +2 | 1/0 +2 |
| RE-354 door FireRating | 17/0 | 17/0 +1 | 18/0 | 18/0 | - | - |
| RE-354 Manufacturer | 9/0 | 10/0 | 11/0 | 11/0 | 5/0 | 5/0 |

The office, per file (as built and at permit, concrete and timber):

| Rule (report) | Office C AB | Office C BP | Office T AB | Office T BP |
|---|---:|---:|---:|---:|
| RE-396 plate Width | 257/0 | 266/0 | 268/0 | 268/0 |
| RE-387 volumes of meshes | 10,137/0 | 10,264/0 | 8,160/0 | 8,103/49 |
| RE-268 mullion NetSurfaceArea | 6,606/0 | 6,636/0 | 5,253/2 | 5,130/25 |
| RE-291/353 columns | 1,114/0 | 1,299/0 | 380/0 | 368/2 |
| RE-266/293 wall Height | 132/0 | 130/0 | 103/1 | 82/1 |
| RE-270/388 wall volumes and areas | 1,420/0 | 1,479/0 | 1,477/4 | 1,424/4 |
| RE-296 space NetVolume | 167/0 | 178/0 | 168/0 | 161/0 |
| RE-254 railing Length | 4/0 | 4/0 | 4/0 | 4/0 |
| RE-215/373 door sizes | 588/0; scorer n/a | 582/0; scorer n/a | 576/0; scorer 265/0 | 576/0; scorer 200/65 |
| RE-215/373 window sizes | 807/0; scorer n/a | 795/0; scorer n/a | 795/0; scorer 265/0 | 795/0; scorer 265/0 |
| RE-386 opening FireExit | 25/0 | 33/0 +1 | 16/0 +1 | 16/0 +1 |
| RE-381/382 opening Reference | 158/0 +3 | 144/0 +4 | 155/0 +2 | 129/1 +2 |
| RE-384 PitchAngle | 62/0 | 62/0 | 45/0 | 36/0 |
| RE-354 door FireRating | 55/0 | 55/0 | 53/0 | 53/0 |
| RE-354 Manufacturer | 6/0 | 6/0 | 5/0 | 5/0 |

## 4. Census lines no row names

Every line with a different value or a value rvt-rs writes that Revit's export omits, on any file, is a finding whether or not a row names it. The concrete office, which the rules were developed on, is listed beside the new files because some of these are already counted there.

| Census line | Per file group: different values, and values rvt-rs writes that Revit's export omits |
|---|---|
| `Pset_BuildingElementProxyCommon.IsExternal` | Housing, four large files: 46 written that Revit omits; Housing terrain files: 15 written that Revit omits |
| `Pset_BuildingElementProxyCommon.Reference` | Housing, four large files: 46 written that Revit omits; Housing terrain files: 15 written that Revit omits |
| `Pset_BuildingStoreyCommon.EntranceLevel` | Housing, four large files: T AB 1 written that Revit omits, T BP 1 written that Revit omits |
| `Pset_ColumnCommon.IsExternal` | Office Concrete (baseline): C BP 1 written that Revit omits |
| `Pset_ColumnCommon.LoadBearing` | Office Concrete (baseline): C BP 1 written that Revit omits |
| `Pset_ColumnCommon.Reference` | Office Concrete (baseline): C BP 1 written that Revit omits |
| `Pset_CoveringCommon.IsExternal` | Housing, four large files: 2 written that Revit omits |
| `Pset_EnvironmentalImpactIndicators.Reference` | Housing, four large files: 54 written that Revit omits; Office Timber: T AB 2 written that Revit omits, T BP 23 different and 11 written that Revit omits; Office Concrete (baseline): C AB 3 written that Revit omits, C BP 5 written that Revit omits |
| `Pset_FurnitureTypeCommon.Reference` | Housing terrain files: 9 written that Revit omits |
| `Pset_MemberCommon.IsExternal` | Office Timber: T BP 1 written that Revit omits |
| `Pset_MemberCommon.Reference` | Office Timber: T BP 20 different and 1 written that Revit omits |
| `Pset_MemberCommon.Span` | Office Timber: T AB 1 different, T BP 53 different and 1 written that Revit omits; Office Concrete (baseline): 1 different |
| `Pset_PlateCommon.IsExternal` | Office Timber: T BP 8 written that Revit omits |
| `Pset_PlateCommon.Reference` | Office Timber: T BP 8 written that Revit omits |
| `Pset_ReinforcementBarPitchOfColumn.Reference` | Office Concrete (baseline): C BP 1 written that Revit omits |
| `Pset_ReinforcementBarPitchOfSlab.Reference` | Housing, four large files: T AB 1 written that Revit omits; Housing terrain files: 2 written that Revit omits |
| `Pset_ReinforcementBarPitchOfWall.Reference` | Housing, four large files: C AB 1 written that Revit omits; Office Timber: T BP 2 different |
| `Pset_RoofCommon.IsExternal` | Housing, four large files: 1 written that Revit omits; Office Timber: T BP 8 written that Revit omits |
| `Pset_RoofCommon.Reference` | Housing, four large files: 1 written that Revit omits; Office Timber: T BP 8 written that Revit omits |
| `Pset_SanitaryTerminalTypeCommon.Reference` | Housing, four large files: 11 written that Revit omits; Office Concrete (baseline): 1 written that Revit omits |
| `Pset_SlabCommon.FireRating` | Housing, four large files: T AB 1 written that Revit omits |
| `Pset_SlabCommon.IsExternal` | Housing, four large files: T AB 1 written that Revit omits; Housing terrain files: 2 written that Revit omits |
| `Pset_SlabCommon.Reference` | Housing, four large files: T AB 1 written that Revit omits; Housing terrain files: 2 written that Revit omits |
| `Pset_SpaceCommon.IsExternal` | Office Timber: 1 written that Revit omits |
| `Pset_StairCommon.IsExternal` | Housing, four large files: 1 different; Housing terrain files: 1 different |
| `Pset_WallCommon.ExtendToStructure` | Housing, four large files: C AB 1 written that Revit omits |
| `Pset_WallCommon.FireRating` | Housing, four large files: C AB 1 written that Revit omits |
| `Pset_WallCommon.IsExternal` | Housing, four large files: C AB 1 written that Revit omits |
| `Pset_WallCommon.Reference` | Housing, four large files: C AB 1 written that Revit omits; Office Timber: T BP 2 different |
| `Qto_MemberBaseQuantities.Length` | Office Timber: T AB 1 different, T BP 53 different; Office Concrete (baseline): 1 different |
| `Qto_OpeningElementBaseQuantities.Area` | Office Timber: 3 written that Revit omits |
| `Qto_OpeningElementBaseQuantities.Depth` | Office Timber: T BP 9 different |
| `Qto_SlabBaseQuantities.Depth` | Housing terrain files: 1 written that Revit omits |
| `Qto_SlabBaseQuantities.GrossArea` | Housing, four large files: 1 different |
| `Qto_SlabBaseQuantities.GrossVolume` | Housing, four large files: 1 different |
| `Qto_WallBaseQuantities.GrossFootprintArea` | Housing, four large files: C AB 6 different and 36 written that Revit omits, C BP 6 different and 2 written that Revit omits, T AB 5 different and 20 written that Revit omits, T BP 1 different and 10 written that Revit omits |
| `Qto_WallBaseQuantities.Width` | Office Timber: T BP 2 different |

Most of the lines are values rvt-rs writes that Revit omits, on few objects each. The ones with a count worth reading:

- `Qto_WallBaseQuantities.GrossFootprintArea` on the housing: 6, 6, 5 and 1 different and 36, 2, 20 and 10 written that Revit omits on the four large files.
- `Pset_BuildingElementProxyCommon.IsExternal` and `.Reference`: 46 on each large file and 15 on each terrain file, 42 of the 46 on the 42 proxies of section 5 (on the first file, 4 on proxies Revit writes and 42 on the others).
- `Pset_EnvironmentalImpactIndicators.Reference`: 54 on each large housing file (the concrete office has 3 and 5), and on the timber office at permit 23 different and 11 written that Revit omits.
- `Pset_StairCommon.IsExternal`: 1 different on every housing file, the one property value that differs there.
- The timber office at permit: `Qto_MemberBaseQuantities.Length` 53 different (1 as built), `Pset_MemberCommon.Span` 53 (1), `Pset_MemberCommon.Reference` 20, `Qto_OpeningElementBaseQuantities.Depth` 9, and 2 walls' `Width` and `Reference`. The first project's concrete states have 1 different `Length` and 1 `Span` each.

## 5. Which elements are written (W4)

`class_sets_vs_ifc` and `global_ids_vs_ifc`, per file, by Tag (the ElementId):

| File | Revit's paired elements and spaces | rvt-rs's | Revit's absent from rvt-rs | Revit's written as another class | rvt-rs's the export lacks | GlobalIds equal, not in Revit's, differing |
|---|---:|---:|---|---|---|---|
| housing concrete as built | 1,575 | 1,586 | 42 `IfcSpace` of 145 | 9 (`IfcFlowSegment` 3, `IfcFurniture` 3, `IfcCovering` 1, `IfcSolarDevice` 1, `IfcTransportElement` 1) | `IfcBuildingElementProxy` 42, `IfcSanitaryTerminal` 11 | 1,427, 53, 3 |
| housing concrete permit | 1,530 | 1,541 | 42 `IfcSpace` of 144 | 8 (`IfcFlowSegment` 3, `IfcFurniture` 3, `IfcSolarDevice` 1, `IfcTransportElement` 1) | `IfcBuildingElementProxy` 42, `IfcSanitaryTerminal` 11 | 1,383, 53, 3 |
| housing timber as built | 1,795 | 1,806 | 42 `IfcSpace` of 147 | 9 (`IfcFlowSegment` 3, `IfcFurniture` 3, `IfcCovering` 1, `IfcSolarDevice` 1, `IfcTransportElement` 1) | `IfcBuildingElementProxy` 42, `IfcSanitaryTerminal` 11 | 1,648, 53, 0 |
| housing timber permit | 1,724 | 1,735 | 42 `IfcSpace` of 145 | 8 (`IfcFlowSegment` 3, `IfcFurniture` 3, `IfcSolarDevice` 1, `IfcTransportElement` 1) | `IfcBuildingElementProxy` 42, `IfcSanitaryTerminal` 11 | 1,577, 53, 2 |
| housing terrain concrete | 557 | 512 | 42 `IfcGeographicElement` of 65, 3 `IfcWasteTerminal` of 28 | 26 (`IfcGeographicElement` 23, `IfcFlowTerminal` 3) | none | 504, 0, 0 |
| housing terrain timber | 558 | 513 | 42 `IfcGeographicElement` of 65, 3 `IfcWasteTerminal` of 28 | 26 (`IfcGeographicElement` 23, `IfcFlowTerminal` 3) | none | 505, 0, 0 |
| office timber as built | 10,866 | 10,865 | 1 `IfcSpace` of 306 | 0 | none | 10,560, 0, 0 |
| office timber permit | 10,817 | 10,825 | 1 `IfcSpace` of 306 | 8 (`IfcSlab` 8) | `IfcMember` 1, `IfcPlate` 8 | 10,511, 9, 0 |
| office concrete as built (baseline) | 12,974 | 12,974 | none | 1 (`IfcFlowTerminal` 1) | none | 12,666, 0, 0 |
| office concrete permit (baseline) | 13,019 | 13,020 | none | 1 (`IfcFlowTerminal` 1) | `IfcColumn` 1 | 12,711, 1, 0 |

- **The 42 `Turning Free Space` family instances** (a family type, 1,500 and 1,300 mm, that Revit exports as `IfcSpace` of `IfcSpaceType` `INTERNAL`, each with a mapped body) are absent from rvt-rs's spaces on every large housing file, and rvt-rs writes 42 `IfcBuildingElementProxy` that Revit's export lacks. The class sets are paired by Tag, so they are not paired here; whether they are the same elements is not read.
- **11 `IfcSanitaryTerminal` per large housing file** are written that Revit's export lacks: the box scorer lists 11 sanitary terminals with no reference body.
- Revit's other elements are all written on the large housing files: 9 as built and 8 at permit are written as another class (`IfcFlowSegment` 3, `IfcFurniture` 3, `IfcSolarDevice` 1, `IfcTransportElement` 1 and, as built, `IfcCovering` 1), and the GlobalIds equal Revit's on every element both files hold but 3, 3, 0 and 2.
- **The terrain files:** 42 `IfcGeographicElement` and 3 `IfcWasteTerminal` of Revit's are not written, and 26 of its elements are written as another class.
- **The timber office at permit:** 8 of Revit's 58 `IfcSlab` are written as `IfcRoof`, and rvt-rs writes 8 `IfcPlate` and 1 `IfcMember` that Revit's export lacks; as built the sets are equal but for one space.

## 6. Bodies (W6) and the pre-registered body and structure rows

W6's bar is that no body is a bounding box, and it fails on these files as it does on the office: the second column counts the bodies still drawn as their record's box (`box_bodies_vs_ifc`), with the class that has most of them and the share of the box Revit's body fills at the median (0.17 to 0.20 for sanitary terminals, 0.05 for railings, 0.06 for the terrain furniture). The other columns are the headline readings of the scorers the pre-registration names. The last two rows are the concrete office at the same commit (run 7): it has 183 and 184 record boxes (97 sanitary terminals at the same median fill), so the housing's counts are not a new failure of the bar; its walls' ends are Revit's on 516 of 607 (85%) and 439 of 589 (75%) against 69 to 79% on the housing, and its stair flights are within the bars on 0 of 35 at both states against 2 to 8 of 11 on the housing. Its openings are Revit's within 0.01 ft on 454 of 463 and 450 of 457, and the housing's are not (81 of 130 to 96 of 132).

| File | Record-box bodies (largest class) | Walls: faces within 0.001 ft | Walls: ends within 0.001 ft | Stair flights: every Revit vertex on our surface; vertices and volume within the bars | Landing slabs within the bars | Openings within 0.01 ft | Doors: mirror rule wrong; no host wall, of |
|---|---|---|---|---|---|---|---|
| housing concrete as built | 119 (46 `IfcSanitaryTerminal`, median fill 0.168) | 369 of 369 | 273 of 369 | 11 of 11; 2 of 11 | 6 of 6 | 81 of 130 | 11; 8 of 74 |
| housing concrete permit | 119 (46 `IfcSanitaryTerminal`, median fill 0.168) | 345 of 345 | 274 of 345 | 11 of 11; 2 of 11 | 6 of 6 | 88 of 131 | 11; 8 of 74 |
| housing timber as built | 132 (56 `IfcRailing`, median fill 0.052) | 367 of 367 | 252 of 367 | 11 of 11; 8 of 11 | 3 of 3 | 96 of 132 | 11; 8 of 76 |
| housing timber permit | 132 (56 `IfcRailing`, median fill 0.052) | 355 of 355 | 253 of 355 | 11 of 11; 8 of 11 | 3 of 3 | 90 of 133 | 11; 8 of 76 |
| housing terrain concrete | 11 (8 `IfcFurniture`, median fill 0.060) | 14 of 14 | 14 of 14 | 1 of 1; 1 of 1 | none | none | none |
| housing terrain timber | 11 (8 `IfcFurniture`, median fill 0.060) | 14 of 14 | 14 of 14 | 1 of 1; 1 of 1 | none | none | none |
| office timber as built | 182 (93 `IfcSanitaryTerminal`, median fill 0.198) | 611 of 611 | 400 of 611 | 29 of 31; 0 of 31 | 18 of 23 | 448 of 455 | 0; 76 of 266 |
| office timber permit | 191 (93 `IfcSanitaryTerminal`, median fill 0.198) | 576 of 586 | 275 of 586 | 29 of 31; 3 of 31 | 19 of 23 | 444 of 455 | 0; 76 of 266 |
| office concrete as built (baseline) | 183 (97 `IfcSanitaryTerminal`, median fill 0.198) | 606 of 607 | 516 of 607 | 35 of 35; 0 of 35 | 6 of 27 | 454 of 463 | 0; 75 of 269 |
| office concrete permit (baseline) | 184 (97 `IfcSanitaryTerminal`, median fill 0.198) | 589 of 589 | 439 of 589 | 35 of 35; 0 of 35 | 6 of 27 | 450 of 457 | 0; 77 of 269 |

- **Walls.** The faces across are Revit's on every wall of every file but 10 of 586 at the timber office's permit (the farthest 1.066 ft). The ends along are Revit's on 273 of 369, 274 of 345, 252 of 367 and 253 of 355 housing walls, which is not the bar either.
- **Wall tops** (`wall_top_cover_vs_ifc`, run 2; run 7 for the concrete office): rvt-rs's top is Revit's over the whole plan on 341 of 369, 320 of 345, 315 of 367 and 313 of 355 housing walls (14 of 14 and 12 of 14 on the terrain files), against 590 of 607 and 579 of 589 on the concrete office and 581 of 611 and 546 of 586 on the timber one. It is never lower than Revit's and taller on 6,226, 6,305, 7,392 and 7,092 of about 64,000 plan points each (9.7 to 11.5%), where the concrete office is taller on 1.3% and 0.4% and the timber office on 3.0% and 5.9%. The scorer does not say which walls a rule drew, so the stepped walls (`wall_top_stepped_at_covering_floor`: 7, 6, 14 and 14) are not attributed here, and RE-320's own result is a count that rose from 544 to 583 of 607, not a bar one wall passes or fails.
- **Openings.** Within 0.01 ft of Revit's boxes on 81 of 130, 88 of 131, 96 of 132 and 90 of 133 housing openings (448 of 455 and 444 of 455 on the timber office): the rest are 34 windows and 8 doors per file in walls, 0.26 to 0.82 ft off.
- **Rooms and Areas** (`space_profiles_vs_ifc`): of the spaces both files hold with a body (103, 102, 100 and 83), the areas are Revit's within 0.1% on 102, 102, 100 and 83 and the outlines within 0.01 ft on 100, 100, 96 and 79. The Areas drawn from the plan topology are Revit's in area, outline and boundary on every one the scorer pairs (20 of 20, 14 of 14, 15 of 15); the timber files write 5 (as built) and 20 (permit) spaces without a body, `area_body_refusals: next_storey_not_proven`.
- **Stairs.** Every vertex of Revit's body is within 0.001 ft of rvt-rs's surface on all 11 flights of each large file, and the vertices and volume are within the bars on 2, 2, 8 and 8 of the 11; the landing slabs are within them on 6 of 6, 6 of 6, 3 of 3 and 3 of 3.
- **Doors.** The mirror rule is wrong on 11 doors per large file, 8 of 74 (76) have no host wall in rvt-rs's export, and `door_axes_vs_ifc` finds none of the 130 to 133 doors and windows the current-copy rule places different from Revit's. The doors' and windows' sizes are in section 3.

### 6.1 The pre-registered body and structure rows

The pre-registration names 31 rows. The 13 read by the censuses are in section 3; the other 18 are here, with the marker that says whether the file exercises them (`diagnostics.json`, or the scorer's own count) and the scorer's reading. A row not exercised is not a row that holds.

| Row (report) | Exercised by, on the four large housing files | Reading | On the housing |
|---|---|---|---|
| Walls a floor covers in part drawn stepped (RE-320) | `wall_top_stepped_at_covering_floor`: 7, 6, 14, 14 | equal over the plan on 341 of 369, 320 of 345, 315 of 367, 313 of 355, against 590 of 607 and 579 of 589 on the concrete office | **not decided**: the scorer does not name the rule's own walls; the housing's rate is below the first project's, taller than Revit's on 9.7 to 11.5% of the plan points against 0.4 and 1.3% |
| Floors with shape-edited corners (RE-336) | `native_slab_shape_edit`: 4, 4, 4, 4 (1 on each terrain file) | `slab_bodies_vs_ifc`: every body within 0.001 ft and volume ratio 1.0000, 4 of 4 and 1 of 1 | **holds** |
| Profile-edited walls (RE-337, RE-348, RE-349, RE-358, RE-378) | `wall_end_sources`, `wall_profile_sources`, `wall_profile_top_sources`: none | | not exercised (the timber office has 7 profile walls at permit) |
| Profile-wall openings as hull pockets (RE-380) | `wall_profile_cuts`: none | | not exercised (the timber office has 9) |
| Wall end order at a corner (RE-322) | `wall_ends_order_dependent`: 0 | | not exercised |
| Roof level blocks from the saved mesh (RE-389) | roofs drawn from the saved mesh: 5, 5, 6, 6 (1 on a terrain file) | `roof_bodies_vs_ifc`, run 2: 3 of 4, 3 of 4, 4 of 5, 4 of 5, and 1 of 1 | **does not hold** on the four large files (one roof each, tag 1627602: 0.033 ft and 0.2% off); holds on the terrain file |
| Stair railings aggregated under their stair (RE-390) | the railings (43 and 56 per large file) | `aggregates_vs_ifc`: 2 railings per file that only Revit aggregates, on all six files | **does not hold** |
| Furniture systems (RE-352) | no `IfcSystemFurnitureElement` in either export | | not exercised |
| Room outlines from the stored solid, room phases (RE-316, RE-326) | `resolved_outline_spaces`, `approximate_outline_spaces`: 0 | | not exercised |
| Area regions from the plan topology (RE-330, RE-333) | Areas the scorer pairs: 20, 14, 15, 0 | area, outline and boundary Revit's on every one: 20 of 20, 14 of 14, 15 of 15 | **holds** where written; the timber files write 5 and 20 spaces without a body |
| Current copy of an instance (RE-216) | doors and windows placed by the rule: 130, 131, 132, 133 | `door_axes_vs_ifc`: none differs from Revit's | **holds** |
| Landing slabs (RE-214) | `partition_landing_slab`: none | landings are within the bars by another path, 6 of 6, 6 of 6, 3 of 3, 3 of 3 | not exercised |
| Hidden-form columns (RE-205) | `column_bodies_vs_ifc`: 0 columns | | not exercised |
| Ports, pipes and ducts (RE-212, RE-370, RE-371) | ducts: 1 written of Revit's 3; pipes and ports: none | `duct_bodies_vs_ifc`, run 2: the one box is 3.0 ft off | **does not hold** on the duct; pipes and ports not exercised |
| Bodiless family spaces left out (RE-312) | | the outputs do not count the family spaces left out | not read |
| Nested components Revit leaves out (RE-79) | | the 53 elements of section 5 that Revit's export lacks are not attributed to a rule | not read |
| Floor void tags (RE-339) | | `openings_vs_ifc` is not split by tag | not read |
| Parameters named for a standard property (RE-194) | | the censuses do not isolate them | not read |

Of the 14 rows the outputs can read, 3 hold, 3 do not, 1 is not decided and 7 are not exercised.

## 7. The roof and duct scorers read the housing wrongly

The first run printed, for every roof of the housing and the timber office, Revit's vertices 85.8 to 213 ft from rvt-rs's, and for the one housing duct a box 134 ft off, while the roofs' volumes were equal to the seventh digit (1,586.688414 ft³ in both files for the timber office's roof, tag 3067686). Neither scorer undoes Revit's IfcSite placement, as the stair, wall and opening scorers do with `ifc_common.site_frame`, and Revit's export of the housing places the site 60.6 degrees off rvt-rs's frame (`frames_vs_ifc`: the fitted rotation is −60.56 degrees, Revit's site placement −60.63) and the timber office's 40.3 degrees, like the concrete office's. In the earlier Measure run that read the frames of 33 models, only the four BIM4LCA office files had a rotated site (40.33 degrees), and none of them has a roof or a duct body in Revit's export, so the scorers had only ever run unrotated and nothing had shown it.

A synthetic pair, one roof and one duct under a site rotated −60.6317 degrees and offset 100 and 200 m in a millimetre file against the same bodies under an unrotated metre site, read 0 of 1 with a farthest vertex 535 ft away and a duct box 427 ft off before the repair, and 1 of 1 and 0 ft after it. The repair is the second commit of #1653 (`tools/re/roof_bodies_vs_ifc.py`, `tools/re/duct_bodies_vs_ifc.py`); run 42896829177 re-scored the eight models with it.

| File | Roofs, run 1 as printed: vertices and volume within the bars; Revit's vertices on our surface | Roofs, run 2 (scorer repaired) | Ducts, run 1 as printed | Ducts, run 2 |
|---|---|---|---|---|
| housing concrete as built | 0 of 4; on the surface 0 of 4 | 3 of 4; on the surface 4 of 4 | 1 of 3 written, 0 within 0.01 ft (worst 134.501 ft) | 1 of 3 written, 0 within 0.01 ft (worst 3 ft) |
| housing concrete permit | 0 of 4; on the surface 0 of 4 | 3 of 4; on the surface 4 of 4 | 1 of 3 written, 0 within 0.01 ft (worst 134.57 ft) | 1 of 3 written, 0 within 0.01 ft (worst 3 ft) |
| housing timber as built | 0 of 5; on the surface 0 of 5 | 4 of 5; on the surface 5 of 5 | 1 of 3 written, 0 within 0.01 ft (worst 134.148 ft) | 1 of 3 written, 0 within 0.01 ft (worst 3 ft) |
| housing timber permit | 0 of 5; on the surface 0 of 5 | 4 of 5; on the surface 5 of 5 | 1 of 3 written, 0 within 0.01 ft (worst 134.148 ft) | 1 of 3 written, 0 within 0.01 ft (worst 3 ft) |
| housing terrain timber | 0 of 1; on the surface 0 of 1 | 1 of 1; on the surface 1 of 1 | none | none |
| office timber as built | 0 of 1; on the surface 0 of 1 | 0 of 1; on the surface 1 of 1 | none | none |
| office timber permit | 0 of 1; on the surface 0 of 1 | 0 of 1; on the surface 1 of 1 | none | none |

After the repair the housing's roofs read 3 of 4 on the two concrete files and 4 of 5 on the two timber files, and 1 of 1 on the terrain timber file. On each large file the one that differs, tag 1627602, has Revit's farthest vertex 0.033 ft from rvt-rs's nearest and a volume 0.2% larger in rvt-rs's body (31.677807 against 31.613562 ft³ at concrete; 33.656735 against 33.588523 at timber), with every Revit vertex within 0.000003 ft of rvt-rs's surface. The timber office's one roof has an equal volume to the ninth digit and every Revit vertex on rvt-rs's surface, with the vertices 0.0394 ft from the nearest of rvt-rs's: the same surface tessellated differently, which the scorer's bar of vertices within 0.001 ft reads as "differs". The one housing duct is 3.0 ft off Revit's, and Revit's export holds 3 ducts of which rvt-rs writes 1.

The roofs' and ducts' first readings stay in the table as printed, labelled as the first run's: they are what the scorers said, and what was wrong was the scorers.

## 8. The mullions

**The housing's mullions.** The rule of RE-268 leaves out no mullion on the housing but those whose axis is not known (2 on each concrete file, 8 on each timber file) and writes a `NetSurfaceArea` on the 354 (concrete) and 444 (timber) mullions that carry a `NetVolume`. It is wrong on 48, 40, 48 and 48 of them, and as many `NetVolume` values differ. Run 6 lists all 48 of each as-built file, and run 3 the first 30 of each file: every one is the same type, `Liukulasituksen yläprofiili`, a section of 50 by 100 mm lying horizontally, 660 to 960 mm long, with 1 or 2 end faces counted, and the value rvt-rs writes is **0.005429 ft² (505 mm²) above Revit's on every one**, whatever the ends read and the length. The count of ends Revit's value implies is not a whole number for any of them. So the ends the rule reads are not what is wrong: Revit's solid for this type has an area that the box's section does not give, by an amount that depends on neither length nor ends. The other 306, 314, 396 and 396 values are Revit's, and the 414 of 414 on each terrain file.

**RE-400 on these files.** RE-400 (#1651) merged at 20:39 UTC as `0bb8067f`, while run 42896838588, which measures its head (`03b345552c52`) against `main` on these files, was running: none of these files was in Measure, so it had not been measured on any of them. On the six housing files no census line moved. On the timber office it moves `Qto_MemberBaseQuantities.NetSurfaceArea` and nothing else:

| File | equal | different | missing | of 7,876 (as built) or 7,872 (permit) |
|---|---:|---:|---:|---|
| timber office as built, `main` | 5,253 | 2 | 2,621 | |
| timber office as built, RE-400 | 5,431 | 2 | 2,443 | +178 values, all Revit's |
| timber office at permit, `main` | 5,130 | 25 | 2,717 | |
| timber office at permit, RE-400 | 5,292 | 49 | 2,531 | +186 values: 162 Revit's and **24 not** |

The 24 wrong values are 22 on overlapped mullions, which RE-400 reads on their own (104 of its 126 there are Revit's; as built all 118 are), and 2 on mullions that meet one shorter than its section (58 of 60 are Revit's; as built all 60 are). The 2 are two of the 12 uprights of 50 by 50 mm of one curtain wall (host 3076052), all 0.0558 ft² above Revit's, of which RE-268 already got the other 10 wrong. Run 6 prints what lies over each overlapped mullion, per overlap:

| What lies over the overlapped mullion's end | right | wrong | right, as built |
|---|---:|---:|---:|
| a mullion of the same type and section (`PAR_IN` with 2 or 4 corners of the end face inside, `PAR_SAME`) | 140 | 0 | 170 |
| a mullion of the same section and another type | 2 | 2 | 0 |
| a mullion of another section (a 50 by 50 mm one over a 60 by 120 mm one, 0 or 4 corners of the end face inside) | 0 | 20 | 0 |

No overlap of a mullion of the same type and section is wrong, on either file (140 pairs at permit, 170 as built), and every wrong one but 2 has another section lying over it. RE-400's report found 159 (157) such overlaps on the concrete office, all right, and its rule rests on them. The repair reads an overlapped mullion on its own only where what lies over it has its type and section, and leaves the other 22 out as RE-268 did; the 2 that meet a short mullion are the next thing to settle, since they sit on a wall RE-268 already gets wrong. It is the next pull request, measured on every Measure model before it lands. Until it lands `main` writes the 24 wrong values.

## 9. Findings to repair

Each is its own pull request after this report, with the row and the number above as its acceptance.

1. **RE-400 narrowed** (section 8): an overlapped mullion is read only where what lies over it has its type and section, and the 2 uprights that meet a short mullion on host 3076052 are settled. Acceptance: `Qto_MemberBaseQuantities.NetSurfaceArea` different 25 or fewer at the timber office at permit and 2 as built, nothing moved on the six housing files, and the concrete office's 926 and 924 values kept.
2. **The mullion profile of the housing** (section 8): the 48, 40, 48 and 48 mullions of `Liukulasituksen yläprofiili`, 505 mm² off Revit's in `NetSurfaceArea` and as many in `NetVolume`, from a section the box does not give. Acceptance: different 0 on both lines on the four large files, or the type left out where it is not read.
3. **The walls' `Height`, volumes and side areas** on the housing (3, 22 and 2 walls different on `Height`; 24, 58, 65 and 10 on the volumes and areas), and the timber office's (1 and 1; 4 and 4).
4. **The sizes of windows and doors:** 17, 11, 11 and 11 housing windows and 65 timber-office doors at permit have an `OverallWidth` or `OverallHeight` that Revit's does not, while every census line agrees (RE-215, RE-373).
5. **Values rvt-rs writes that Revit omits** (section 4): `GrossFootprintArea` on the housing's walls, the 46 (15) proxies' property sets, `Pset_EnvironmentalImpactIndicators.Reference`, the opening `Reference`, `PitchAngle` and `FireRating` extras; and the single different values: `Pset_StairCommon.IsExternal` (6 files), the slab `GrossArea` and `GrossVolume` (4 files), the prism room's `NetVolume` (1).
6. **Elements** (section 5): the 42 `Turning Free Space` spaces written as proxies, the 11 sanitary terminals, the terrain files' 42 geographic elements and 3 waste terminals, and the timber office's 8 slabs written as roofs with 8 plates and 1 member Revit lacks.
7. **Bodies** (section 6): wall tops (25 to 52 walls per large file), wall ends, the openings 0.26 to 0.82 ft off, the roof of each large file 0.033 ft and 0.2% off, 2 of 3 ducts, the 2 railings per file only Revit aggregates, and the 5 and 20 bodiless Areas at the timber files.
8. **The timber office at permit** (sections 3 and 4): `Length` and `Span` of 53 mullions, 20 `Reference` values, 23 environmental `Reference` values, 9 opening `Depth` values, 2 wall `Width` values and the 65 doors, a state the concrete office does not show.

## 10. Not shown

- **A cause for any of them**, except where one is measured: the mullion type's constant (section 8) and the scorers' frame (section 7). Each repair starts with its own probe.
- **The rows the outputs cannot read** (section 6.1: RE-312, RE-79, RE-339, RE-194) and the type value blocks of RE-354 beyond the census.
- **Whether a rule that rests on one project may ship.** That stays the maintainers' call; this is the measurement it needs, and it says, row by row, which rules the second project confirms.
- **Revit 2025 to 2027 and the earlier releases.** Both projects are Revit 2024.
- **These files as W5 candidates.** They are development reference models now (`docs/reference-models.md`); their later measurements are regression checks, not first measurements.

## 11. Reproduce

The scorers and the workflow come from `--ref`, the code from `-f ref`; once #1653 has landed, `--ref main` replaces its branch.

```bash
gh workflow run measure.yml --ref main -f ref=9066e124711917edfc8aa5515d0a05ba945b22d5 -f base=none -f nordic=true -f models="nordic-housing-* nordic-office-timber-*"
gh workflow run measure.yml --ref main -f ref=9066e124711917edfc8aa5515d0a05ba945b22d5 -f base=none -f nordic=true -f models="nordic-office-as-built nordic-office-permit" -f scorers="qtos_vs_ifc psets_vs_ifc"
gh workflow run measure.yml --ref main -f ref=9066e124711917edfc8aa5515d0a05ba945b22d5 -f base=none -f nordic=true -f models="nordic-housing-* nordic-office-timber-*" -f scorers="roof_bodies_vs_ifc duct_bodies_vs_ifc slab_bodies_vs_ifc wall_top_cover_vs_ifc"
gh workflow run measure.yml --ref main -f ref=9066e124711917edfc8aa5515d0a05ba945b22d5 -f base=none -f nordic=true -f models="nordic-housing-* nordic-office-timber-*" -f ifc_flags=--no-saved-meshes -f scorers="mullion_ends_vs_boxes"
gh workflow run measure.yml --ref main -f ref=03b345552c5240bbb7c945e7623b4052a6895762 -f base=9066e124711917edfc8aa5515d0a05ba945b22d5 -f nordic=true -f models="nordic-housing-* nordic-office-timber-*" -f scorers="qtos_vs_ifc psets_vs_ifc"
```

The per-mullion detail of run 42898163164 is `mullion_wrong_detail.py` at commit `7b5bdc87e6ae`, a scratch scorer on top of `mullion_ends_vs_boxes.py`; it is not merged, and the repair's own report carries what it needs.
