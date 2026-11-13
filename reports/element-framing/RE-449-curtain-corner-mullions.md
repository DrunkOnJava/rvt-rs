# RE-449: a curtain part that names two curtain walls takes the one its grid's data names: 74 of the 79 corner mullions of `rac_advanced` 2027 get their wall back

**Date:** 2026-11-13
**Issues:** #1865 (79 curtain members of Autodesk's `rac_advanced` saved by Revit 2027 get no `IsExternal` where the 2025 save writes it); follows RE-46 (a part names exactly one curtain wall), RE-62 (a part that names none takes the wall whose record box holds it), RE-72 (a part's grid names its wall) and RE-179 (79 corner mullions take no curtain wall on 2027)
**Artefacts:** Autodesk's sample projects `racadvancedsampleproject.rvt` of Revit 2025 (sha256 `512e9188b245ba58fcc918630e4388b836df37042aa07de6db0e223b48de25d1`, 17,068,032 bytes) and Revit 2027 (`a88eb9f85e1e2d47bbc91ee214b178c2d7075822b6d08694f59e3e6f259024ed`, 17,133,568 bytes), both public and redistributable (`research/autodesk-sample-projects.tsv`). The same project saved by the two releases: no Revit export of either exists, so the 2027 save is held to the 2025 save element by element, as #1861 was.
**Probes:** `probes/examples/probe_1865_curtain_is_external.rs` (#1877), `probe_1865_part_walls.rs` (#1879) and `probe_1865_grid_walls.rs` (#1890)
**Runs:** Measure [43379354161](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43379354161) (`probe_1865_curtain_is_external`), [43381092468](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43381092468) (`probe_1865_part_walls`), [43382999619](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43382999619) and [43384818097](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43384818097) (`probe_1865_grid_walls`), each on `rac_advanced` 2025 against its 2027 save (`probe_pair_release=2027`)
**Status:** Positive for 74 of the 79 mullions; a measured negative for the other 5 (no id of their lists has element data that names a wall) and for the curtain door 216844, which loses its wall on 2027 for another cause that is not found.

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

**Only these 79 name two walls.** The one-file counts of `probe_1865_part_walls` over the 16 models of its run have `named2` above 0 only on the 2027 `rac_advanced` mullions (79, none aggregated), and `named3plus` is 0 everywhere. A rule for this arm reaches no other model Measure holds.

**The door.** The curtain door 216844 (`M_Curtain Wall Sgl Glass`) is aggregated into wall 162023 on 2025 and into none on 2027, names that one wall on both saves and is `(0, 1)` on both, so the match gives it the wall on both. Its loss has another cause, not found here, and it is unchanged by this report's change.

## 3. The rows (registered on #1865 before each run)

Comments [6845864311](https://github.com/DrunkOnJava/rvt-rs/issues/1865#issuecomment-6845990245) (the first probe), [6846024013](https://github.com/DrunkOnJava/rvt-rs/issues/1865#issuecomment-6846024013) (row 5, from RE-179), [6846247941](https://github.com/DrunkOnJava/rvt-rs/issues/1865#issuecomment-6846247941) (the list probe), [6846501673](https://github.com/DrunkOnJava/rvt-rs/issues/1865#issuecomment-6846501673) (the grid probe) and [6846957690](https://github.com/DrunkOnJava/rvt-rs/issues/1865#issuecomment-6846957690) (the change). The expectation I had that failed: that the 79 are the parts of three curtain walls whose boxes `curtain_wall_box_not_read` points to; they leave six walls and none of the three is involved, and the diagnostics line has no bearing on them.

## 4. What Measure found

*(added with the run of the change)*

## 5. The change

`scan_walls_in_element_data` (new, `curtain.rs`) reads, for the ids of the lists of the parts that name two or more curtain walls and no grid, which curtain walls each id's element data names: those whose ElementId, as a `u64`, lies at least twice within 1,200 bytes past the id, where the release's element-data header opens the data. `attach_curtain_walls` takes, for a part with no grid and two or more named walls, the one wall of its list that these name, where exactly one does. A part for which that gives no one wall stands alone and keeps the walls it names (`m_curtain_walls_named`); `attach_curtain_member_is_external` gives such a part the `IsExternal` all its walls have, where each has one and they agree.

## 6. What is not done

- The five mullions without a grid id stand alone; they take the `IsExternal` of their two walls (both true) and no wall. Their wall would be a guess, and nothing here can check one.
- The door 216844 is not changed.
- RE-72's offsets stay as they are: they find RE1's grids, and nothing here shows they are wrong for them. A part with two walls no longer depends on them.
