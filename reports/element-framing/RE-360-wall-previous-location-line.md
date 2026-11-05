# RE-360: why eight host walls of `rac_advanced`'s 2027 save return no native straight-line row

**Date:** 2026-11-05
**Issues:** #1475; follows RE-347 (section 7, the same-project check of the door and window axes), RE-222 (the native wall reader), RE-348 (#1410, `location_curve` reads `VWallDriver.m_pCrv`)
**Probe:** `probes/examples/probe_re360_wall_location_curve.rs`
**Scorer:** none: the probe is read directly. No Revit export of any of these files exists.
**Status:** the cause is determined from the bytes (positive). No reader change is made: every rule that reads these walls on the 2027 save also reads walls on the 2024 to 2026 saves, or reads the same records differently by release with no difference in the bytes, so what would place the 14 openings is a decision on #1454's check, not a change this report can make by itself (section 5). That part is a measured negative.

## 0. The question

RE-347 section 7 holds each door and window placed by the axes rule to the 2024 save of the same Autodesk sample project. On `rac_advanced`'s 2027 save 14 of them (set in 8 host walls) keep the default axes, because `native_walls` returns no row for their host walls, where the 2024 save's host walls all return one. #1475 asks whether that is a native wall read or the save's own content, and what the walls' records hold.

## 1. What was measured

Run V0: Measure [42129732742](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42129732742), measured head `d741927988f73dc2f97a5ced7bd607696cc1a0fe` (the probe; no `src/` change), dispatched from `main`: `-f base=none -f samples=true -f sample_years="2024 2027" -f models="-" -f scorers=none -f probe=probe_re360_wall_location_curve`. Every job succeeded. The files are Autodesk's sample projects of the releases 2024 to 2027 pinned in `research/autodesk-sample-projects.tsv`: `rac_advanced`, `rac_basic`, `rst_basic`, `rme_advanced` and `rme_basic` of each release and the five Snowdon Towers models of 2024, 25 files. No held-out model is read.

For every `SWall` record of each file the probe prints each `GLine` and `GArc` the record holds, who names it (the owning object's class and the path of the field, from the record's own pointer table), whether it is a level line, what `VWallDriver.m_pCrv` names, and whether the current rule (exactly one curve, a level `GLine`) and `location_curve`'s (the one level `GLine` `m_pCrv` names) each read the wall. The walls are the same ElementIds in every save of one project, so each is followed across the saves by its id.

## 2. The cause

On `rac_advanced`:

| save | `SWall` records | one `GLine` | two `GLine` | read by `native_walls` |
|---|---:|---:|---:|---:|
| 2024, 2025, 2026 | 180 | 180 | 0 | 180 |
| 2027 | 180 | 162 | 18 | 162 |

- In every save, 180 of 180 records have a `GLine` owned by `VWallDriver.m_pCrv` (the path from the root is `SWall.m_pCurveDriver`, then `VWallDriver.m_pCrv`), and `m_pCrv` names exactly one object, a `GLine`, in each.
- In the 18 two-line records of the 2027 save the second `GLine` is owned by `WallRunTimeData.m_prevLocLine` (the path is `SWall.m_wallRunTimeData`, then `WallRunTimeData.m_prevLocLine`). No other curve is in any of the 180 records of any save.
- The 18 are the same walls in the 2024 save. In all 180 the length of the line `m_pCrv` names in the 2027 save is its length in the 2024 save (compared to 1e-6 ft, length only: the probe's short line does not print a line's ends).
- The 8 host walls of the 14 openings are among the 18 (the openings, by host wall: 144837 holds 1, 149505 1, 149551 1, 150342 3, 157807 5, 159427 1, 164345 1, 164417 1). The other 10 host no door or window.

So the missing row is the save's own content: Revit 2027 stores a second line in these records, and the rule "exactly one curve" counts it. The location curve itself is identifiable by its owner, as RE-348 reads it.

`m_prevLocLine` is a stored line, not a second location curve. The name reads as the previous location line, which the bytes neither confirm nor contradict. In all 14 records of the 2027 save that the probe prints whole (the 8 hosts and 6 others) it has the location curve's direction vector and length (to 1e-6 ft); it lies on it exactly in 10, 5 of the 8 hosts among them, and in the other 4 it is the same line shifted along the plan by 0.497 ft (2 walls) or 0.227 ft (2). No property of the line separates one wall from another, and a rule that told walls apart by it would be fitted to the result.

## 3. The same line in the earlier saves

`WallRunTimeData.m_prevLocLine` is not new in 2027: other records of the 2024 to 2026 saves hold it too. Per model and save, `SWall` records, those the current rule reads, those holding a `m_prevLocLine` line, and how many walls each candidate rule would read that the current rule does not:

| model | save | records | read now | `m_prevLocLine` lines | plain: `m_pCrv` alone | narrow: `m_pCrv` and `m_prevLocLine` only |
|---|---|---:|---:|---:|---:|---:|
| `rac_advanced` | 2024, 2025, 2026 | 180 | 180 | 0 | 0 | 0 |
| `rac_advanced` | 2027 | 180 | 162 | 18 | 18 | 18 |
| `rac_basic` | 2024, 2025, 2026 | 60 | 47 | 5 | 13 | 5 |
| `rac_basic` | 2027 | 60 | 46 | 7 | 14 | 6 |
| `rme_advanced`, `rme_basic` | 2024 to 2027 | 166 | 163 | 0 | 3 | 0 |
| `rst_basic` | 2024 to 2027 | 9 | 9 | 0 | 0 | 0 |
| Snowdon Towers Architectural | 2024 | 1,215 | 1,202 | 4 | 13 | 2 |
| the other four Snowdon models | 2024 | 57 | 57 | 0 | 0 | 0 |

- **Plain** reads a wall wherever `m_pCrv` names one level `GLine`, whatever else the record holds. The walls it adds beyond the narrow rule's hold lines owned by `CurveLoop.m_curves`: `rac_basic` 8 (2024 to 2026) and 8 (2027, 7 of them with no other line and 1 that also holds a `m_prevLocLine` line), `rme_advanced` and `rme_basic` 3 each, Snowdon Architectural 11 (9, and 2 that also hold a `m_prevLocLine` line). A wall with an edited elevation profile holds several curves (#1410), and `native_walls` leaves any wall with more than one out; the owner is what tells these records from the `m_prevLocLine` ones.
- **Narrow** reads a wall where `m_pCrv` names one level `GLine` and every other `GLine` or `GArc` of the record is owned by `m_prevLocLine` alone. It is the rule the 2027 `rac_advanced` walls need and nothing wider.
- The 2027 `rac_basic` wall 234869 is the one that changes with the save: one `GLine` in 2024 to 2026 (read), two in 2027 (not read), with a `m_prevLocLine` line that lies on its location curve. The 5 `rac_basic` walls the narrow rule adds in 2024 to 2026 (709245, 628450, 628452, 628454, 628456) hold the same two lines in the same places in every save, each `m_prevLocLine` line on its `m_pCrv` line.
- In the files with a read, `m_pCrv` and the current rule name the same line on all of them: "read by both, same line" is 180 of 180 on `rac_advanced` 2024 to 2026, 162 of 162 on 2027, 47 of 47 and 46 of 46 on `rac_basic`, 163 of 163 on `rme` and 1,202 of 1,202 on Snowdon Architectural; no wall is read by the current rule alone and none by both with a different line.

## 4. What each rule would reach

Walls newly read, by model and save, from section 3. Doors and windows set in them are the openings left on the default axes.

| rule | 2024 | 2025 | 2026 | 2027 |
|---|---|---|---|---|
| plain | `rac_basic` 13 (hosting 9 openings), `rme` 3 and 3, Snowdon Architectural 13 | `rac_basic` 13 (9 openings), `rme` 3 and 3 | `rac_basic` 13 (8 openings), `rme` 3 and 3 | `rac_advanced` 18 (14 openings), `rac_basic` 14 (8), `rme` 3 and 3 |
| narrow, every release | `rac_basic` 5, Snowdon Architectural 2 | `rac_basic` 5 | `rac_basic` 5 | `rac_advanced` 18 (14 openings), `rac_basic` 6 |
| narrow, 2027 only | none | none | none | as the row above |

None of the walls the narrow rule adds in 2024 to 2026 hosts a door or window whose flips are read, so no placement on those saves would move by the axes rule; what each adds to the wall's own fields and its neighbours' is listed in section 6 and is not measured.

## 5. Why no reader change is made here

The bar for this lane: reading `m_pCrv` must move nothing on 2024 to 2026, and the walls read on 2027 must equal the 2025 save's. Measured:

- **Plain** moves 2024 to 2026: 13 `rac_basic` walls in each of the three saves (3 of them host 9 openings in 2024 and 2025 and 2 host 8 in 2026, which would take axes), 3 walls in each of `rme_advanced` and `rme_basic` in each save and 13 in Snowdon Architectural. Fails.
- **Narrow, every release** moves 2024 to 2026: 5 `rac_basic` walls in each of the three saves and 2 in Snowdon Architectural (17 wall reads, 7 distinct walls), none of them a host. On 2027 it reads 18 more `rac_advanced` walls and 6 more `rac_basic` walls, so the 2027 save's walls equal the 2025 save's (180 and 52) only because the 2025 save changes the same way. Fails the bar as set, though every change is of walls the bytes show alike.
- **Narrow, 2027 only** moves nothing on 2024 to 2026 by construction, and reads on 2027 `rac_advanced` 180 walls, which are the 2025 save's 180 walls; but on 2027 `rac_basic` it reads 52 (46 and 6), where the 2025 save reads 47: the 5 walls named in section 3, whose two lines are the same in both saves. A reader that treats the same bytes differently by release has no byte reason for it, so that is not fail-closed, it is a gate on the release fitted to the result. Not built.

Nothing was built or run beyond the probe: the second read and a Measure run of any rule are for the decision below.

**What is left open for #1454's check.** 14 openings of `rac_advanced`'s 2027 save (8 host walls, no other model) keep the default axes, and the check compares each opening with the 2025 save, so those 14 differ on axes alone (their positions and world vertices are equal). The choice, in numbers:

1. Read by `m_pCrv`'s identity with `m_prevLocLine` ignored, on every release (narrow, every release): the 2024 to 2026 saves read 17 more walls, 7 distinct (`rac_basic` 5 in each of three saves, Snowdon Architectural 2 in one), none a host; the 2027 save reads `rac_advanced` 18 and `rac_basic` 6.
2. The same on the 2027 save alone: no earlier save changes, and 2027 `rac_basic` reads 5 walls that its 2025 save does not.
3. No change: the 14 stay on the default axes and the check on the 2027 save needs a decision.

## 6. What reads the map

`native_walls_and_top_joins_into`'s `found` map is what the "exactly one curve" rule fills. Everything that reads it is in `partition_schema_mvp/walls.rs` and `wall_joins.rs`, on Revit 2024 to 2027:

- the placement rule's wall direction (`m_wall_native_direction_*`, `attach_opening_placement_axes`, RE-347), which is the reading this report is about;
- `m_wall_type_confirmed`, from the record's type;
- the orientation fallback (the record's word and flip where the partition data gives none);
- `attach_wall_butt_joins` (the walls' `shells`), `attach_native_tee_reaches`, `attach_wall_corner_reaches` and `joined_at_ends`;
- `attach_wall_lengths`: the curve length, the cut-back length and the base-quantity `Length`, curtain walls included.

`trimmed_ends` returns nothing where a partner wall is not in the map, and a corner counts a partner missing from it as another cutter, so a newly read wall can change a neighbour's reaches and lengths, not only its own. The wall cuts (`top_cutters`, `pair_only`, `base_cutters`, `whole`, `bounded`), `is_top_joined`, `uncut` and RE-348's `end_joins` do not depend on it: they are read for every `SWall` and `ArcWall` record. None of the changes in these fields was measured here.

## 7. Not claimed, not measured

- No Revit export of any file read here exists: the saves are held to each other, not to Revit.
- What reading a wall changes in an export: not measured, since no rule was built. The consumers are listed, and the per-save `ifc.sha256` difference of a run against `main` would show which saves move.
- That the narrow rule's walls are read right: the line `m_pCrv` names is the one `location_curve` reads (RE-348) and equals the current rule's on every wall both read (section 3); nothing checks the 18 and 6 walls beyond that.
- Doors and windows with no flips read, and hosts with no `SWall` record.
- ArcWall records, curtain walls, Revit 2023 and older.

## 8. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=d741927988f73dc2f97a5ced7bd607696cc1a0fe -f base=none \
  -f samples=true -f sample_years="2024 2027" -f models="-" -f scorers=none \
  -f probe=probe_re360_wall_location_curve
```

The output of each file is `sample-<year>-<project>/probe.txt` in the artifact `measure-head-all`: one JSON object per line (`file`, `openings`, `unread`, `swall`, `wall`, `census`). The census line holds the shapes, the curve owners, what `m_pCrv` names and the agreement table of section 3; a `swall` line is one wall.
