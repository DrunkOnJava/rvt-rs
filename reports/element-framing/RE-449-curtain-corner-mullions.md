# RE-449: a curtain part that names two curtain walls takes the one its grid's data names: 74 of the 79 corner mullions of `rac_advanced` 2027 get their wall back

**Date:** 2026-11-13
**Issues:** #1865 (79 curtain members of Autodesk's `rac_advanced` saved by Revit 2027 get no `IsExternal` where the 2025 save writes it); follows RE-46 (a part names exactly one curtain wall), RE-62 (a part that names none takes the wall whose record box holds it), RE-72 (a part's grid names its wall) and RE-179 (79 corner mullions take no curtain wall on 2027)
**Artefacts:** Autodesk's sample projects `racadvancedsampleproject.rvt` of Revit 2025 (sha256 `512e9188b245ba58fcc918630e4388b836df37042aa07de6db0e223b48de25d1`, 17,068,032 bytes) and Revit 2027 (`a88eb9f85e1e2d47bbc91ee214b178c2d7075822b6d08694f59e3e6f259024ed`, 17,133,568 bytes), both public and redistributable (`research/autodesk-sample-projects.tsv`). The same project saved by the two releases: no Revit export of either exists, so the 2027 save is held to the 2025 save element by element, as #1861 was.
**Probes:** `probes/examples/probe_1865_curtain_is_external.rs` (#1877), `probe_1865_part_walls.rs` (#1879) and `probe_1865_grid_walls.rs` (#1890)
**Runs:** Measure [43379354161](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43379354161) (`probe_1865_curtain_is_external`), [43381092468](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43381092468) (`probe_1865_part_walls`), [43382999619](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43382999619) and [43384818097](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43384818097) (`probe_1865_grid_walls`), each on `rac_advanced` 2025 against its 2027 save (`probe_pair_release=2027`); the change itself: [43386636537](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43386636537) (the pair run), [43392439717](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43392439717) (unfiltered, `8744d8d7` against `b979b791`) and [43397936815](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43397936815) (the same two commits on eight of the exports that differ, their IFC kept)
**Status:** Positive for 74 of the 79 mullions; a measured negative for the other 5 (no id of their lists has element data that names a wall) and for the curtain door 216844, which loses its wall on 2027 for another cause that is not found. The change reaches 14 exports the registered row 3 said it would not (section 4a): they gain `IsExternal` on curtain mullions, and two members of the BIM4LCA timber office take the curtain wall Revit's own export names.

## 1. The question

Of the 3,192 curtain members of `rac_advanced`, the 2025 save and the 2026 save give every one a `Pset_MemberCommon.IsExternal` (#1861), and the 2027 save leaves 79 without. A mullion takes its `IsExternal` from its curtain wall, which it reaches through `m_aggregate_whole`, set by `attach_curtain_walls` from its reference list. Why do 79 mullions have no wall on 2027, and which wall is theirs?

## 2. What the bytes say

**All 79 are standalone, and nothing else moved.** `probe_1865_curtain_is_external` finds `is_external_lost IFCMEMBER=79` and nothing for any other class; every one is a `L Corner Mullion:L Mullion 1`, aggregated into a curtain wall on 2025 and into none on 2027. The 34 curtain walls exist on both saves, are all `IsExternal` true on both, and no field of any of them differs between the saves.

**They name two walls on 2027 and one on 2025.** `probe_1865_part_walls` lists for each lost part the curtain walls its list names, in list order. The 79 lie on 9 corners, the second wall of each being a different one:

| corner (walls named on 2027) | the wall they took on 2025 | mullions |
| --- | --- | ---: |
| 140012, 140015 | 140015 | 12 |
| 140015, 140018 | 140015 | 12 |
| 140003, 140006 | 140003 | 9 |
| 139858, 140009 | 140009 | 9 |
| 139855, 149877 | 139855 | 9 |
| 139856, 148422 | 139856 | 9 |
| 140009, 156408 | 140009 | 9 |
| 140003, 155419 | 140003 | 9 |
| 157669, 162524 | 162524 | 1 |

`attach_curtain_walls` matches on the pair (walls the part's grids give, curtain walls its list names): `(1, _)` takes the grid's wall, `(0, 1)` the named one, `(0, 0)` the one whose record box holds the part, anything else none. `probe_1865_grid_walls` reproduces it for the 80 parts that lose their wall: on 2025 all 80 are `(0, 1)`, on 2027 the 79 mullions are `(0, 2)`, an arm the match does not have.

**Neither the list nor the boxes pick the wall.** The 2025 wall is the first of the two in the 2027 list for 57 of the 79 and the last for 22, and no part's record box lies inside either wall's within 0.05 ft (RE-62's tolerance): `box_none=80`.

**The data of the part's grid does.** For each of the 75 parts whose list holds an id that has element data naming one of the part's curtain walls, **exactly one** of the part's named walls lies in that data, on the 2025 save and on the 2027 save, and it is the wall that held the part on 2025: 75 of 75 on each save, no part with two. The data names the wall four times (the 74 mullions' grids at 84, 539, 930 and 1130 bytes past the id's eight on 2025, 84, 531, 914 and 1114 on 2027; the door's at 84, 583, 976, 1190 and 84, 575, 960, 1174), so no fixed offset is read: RE-72's 85 and 478, measured on RE1's grids, are on neither save for these. Five mullions (162801, 183114, 183316, 183402, 197365) have no such id on either save.

**In the 16 models of that run, only these 79 name two walls.** The one-file counts of `probe_1865_part_walls` over the 16 models of its run (`core-interior` and the 2025 and 2027 samples) have `named2` above 0 only on the 2027 `rac_advanced` mullions (79, none aggregated), and `named3plus` is 0 everywhere. I wrote from that that the rule reaches no other model Measure holds, which the census does not say: its run held neither the 2015 to 2023 samples nor the BIM4LCA offices, and the unfiltered run (section 4a) shows 14 more exports that the rule changes.

**The door.** The curtain door 216844 (`M_Curtain Wall Sgl Glass`) is aggregated into wall 162023 on 2025 and into none on 2027, names that one wall on both saves and is `(0, 1)` on both, so the match gives it the wall on both. Its loss has another cause, not found here, and it is unchanged by this report's change.

## 3. The rows (registered on #1865 before each run)

Comments [6845990245](https://github.com/DrunkOnJava/rvt-rs/issues/1865#issuecomment-6845990245) (the first probe), [6846024013](https://github.com/DrunkOnJava/rvt-rs/issues/1865#issuecomment-6846024013) (row 5, from RE-179), [6846247941](https://github.com/DrunkOnJava/rvt-rs/issues/1865#issuecomment-6846247941) (the list probe), [6846501673](https://github.com/DrunkOnJava/rvt-rs/issues/1865#issuecomment-6846501673) (the grid probe) and [6846957690](https://github.com/DrunkOnJava/rvt-rs/issues/1865#issuecomment-6846957690) (the change). The expectations I had that failed: that the 79 are the parts of three curtain walls whose boxes `curtain_wall_box_not_read` points to (they leave six walls and none of the three is involved, and the diagnostics line has no bearing on them); and, in the last comment's row 3, that no model but the 2027 `rac_advanced` has a part this rule reaches, which I read from a census of 16 models as if it were one of every model Measure holds (section 4a).

## 4. What Measure found

Rows 1 and 2 of section 3's last comment, read on the pair run [43386636537](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43386636537) (`f21e8d33`, the change; `probe_1865_curtain_is_external` on each sample project's 2025 and 2026 saves against its 2027 save). Rows 3 and 4 are read on the unfiltered run in section 4a.

1. **Holds.** On `rac_advanced`'s 2027 save the 34 curtain walls hold 4,458 parts, 74 more than before the change (4,384), and 29 of the 34 hold exactly as many parts as on the 2025 save. The five that differ are the walls of the six parts that stand alone: 140003 (1 fewer), 140009 (1), 140015 (2) and 162524 (1) lose a corner mullion each of the five without a grid id, and 162023 (1) the door 216844, the one part that shows `other=1` on the 2025 save. Against the 2026 save the same four walls differ and 162023 does not (4,463 parts there, the door being a part of no wall on 2026 as on 2027). The count of a wall does not prove that each mullion is under the wall it took on 2025; what does is that the rule picks, for 75 of 75 parts, the one wall of the list that the part's grid data names, and that wall is the one that held the part on 2025 (section 2).
2. **Holds.** `is_external_lost` is empty on the 2025 to 2027 and 2026 to 2027 pairs of `rac_advanced` (from 79 `IFCMEMBER`), and on every other project's pair (`rac_basic`, `rme_advanced`, `rme_basic`, `rst_basic`): no member that has an `IsExternal` on the earlier save lacks one on 2027. All 34 walls read `without_is_external=0` on 2027 (from 1 wall with a part that has none, the door's, on 2025).
3. **Does not hold.** On the unfiltered run [43392439717](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43392439717) 15 of 146 export hashes differ from `main`'s and 11 of 121 `diagnostics.json` (the four `racadv-*` models write none). The row allowed one, the 2027 `rac_advanced`; section 4a reads the other 14.
4. **Holds.** The `same-project gates` check run on `8744d8d7` is `success`: 15 samples held to their earlier saves, and the Areas on 4 saves and on the 2021 save.

Rows 1, 2 and 4 held. Row 3 failed, and section 4a reads what it found.

## 4a. The unfiltered run

Run 43392439717 measured `8744d8d7` against `b979b791` with every input on (`samples`, `nordic`, `r2025`, `racadv`, `holdout`, `bimcollab`, `families`). 131 of the 146 exports equal `main`'s byte for byte, among them every held-out model, Core Interior, Einhoven, RE1 Architecture, Snowdon Towers and the 2024 to 2026 saves of `rac_advanced`. The 15 that differ, with the diagnostics' `parameter_value_count` before and after:

| export | `parameter_value_count` | what the IFC gains |
| --- | ---: | --- |
| `rac_advanced` 2015, 2016, 2017 | 19,900 to 19,979 | `IsExternal` true on 79 `IfcMember` |
| `rac_advanced` 2020 | 19,970 to 19,979 | `IsExternal` true on 9 |
| `rac_advanced` 2021 | 19,960 to 19,979 | `IsExternal` true on 19 |
| `rac_advanced` 2023 | 19,964 to 19,973 | `IsExternal` true on 9 |
| `rac_advanced` 2027 | 20,956 to 21,035 | `IsExternal` true on 79: the change's target |
| BIM4LCA office, as built and permit | 54,868 to 54,870 and 55,067 to 55,069 | `IsExternal` true on 2 (read on the as-built file) |
| BIM4LCA timber office, as built and permit | 46,764 to 46,768 and 46,445 to 46,449 | `IsExternal` false on 4, and two curtain walls gain a member each (read on the as-built file) |
| `racadv-2015`, `-2016`, `-2017`, `-2020` | none written | counts only; their scorer counts do not move |

**What the files hold.** I read the IFC of eight of the fifteen on run [43397936815](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43397936815) (the same two commits with `keep_ifc`: the two as-built offices and the `rac_advanced` samples of 2015, 2016, 2017, 2020, 2021 and 2023) with `tools/re/ifc_bag_diff.py`, which counts the entity lines of the two files with their step numbers removed, since one entity added early renumbers every line after it. In all eight the entities that differ are `Pset_MemberCommon` property sets of `IfcMember` entities that gain one property, `IsExternal`, as many as the table says (79, 79, 79, 9, 19, 9, 2 and 4); and in the as-built timber office two `IfcRelAggregates` that gain a member each (21 to 22) and two `IfcRelContainedInSpatialStructure` lists that lose one each (344 to 343 and 343 to 342), which is what a part does when it leaves a storey's containment for its wall's aggregation. No other entity line is added or removed: no body, quantity, name, type or placement. The comparison is blind to one thing, a reference that points elsewhere with the list the same length; the new arm cannot cause that, since a part that already has a wall never reaches it (`attach_curtain_walls` reaches the arm only on `(0, 2 or more)`, which was `None`). The permit offices and the four `racadv-*` models I did not read as IFC; their hashes differ, the offices' diagnostics move by the same kind of amount (2 and 4), and the timber permit office's `aggregates_vs_ifc` moves as the as-built one's does.

**Why the change reaches them.** These are files in which a curtain part names two or more curtain walls and finds no grid, which the census of section 2 could not show for the models it did not hold (`core-interior` and the 2025 and 2027 samples). In seven of the eight read, no part gains a wall: the part stays alone and takes the `IsExternal` its walls share (`attach_curtain_member_is_external`), written only where every wall it names has one and they agree. The arm that picks a wall by element data fires on the two members of the timber office. The old rule gave these parts neither a whole nor an `IsExternal`, and it cannot have re-parented a part, since the arm it adds is reached only by a part that took no wall before.

**What checks it.**

- *Revit's aggregates (an oracle, for the walls).* On the timber office `aggregates_vs_ifc` reads `IfcMember` "only Revit aggregates" 4 to 2 and "same whole" 7,872 to 7,874 (permit 7,868 to 7,870): the two members now sit under the wall Revit's own export puts them under. Nothing else in any scorer moves on any model (`compare-scorer-diff` holds only these lines and the 2027 `bodies_across_saves` lines).
- *Revit's `IsExternal` (no oracle).* Revit's export writes `IsExternal` on curtain members as an artefact of its exporter's order (RE-269) and the census counts those values apart (RE-288 took the host rule), so `psets_vs_ifc` does not move on any model (on the two offices its lines are byte-identical before and after: 45,798 values on objects rvt-rs writes, 44,501 equal, 1 different, 1,296 missing, 7 values Revit does not write; 39,953, 38,501, 1, 1,451 and 6 on the timber office) and cannot score the 2 + 4 + 79 + 79 + 79 + 9 + 19 + 9 values added. **They are unmeasured, not equal.** They are the host rule's value and nothing more: a part whose walls disagree gets none.
- *The same project's other saves.* `parameter_value_count` of `rac_advanced` after the change is 19,979 on the 2015 to 2022 saves (19,900 on 2015 to 2017, 19,970 on 2020 and 19,960 on 2021 before; 2018, 2019 and 2022 were there already) and 21,035 on the 2024 to 2027 saves (20,956 on 2027 before). The 2023 save is the one that does not meet its neighbours: 19,973, from 19,964, six values short.

The expectation that failed is the one in section 3, about the three walls, and row 3. Row 3's premise was a census of 16 models read as a census of every model; the fifteen changes it missed are all of one kind and each is the host rule's value on a curtain member, but I did not know that when I registered the row, and the registered row was wrong.

## 5. The change

`scan_walls_in_element_data` (new, `curtain.rs`) reads, for the ids of the lists of the parts that name two or more curtain walls and no grid, which curtain walls each id's element data names: those whose ElementId, as a `u64`, lies at least twice within 1,200 bytes past the id, where the release's element-data header opens the data. `attach_curtain_walls` takes, for a part with no grid and two or more named walls, the one wall of its list that these name, where exactly one does. A part for which that gives no one wall stands alone and keeps the walls it names (`m_curtain_walls_named`); `attach_curtain_member_is_external` gives such a part the `IsExternal` all its walls have, where each has one and they agree. Both reach every file with a part that names two or more curtain walls and finds no grid, not the 2027 `rac_advanced` alone (section 4a). The scan takes the positions of the element-data header from the reader's memo (`header_hits`, which the readers of element data share), so it makes no pass over a partition of its own once another reader has made one; it runs only for a file with such a part.

## 6. What is not done

- The five mullions without a grid id stand alone; they take the `IsExternal` of their two walls (both true) and no wall. Their wall would be a guess, and nothing here can check one.
- The door 216844 is not changed.
- RE-72's offsets stay as they are: they find RE1's grids, and nothing here shows they are wrong for them. A part with two walls no longer depends on them.
- The 2023 save of `rac_advanced` stays six values short of its 2015 to 2022 neighbours (19,973 against 19,979, [#1899](https://github.com/DrunkOnJava/rvt-rs/issues/1899)); this change closed nine of its fifteen.
- The `IsExternal` values added outside the 2027 file have no Revit value to be scored against (section 4a): they rest on the host rule (RE-288) and on the same project's other saves.
