# RE-360: a wall's record holds its previous location line beside its location curve

**Date:** 2026-11-05
**Issues:** #1475; follows RE-347 (section 7, the same-project check of the door and window axes), RE-222 (the native wall reader), RE-348 (#1410, `location_curve` reads `VWallDriver.m_pCrv`)
**Probe:** `probes/examples/probe_re360_wall_location_curve.rs`
**Scorer:** none: no Revit export of the files measured here exists. The saves are held to each other (`tools/re/bodies_across_saves.py`) and the two builds to each other.
**Status:** positive, and the rule is written. On Revit 2027's save of `rac_advanced`, 18 of 180 `SWall` records hold a second `GLine` besides their location curve, owned by `WallRunTimeData.m_prevLocLine`; `native_walls` left out any wall with a second curve, so the 8 host walls of 14 doors returned no native line and the doors kept the default axes (#1475). `native_walls` now reads a wall whose record holds exactly two curves, the level `GLine` `VWallDriver.m_pCrv` names and a `GLine` only `WallRunTimeData.m_prevLocLine` names, by the first. Every wall the old rule read is read as before. The 14 doors are placed by the axes rule and equal to the 2025 save's; no wall that was read moves; 26 walls are read that were not, counted over every `SWall` record of the 25 files (18 of `rac_advanced` 2027, 6 of `rac_basic` 2027 and 5 of them on 2024 to 2026, 2 of Snowdon Architectural), of which 20 can reach an export (the 18 of `rac_advanced` 2027, and `rac_basic`'s 709245 and 234869; none on Snowdon). What stays open is in section 5.

## 0. The question

RE-347 section 7 holds each door and window placed by the axes rule to the 2024 save of the same Autodesk sample project. On `rac_advanced`'s 2027 save 14 of them (set in 8 host walls) kept the default axes, because `native_walls` returned no row for their host walls, where the 2024 save's host walls all return one. #1475 asks whether that is a native wall read or the save's own content, and what the walls' records hold.

## 1. The cause

Run V0: Measure [42129732742](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42129732742), measured head `d741927988f73dc2f97a5ced7bd607696cc1a0fe` (the probe; no `src/` change), dispatched from `main`: `-f base=none -f samples=true -f sample_years="2024 2027" -f models="-" -f scorers=none -f probe=probe_re360_wall_location_curve`. Every job succeeded. The files are Autodesk's sample projects of the releases 2024 to 2027 pinned in `research/autodesk-sample-projects.tsv` (25 files). No held-out model is read. For every `SWall` record the probe prints each `GLine` and `GArc`, who names it (the owner's class and the path of the field, from the record's own pointer table), whether it is a level line, and what `VWallDriver.m_pCrv` names. The walls are the same ElementIds in every save of one project.

| save of `rac_advanced` | `SWall` records | one `GLine` | two `GLine` | read by the old rule |
|---|---:|---:|---:|---:|
| 2024, 2025, 2026 | 180 | 180 | 0 | 180 |
| 2027 | 180 | 162 | 18 | 162 |

- In every save, 180 of 180 records have a `GLine` owned by `VWallDriver.m_pCrv` (the path from the root is `SWall.m_pCurveDriver`, then `VWallDriver.m_pCrv`), and `m_pCrv` names exactly one object, a `GLine`, in each.
- In the 18 two-line records of the 2027 save the second `GLine` is owned by `WallRunTimeData.m_prevLocLine` (`SWall.m_wallRunTimeData`, then `WallRunTimeData.m_prevLocLine`). No other curve is in any of the 180 records of any save.
- The 18 are the same walls in the 2024 save, and in all 180 the length of the line `m_pCrv` names in the 2027 save is its length in the 2024 save (1e-6 ft). That is a check of length alone, and the two lines of a 2027 record have equal lengths, so it does not say which of them is the location curve; what does is the next section.
- The 8 host walls of the 14 doors are among the 18 (144837 holds 1 door, 149505 1, 149551 1, 150342 3, 157807 5, 159427 1, 164345 1, 164417 1). The other 10 host no door or window. The 2024 save's 8 host walls hold 15 doors and the 2027 save's hold 14: the 15th, door 164921, is hosted by 157807 in the 2024 save and by another wall (164144) in the 2027 save, where it is not placed. That is the host rebinding of #1454, not this cause.

`m_prevLocLine` is a stored line, not a second location curve. The name reads as the previous location line, which the bytes neither confirm nor contradict. In all 14 records of the 2027 save the probe printed whole (the 8 hosts and 6 others) it has the location curve's direction vector and length (1e-6 ft); it lies on it exactly in 10, 5 of the 8 hosts among them, and in the other 4 it is offset across the wall, not along it (the component along the wall is 0.000000 ft), by half the wall type's thickness: 0.4970 ft of 0.9941 ft on 144837 and 148769, 0.2272 ft of 0.4544 ft on 150342 and 157807 (3 of the 4 are hosts). So it is a line parallel to the location curve at a face of the wall, and in the other 10 coincident with it.

**What settles which line is the location curve.** Their lengths cannot, and a rule that told walls apart by the previous line's properties would be fitted to the result. Three things do, each measured on the head of run V1 (section 3):

- `m_pCrv`'s line equals the independent partition axis (RE-53, read from the partition stream, not from the native record) to 0.0000 ft at both ends on all 13 of the 14 dumped walls that have an axis (all 8 hosts among them; the 14th is a curtain wall, which has none). The previous line equals that axis on 9 of the 13 and is 0.4970 (2 walls) or 0.2272 ft (2) off it on 4, 3 of them hosts.
- `attach_wall_lengths` (`wall_joins.rs`, its `near` test) gives a plain wall `m_wall_curve_length` only where the native line's two ends are within 1e-4 ft of that axis. All 17 newly read plain walls of `rac_advanced` 2027 have it (and an axis), so the line read is the axis, within 1e-4 ft, on all 17; the previous line would have failed that test on the 4 whose offset is 0.4970 or 0.2272 ft.
- `m_pCrv` names the location curve in all 180 records of every save, and `location_curve` has read it since RE-348 (#1410).

What cannot be told apart is a record whose `m_pCrv` names the stale line: the rule trusts the pointer, and nothing in the record besides the pointer says which line is current. None exists in the 25 files (the axis check above, and `m_pCrv`'s length equal to the 2024 save's on all 180). A curtain wall has no axis and no such guard (the curtain wall branch of `attach_wall_lengths`), so only the rule protects its line: both curtain walls this change reads (`rac_basic` 709245 and `rac_advanced` 149877) have a previous line coincident with the location curve, so a misreading would give the same value.

The same member is in other records of the 2024 to 2026 saves: `rac_basic` holds it in 5 (7 on 2027) and Snowdon Towers Architectural in 4 (2 of them also hold `CurveLoop.m_curves` lines, a profile). So a rule that reads these walls on the 2027 save reads walls on the earlier saves too.

## 2. The rule

`record_line` in `src/research/native_walls.rs`, on every release `native_walls` reads:

- a record whose curves are one `GLine`: that line, as before;
- a record whose curves are two: the level `GLine` that `VWallDriver.m_pCrv` names (one object, a `GLine`: `location_curve_index`, RE-348's reading of the pointer), when the other is a `GLine` that only `WallRunTimeData.m_prevLocLine` names (every pointer to it, and there is one at least, is that member of a `WallRunTimeData` object: `is_previous_location_line`);
- any other set of curves, an arc among them, `m_pCrv` null or naming several objects, or a second line anything else names: not read.

On the 25 files the second branch reads exactly the walls with two curves and a `m_prevLocLine` line: 18 on `rac_advanced` 2027, 5 on `rac_basic` 2024 to 2026 and 6 on 2027, 2 on Snowdon Architectural. It does not read the walls whose second curve is a `CurveLoop.m_curves` line (a plain `m_pCrv` rule would: `rac_basic` 8 more, `rme_advanced` and `rme_basic` 3 each and Snowdon Architectural 11, in each save that holds them), and a wall the old rule read is read by the first branch, so no wall is un-read.

## 3. What was measured

Run V1: Measure [42134051521](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42134051521), head `0a0ba7ce498f188072ce6719c037f55e84edde5b` (the rule) against base `ce3a3c4731e46b847e7b3c16a1eea16a445e7405` (`main` at `daf36102` with this branch's probe, report and docs, no `src/` change), dispatched from `main`: `-f base=<base> -f samples=true -f sample_years="2024 2027" -f models="-" -f probe=probe_re360_wall_location_curve -f keep_ifc=true`, every default scorer. Every job succeeded. The `same-project gates` check run on the head reads `pass: 9 samples held to their earlier saves, and the Areas on 4 saves`. The files are the 25 of section 1. The probe, on each build, prints the walls `native_walls` reads, every field of each wall with an `SWall` record and each door and window.

### 3.1 What each save reads

Walls `native_walls` reads of the `SWall` records, base then head:

| model | 2024 | 2025 | 2026 | 2027 |
|---|---|---|---|---|
| `rac_advanced` (180 records) | 180, 180 | 180, 180 | 180, 180 | 162, 180 |
| `rac_basic` (60) | 47, 52 | 47, 52 | 47, 52 | 46, 52 |
| `rme_advanced` and `rme_basic` (166 each) | 163, 163 | 163, 163 | 163, 163 | 163, 163 |
| `rst_basic` (9) | 9, 9 | 9, 9 | 9, 9 | 9, 9 |
| Snowdon Architectural (1,215) | 1,202, 1,204 | | | |
| Snowdon Electrical (1), Structural (56) | 1, 1 and 56, 56 | | | |
| Snowdon HVAC and Plumbing (0) | 0, 0 | | | |

By id, on the head `rac_basic` reads the same 52 walls on all four saves and `rac_advanced` the same 180 on all four (the other three projects the same 163, 163 and 9). On the base, `rac_advanced` 2027's 162 lack the 18 and `rac_basic` 2027's 46 lack 234869; so the +5 of 2024 to 2026 and the +6 of 2027 are one baseline apart (47 and 46), not two different sets.

### 3.2 What moves

`ifc.sha256` moves on `rac_basic` 2024, 2025, 2026 and 2027 and on `rac_advanced` 2027, and on no other save: `rac_advanced` 2024 to 2026, `rme_advanced`, `rme_basic` and `rst_basic` of every year and all five Snowdon models are byte for byte the base's. The compare job's diff of every scorer output holds only the probe's output of six samples, `bodies_across_saves.txt` of `rac_advanced` 2027 and the head-only `areas_across_saves.txt`: the wall rule counts, stage counts and schema findings are identical on every sample.

Every product of the five moved exports, by GlobalId, base against head (class, Tag, placement, every property and quantity, body volume and world box, IfcOpeningElement included):

| export | products | what differs |
|---|---:|---|
| `rac_basic` 2024, 2025, 2026, 2027 | 567, 561, 560, 560 | one value each, the same: `IfcCurtainWall` 709245 gains `Qto_CurtainWallQuantities.Length` 3.01 m (9.8753 ft: its curve's length, and its record box is 9.8753 ft wide). No placement, volume or world box moves, nothing added or removed |
| `rac_advanced` 2027 | 5,827 | `IfcCurtainWall` 149877 gains `Length` 25.6109 m (84.0253 ft, the value its 2024, 2025 and 2026 saves write); 14 doors gain `PlacementAxesSource`; 26 placements move, 13 doors and the 13 openings they cut (door 149921's axes were already the rule's); no body volume or world box moves, nothing added or removed |

The wall fields the reader feeds, by wall, base against head (the probe's field dump of every wall with a record; the figures are the head's value and none was present on the base):

- `rac_advanced` 2027: 18 walls gain `m_wall_native_direction_x` and `_y` and `m_wall_type_confirmed`, 17 gain `m_wall_curve_length` (6.5617 to 122.0028 ft), one (149877, the curtain wall above) `m_wall_length` 84.0253 ft;
- `rac_basic` 2024 to 2026: 709245 gains the direction, `m_wall_type_confirmed` and `m_wall_length` 9.8753 ft; the other 4 newly read walls (628450, 628452, 628454, 628456) are not walls of the export (the probe's dump holds no wall element with their ids), so no field and no element of it changes; 709245 is a curtain wall;
- `rac_basic` 2027: the same, and 234869 gains the direction, `m_wall_type_confirmed` and `m_wall_curve_length` 15.2920 ft (none of which is written to the IFC).

**No neighbour moves.** On every moved save the walls with a changed field are exactly the newly read ones that the export holds (18, 1, 1, 1 and 2): no other wall gains or changes a length, a cut-back length, a join reach, a partner or a shell, and the opening cuts and the other wall cuts are not read from this map. The compare of the 2027 save to the 2024, 2025 and 2026 saves, over these fields by wall id: on the base `rac_advanced` is equal on 162 of 180 walls (the 18 differ) and `rac_basic` on 55 of 56 (234869 differs); on the head, all 180 and all 56, for each of the three saves.

### 3.3 The 16 pinned walls of `rac_basic`

`PINNED_WALLS` of `lane/1454-gates` holds the tags 234869, 428588, 428745, 429964, 430064, 430318, 430361, 430412, 430859, 497540, 506386, 506797, 768442, 937935, 938974 and 939084 on the 2026 and 2027 saves. Of the walls this change reads that were not read, one is among them: 234869, on 2027. Its body, volume, world box and every property in the exported IFC are the base's (the table above holds one changed value on `rac_basic` 2027, a curtain wall that is not pinned), so its pin is unaffected and nothing heals. 428745, also pinned and the host of 4 doors, stays unread (its curves are `CurveLoop.m_curves` lines). This report does not re-pin anything.

### 3.4 Snowdon Towers Architectural

The head's export is byte for byte the base's (`ifc.sha256` equal) and none of the 1,120 wall elements with a record, nor any door or window, has a field that differs between the two builds. The 2 newly read walls are not in the export: no product of either build holds their Tag, nor does Revit's own IFC4 export of the file (local, no licence stated, counts only: 0 of 2 in the export, 0 of 2 in Revit's). So there is nothing to score for them and no wall moves from Revit's; the read reaches no output on this file.

### 3.5 The 14 doors of `rac_advanced` 2027

The head places all 14 by the rule (`PlacementAxesSource` `stored_axes_reversed_by_flips`); the base placed none of them by it, and 13 had axes differing from the 2025 save's by 1 or 2 (the 14th's default axes happen to be the rule's). Per Tag, against the 2025 save's export and the 2024 save's (each head against head: class, storey, representation, placement axes, origin, body volume and vertices, GlobalId left out): 14 of 14 equal on both, axes differing by 0 and the origin 0.00000 ft apart; on the base, 1 of 14.

The gate (`tools/re/bodies_across_saves.py`, the doors and windows group, held to the 2024 save) on `rac_advanced` 2027: base compared 65 (47 doors, 18 windows), left out 42 (36, 6), 31 off the default axes; head compared 79 (61, 18), left out 28 (22, 6), 44 off the default axes, the 2026 save's counts. All 9 samples read `bodies across saves: PASS`. The 28 are the host rebinding of #1454, a different cause.

**The cut openings.** The 14 openings those doors cut now have the placement of the same openings of the 2025 save and of the 2024 save (axes differing by 0 on all 14; on the base 13 of 14 differed by 1 or 2). Their volume and farthest vertex still differ from both saves' on all 14, before and after this change (for example 1.26024 m3 against 1.11862 m3, the farthest vertex 0.3526 ft away): the cut loop of a door's symbol is read on `CUT_LOOP_REVIT_VERSIONS`, 2024 and 2025 only, so a 2027 opening is its record's box. #1454 widens that list; whether the 14 then equal the 2025 save's in vertices too was not measured here, since the two changes are not on one head.

## 4. What reads the map

`native_walls_and_top_joins_into`'s `found` map is what the rule fills. Everything that reads it is in `partition_schema_mvp/walls.rs` and `wall_joins.rs`, on Revit 2024 to 2027: the placement rule's wall direction (`m_wall_native_direction_*`, `attach_opening_placement_axes`), `m_wall_type_confirmed`, the orientation fallback, `attach_wall_butt_joins`, `attach_native_tee_reaches`, `attach_wall_corner_reaches`, `joined_at_ends` and `attach_wall_lengths` (the curve length, the cut-back length and `Length`, curtain walls included). `trimmed_ends` returns nothing where a partner is not in the map, and a corner counts a partner missing from it as another cutter, so a newly read wall can change a neighbour's reaches and lengths; on these 25 files none does (section 3.2). The wall cuts, `is_top_joined`, `uncut` and RE-348's `end_joins` do not depend on it: they are read for every `SWall` and `ArcWall` record.

## 5. Not claimed, not measured

- No Revit export of `rac_basic` or `rac_advanced` exists: the saves are held to each other, so the newly read walls' lengths and directions are not checked against Revit's.
- The two curtain walls' `Length` values (709245 on every save, 149877 on 2027) are held by no check on these two models. No gate compares a curtain wall's `Length`; 709245 gains it on all four saves at once, so no earlier save compares it, and only its record box's width of 9.8753 ft corroborates the value 9.8753 ft; 149877's equals the value its 2024 to 2026 saves write (a comparison of the exports made for this report, not a gate), and its join-trimmed box is 82.3848 ft deep against the 84.0253 ft written. The value is the curve's length, as for every other curtain wall that was read.
- The 14 openings' size on 2027 (section 3.5).
- What other files hold: a record with two curves where the second is not a `m_prevLocLine` line, or with several previous location lines, is not read; a held-out model was not read for this.
- Doors and windows with no flips read, hosts with no `SWall` record, ArcWall records, Revit 2023 and older (the native read does not run there).

## 6. The gate

The 2027 floor of `OPENINGS_LEAST` for `rac_advanced` goes from 47 doors to 61 and the ceiling `OPENINGS_LEFT_OUT_MOST` from 42 to 28 (`tools/re/bodies_across_saves.py`), as RE-347 requires of a count that moves. Neither measured run exercised that edit: V0 and V1 were dispatched from `main` and took `main`'s scorers, and the `same-project gates` check run on this PR's head (Measure run [42137808608](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42137808608), a pass on 9 samples) reads "Scored by workflow commit `c5bae9ed`", `main`'s gate, whose 2027 ceiling is 42. The run that scores 61, 18 and 28 is the unfiltered one dispatched from this branch (V2, below); its figures are not in this report yet.

These are the numbers on `main` with this change. #1454's branch (`lane/1454-gates`) holds its own 2027 numbers, counted with this cause present: 68 doors, 24 windows and a ceiling of 15. This change's 61, 18 and 28 would loosen them, and #1454's own measured numbers once this has landed are 83, 24 and 0 (the 2026 save's). So whichever of the two lands second sets 2027's numbers from a samples run on the merged head, and neither takes the other's.

## 7. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=d741927988f73dc2f97a5ced7bd607696cc1a0fe -f base=none \
  -f samples=true -f sample_years="2024 2027" -f models="-" -f scorers=none \
  -f probe=probe_re360_wall_location_curve                                    # V0, the cause
gh workflow run measure.yml --ref main -f ref=0a0ba7ce498f188072ce6719c037f55e84edde5b \
  -f base=ce3a3c4731e46b847e7b3c16a1eea16a445e7405 -f samples=true -f sample_years="2024 2027" \
  -f models="-" -f probe=probe_re360_wall_location_curve -f keep_ifc=true     # V1, the rule
gh workflow run measure.yml --ref <this branch> -f ref=<its head> -f base=main -f samples=true \
  -f nordic=true -f r2025=true -f holdout=true -f keep_ifc=true \
  -f probe=probe_re360_wall_location_curve                                    # V2, unfiltered
```

V2 is the unfiltered run, dispatched from this branch so that its scorers include the edited gate; it is not yet read.

The output of each file is `sample-<year>-<project>/probe.txt` in the artifacts `measure-head-all` and `measure-base-all`, one JSON object per line (`file`, `openings`, `unread`, `swall`, `wall`, `census`, `native`, `wallfields`, `opening`). With the kept IFCs of V1 downloaded, the products compared in section 3.2 are each export's `IfcProduct`s by GlobalId, and section 3.5's doors are `tools/re/bodies_across_saves.py`'s `differences` of the head's 2027 and 2025 exports by Tag.
