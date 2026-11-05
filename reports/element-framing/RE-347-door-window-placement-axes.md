# RE-347 — The local axes rvt-rs writes for a door or window, against Revit's

**Date:** 2026-11-05
**Issues:** #1447; follows RE-343 (the hand and facing flips, #1357), RE-87 (instance transforms) and RE-84 and RE-215 (openings hang off the filler's placement)
**Probe:** `probes/examples/probe_re343_door_flips.rs`
**Scorer:** `tools/re/door_placements_vs_ifc.py`
**Status:** positive, and the rule is written. Before it, rvt-rs wrote the default axes (X along the model's +X, Z up) on every door and window it exports, and Revit's axes were the same as those on 115 of the 521 doors and 226 of the 540 windows that rvt-rs binds to a plain wall in the open models (counted as run, the permit office file included), exactly where Revit's X happens to be the model's +X. The difference is not invisible: rvt-rs writes no `OperationType`, so the placement's axes are all its IFC says of which way a door faces. The rule of RE-343 (the stored axes, reversed where the flips differ) is now written for every door and window rvt-rs binds to a plain wall: 1,061 of 1,061 in the open models and 145 of 146 in the held-out models of 2024 to 2027, and Revit's axes are the same on all of them (0 wrong). Every body and every opening is where it was in world coordinates, and no GlobalId, type GlobalId or host moved. What it leaves out is stated in section 5. The origin is a separate difference (#1465) and is not moved.

## 0. The question

RE-343 measured Revit's `IfcLocalPlacement` axes against the axes the instance stores, and left open what rvt-rs writes for the same elements (it takes the stored X only for a box turned off the model's axes, `turned_box`, RE-87). #1447 asks for that measurement, tabulated by the two flips and by whether the element is bound to a plain wall, and then either a rule with 0 wrong on walled elements or a closure as invisible.

## 1. What was measured

`tools/re/door_placements_vs_ifc.py` pairs, by Tag, each door and window of the probe with the `IfcDoor` or `IfcWindow` of Revit's export and of rvt-rs's own export. It composes each element's `IfcLocalPlacement` chain, moves the X, Y and Z axes into Revit's internal frame through that file's own `IfcSite` placement (rotation only; rvt-rs's site is the identity on every model here), and gives one verdict per element:

- `same`: the three axes the same;
- `half turn`: X and Y reversed, Z the same;
- `quarter turn`: X and Y a right angle from Revit's, Z the same;
- `other`: anything else (a tilt, or a building turned off the model's axes, where Revit's X is neither of the four plan directions).

The binding is RE-343's Rule 1 subset: `bound to a plain wall` where the probe read the stored transform and the wall rvt-rs binds to has a native record with a curve; `bound to none` where it read the transform and no such wall; `no stored transform` where it read none (Revit 2023 and older). The state is `m_flippedX m_flippedY` of the native record.

Run V0 (the measurement, sections 1 to 4): Measure [42066757105](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42066757105), measured head `eb6de281f64761045ef01fdae6026c0a0b522b0b` (the code of `main` at `d6f69d97` and the scorer; no `src/` change), dispatched from `main`: `-f base=none -f scorers=none -f keep_ifc=true -f nordic=true -f holdout=true`, the probe and scorer above, `-f models="core-interior re1-architecture nordic-office-as-built nordic-office-permit holdout-*"`. Every job that ran succeeded. The held-out lines are counts the scrub kept (`door flips:` in `probe_score*.txt`).

**The denominators are RE-343's.** The elements bound to a plain wall are Core Interior 132 doors and 6 windows, RE1 Architecture 4 doors, the office as built 193 doors and 269 windows, the office at permit 192 doors and 265 windows, and the doors bound to none are RE1's 1, the office's 76 and 77: the figures of RE-343's Rule 1 table, so the probe's host read is the same on this head. Every door and window of every model, the held-out ones included, is in both exports (0 missing from either), and the `Ifc` class agrees on all of them.

## 2. Before the rule: rvt-rs wrote the default axes everywhere

`rvt-rs axes are the default axes` (X along +X, Z up) equals the number checked on every line of the open models: 521 of 521 bound doors, 154 of 154 doors bound to none, 540 of 540 bound windows. In the held-out models of 2024 to 2027, 41 of 41 bound doors and 104 of 105 bound windows; the one window that is not default is not looked at further (`turned_box`, RE-87, is the only code that writes a rotation, so it is the likely case, not checked), and the 385 doors and 525 windows of the held-out models of 2018 to 2023 are default on all of them. rvt-rs's Z axis is up on every element of every model.

## 3. Before the rule: against Revit's, bound to a plain wall

Open models, the three that are different buildings (Core Interior, RE1 Architecture, the office as built):

| | checked | same | half turn | quarter turn | other | differ |
|---|---:|---:|---:|---:|---:|---:|
| doors | 329 | 78 | 115 | 136 | 0 | 251 |
| windows | 275 | 113 | 103 | 59 | 0 | 162 |

As run, with the permit office file (the office again, 192 doors and 265 windows bound):

| | checked | same | half turn | quarter turn | other | differ |
|---|---:|---:|---:|---:|---:|---:|
| doors | 521 | 115 | 189 | 217 | 0 | 406 |
| windows | 540 | 226 | 202 | 112 | 0 | 314 |

Per model, doors then windows, differing of checked: Core Interior 92 of 132 and 6 of 6; RE1 3 of 4; the office as built 156 of 193 and 156 of 269; at permit 155 of 192 and 152 of 265. `same` is where Revit's X is the model's +X, which is a fact about the walls, not about the flips.

By state, the open models as run (doors then windows; `same` / `half turn` / `quarter turn`, no `other`):

| state | doors checked | doors: same / half / quarter | windows checked | windows: same / half / quarter |
|---|---:|---|---:|---|
| 00 | 91 | 22 / 27 / 42 | 52 | 0 / 0 / 52 |
| 01 | 114 | 25 / 48 / 41 | 282 | 226 / 2 / 54 |
| 10 | 129 | 20 / 49 / 60 | 2 | 0 / 2 / 0 |
| 11 | 187 | 48 / 65 / 74 | 204 | 0 / 198 / 6 |

So the mixed flip state of RE-343 (`01` and `10`: 287 of 653 doors and 286 of 626 windows) is only part of the difference. In the states where the flags are equal (`00`, `11`) Revit's axes are the stored ones, and rvt-rs's still differ from them wherever the wall does not run along +X: 22 of 91 doors in `00` agree with Revit's, 48 of 187 in `11`. The quarter turns (217 doors, 112 windows) are elements whose Revit X is along Y, which is the wall's direction (RE-343 Rule 2), so walls along Y; the half turns are elements whose Revit X is -X, a wall running the other way or the reversal of a mixed flip state.

**RE-343's rule reproduces on this head.** From this run's per-element rows (stored X against rvt-rs's X, and the state), the verdict each element should have if Revit's placement is the stored axes reversed where `m_flippedX` differs from `m_flippedY` is the verdict it has on 521 of 521 doors and 540 of 540 windows bound to a plain wall. It is the same elements as RE-343's, so it repeats that measurement through a second path (the rvt-rs axes are the fixed point); it is not new evidence for the rule.

**Doors bound to none** (154 of the open models as run, 77 counted once): the same default axes, and 118 of 154 differ (36 same, 71 half turn, 47 quarter turn). They are outside the rule (section 5).

**Held-out models, counts only.** The 2024 to 2027 ones (41 doors and 105 windows bound to a plain wall; four releases each of two small layouts and one 2026 house): doors 22 same, 6 half turn, 13 quarter turn, windows 22 same, 28 half turn, 55 quarter turn, none `other`; 19 doors and 83 windows differ. The mixed states (`01`, `10`) are the 2026 house's alone (RE-343). The 2018 to 2023 ones have no stored transform, so their binding is not established: 385 doors (26 same, 94 half turn, 99 quarter turn, 166 other) and 525 windows (68 same, 80 half turn, 64 quarter turn, 313 other), where `other` is, on the two models read line by line, a Revit X that is none of the four plan directions (a building turned off the model's axes); the rest were not read line by line.

## 4. Not invisible

- The IFC rvt-rs writes for a door says nothing of its operation: the `IfcDoor` leaves `OperationType` unset (`$`) and its `IfcDoorType` writes `NOTDEFINED` (`src/ifc/step_writer.rs`, `element_tail_for` and `type_entity_for`), so the placement's +Y is the only statement in the file of which side a door or window faces (`src/ifc` reads the stored facing only for the type's flip, `export_content.rs`, and writes neither the facing nor the flips as a property).
- The world geometry is the same under any local frame that the body is re-expressed in (the box is centred on the origin), so a consumer that reads only the body sees nothing. A consumer that reads the element's placement, its axes or the orientation of its local frame sees rvt-rs's differ from Revit's on 406 of 521 bound doors and 314 of 540 windows. Whether any consumer does is not measured here, and a difference that is on most elements and fixable at 0 wrong is not a candidate for "invisible".
- The origin is a separate difference and is not claimed: rvt-rs's is the record box's centre, and in plan it is more than 0.5 ft from Revit's on 132 of 132 Core Interior doors and 6 of 6 windows (within 3 ft on all the doors; farther on all six windows), and on 260 of 269 office doors and all 269 office windows as built (39 doors and 25 windows farther than 3 ft). The rule below does not move it.

## 5. The rule

For a Door or Window that rvt-rs binds to a plain wall, the `IfcLocalPlacement` axes are Z up, X the stored X and Y the stored Y, both reversed where `m_hand_flipped` differs from `m_facing_flipped` (RE-343 Rule 1). The origin stays the record box's centre. In the reader (`partition_schema_mvp`), after the host and the flips are final (`attach_cut_loop_sizes` may rebind the host, RE-245):

- a wall whose native `SWall` record gives exactly one level line and a flip carries that line's plan direction (`m_wall_native_direction_*`), the wall RE-343's probe takes the host from;
- a door or window whose host is such a wall that is not a curtain wall, whose two flips are read, whose stored transform is upright and whose stored X runs along the wall (the absolute dot product of the two unit vectors at least 0.999; all 1,061 placed in the open models have 0.999999 or more, and the ones left out are about 1e-15, so the bound loses nothing and rejects an oblique wrong host as well as a perpendicular one), and whose X is within `INSTANCE_TURN_MIN_RADIANS` of one of the model's axes, carries Revit's placement X (`m_opening_placement_x_axis_*`): the stored X, reversed where the flips differ.

The writer (`export_content`) rotates such an element to that X (0, a quarter turn either way or a half turn) and swaps the sides of its box on a quarter turn, so the box's extents in the world do not change, and writes `RvtElementRecordGeometry.PlacementAxesSource` = `stored_axes_reversed_by_flips` (the MCP server lists it among an element's trust markers). Everything downstream already composes an element's rotation, as for a turned instance: the opening cuts (`opening_cuts_through_hosts`, `filler_type_cut`), saved meshes, family extrusions and the wall-cut openings (`door_box`).

Left out, so they keep what they write today (fail closed):

- the doors rvt-rs binds to no plain wall (160 in RE-343), 49 of which are inverted relative to the rule, uniformly by `IfcDoorType`, with the type-creator mechanism not shown;
- anything without a stored transform: Revit 2023 and older, and a record whose flips are not read;
- an instance turned off the model's axes (RE-87): it keeps its turned box, whose X is the stored one without the flip's reversal, which is Revit's wherever the flips are equal. one with unequal flips would be a half turn from Revit's, with no marker. There is none to find: the only 27 turned instances set in a plain wall in Autodesk's 2024 to 2027 samples (24 windows and 3 doors, all in Snowdon Towers Architectural, from the sample probes) all have equal flips, so 0 of 27; the held-out window that is turned has Revit's axes already. No extension;
- a door or window whose stored X does not run along its wall, whose transform is not upright, or whose host wall's record is not a straight level line (a curved wall).

## 6. Measured on the rule

Run V1: Measure [42092391898](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42092391898), head `b452f96708aa3564b5376c550b3bd32e198513f5`, `main` at `5c35e190` as base, dispatched from this branch (`-f base=main -f keep_ifc=true -f nordic=true -f holdout=true`, probe `probe_re343_door_flips`, scorer `door_placements_vs_ifc`, models Core Interior, RE1 Architecture, both office files and the held-out models, scorers `door_bodies_vs_ifc door_window_sizes_vs_ifc openings_vs_ifc opening_boxes_vs_ifc opening_hosts_vs_ifc unfilled_openings_vs_ifc window_openings_vs_box global_ids_vs_ifc type_objects_vs_ifc`). Every job succeeded.

**The rule is written for every element RE-343's Rule 1 binds, and agrees with Revit's on all.** `rule: written for N; differ from Revit 0` on every model, from the elements that carry `PlacementAxesSource` in rvt-rs's own export:

| | doors bound to a plain wall | windows bound to a plain wall | written | differ |
|---|---:|---:|---:|---:|
| Core Interior | 132 | 6 | 138 | 0 |
| RE1 Architecture | 4 | 0 | 4 | 0 |
| office as built | 193 | 269 | 462 | 0 |
| office at permit | 192 | 265 | 457 | 0 |
| open models, as run | 521 | 540 | 1,061 | 0 |
| held-out 2024 to 2027 (counts only) | 41 | 105 | 145 | 0 |

In the held-out models the one window not written already had a rotation of its own, and its axes are Revit's (all 105 windows `same`). The doors bound to none are unchanged (154 in the open models: 36 `same`, 71 half turn and 47 quarter turn from Revit's, as before), and so are the elements without a stored transform (385 doors and 525 windows of the held-out models of 2018 to 2023). The two office files are one building and R2025 models 12 to 17 were not run; counted once, without the permit file, the written count is 329 doors and 275 windows (Core Interior, RE1 and the office as built), all `same`. Per state (`00`, `01`, `10`, `11`), every cell of the first run's table is now `same` (the scorer prints it).

**The bodies and openings are where they were.** `tools/re/placed_bodies_vs_base.py` pairs the base's and the head's export of each model by Tag, meshes each door and window and the opening it fills in world coordinates, and compares their vertices both ways and their volumes (kept IFCs of this run, scored on the Mac):

| | doors' axes moved / unchanged | windows' axes moved / unchanged | bodies | openings | identity differences |
|---|---|---|---|---|---|
| Core Interior | 92 / 40 | 6 / 0 | 138 paired, 0 moved | 138 paired, 0 moved | none |
| RE1 Architecture | 3 / 2 | 0 / 0 | 5 paired, 0 moved | 4 paired, 0 moved | none |
| office as built | 156 / 113 | 156 / 113 | 538 paired, 0 moved | 463 paired, 0 moved | none |
| office at permit | 155 / 114 | 152 / 113 | 534 paired, 0 moved | 457 paired, 0 moved | none |

"Moved" is a vertex farther than 1e-6 m (0.000003 ft) from the other export's nearest, or a volume more than 1e-9 relative: none is, and the largest vertex difference prints as 0.000000 ft (the largest relative volume change is 2.5e-15). Identity differences are a different class, GlobalId, type GlobalId, opening GlobalId or host wall: none. The number of doors and windows whose axes moved is the number that differed from Revit's before the rule (92 + 6, 3, 156 + 156, 155 + 152), as it should be.

**Nothing else moved.** In the run's scorer diff (base against head) the only differences are the new `door flips:` lines; the outputs of `door_bodies_vs_ifc`, `door_window_sizes_vs_ifc`, `openings_vs_ifc`, `opening_boxes_vs_ifc`, `opening_hosts_vs_ifc`, `unfilled_openings_vs_ifc`, `window_openings_vs_box`, `global_ids_vs_ifc` and `type_objects_vs_ifc` are the same on every open model and on the held-out models' kept counts. `ifc.sha256` moves on the four open models, as it must. V1 did not run the other scorers, the witness, the MCP answers or the semantic diff, nor time the added pass over the stored transforms; V2 below does.

### The final head (run V2 and the box-body run)

Run V2: Measure [42106711251](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42106711251), head `c1a2a7399d4ee71bb9a10e3cdc4e6927884ca45c` (V1's rule with the along-wall bound at 0.999 and `main` at `e9f4c592` merged), base `main` at `e9f4c592`, unfiltered (`-f base=main -f nordic=true -f r2025=true -f holdout=true -f samples=true -f sample_years="2024 2027" -f keep_ifc=true`, probe `probe_re343_door_flips`, scorer `door_placements_vs_ifc`; every default scorer on every model). Every job succeeded, `same-project gates` included (9 samples).

- **The default scorer `tools/re/door_axes_vs_ifc.py` (no probe), per model, base against head.** Doors, then windows, as same / half turn / quarter turn from Revit's, and how many were placed by the rule (`PlacementAxesSource`) and differ from Revit's:

  | model | base: doors | base: windows | head: doors | head: windows | placed by the rule, differ |
  |---|---|---|---|---|---:|
  | Core Interior | 40 / 39 / 53 of 132 | 0 / 0 / 6 of 6 | 132 same of 132 | 6 same of 6 | 138, 0 |
  | RE1 Architecture | 1 / 1 / 3 of 5 | none | 4 same, 1 half turn of 5 | none | 4, 0 |
  | office as built | 55 / 110 / 104 of 269 | 113 / 103 / 53 of 269 | 211 same, 34 half, 24 quarter of 269 | 269 same of 269 | 462, 0 |
  | office at permit | 55 / 110 / 104 of 269 | 113 / 99 / 53 of 265 | 210 same, 36 half, 23 quarter of 269 | 265 same of 265 | 457, 0 |
  | R2025 models (24 files; the doors and windows are in models 12 to 17) | 0 / 0 / 6 of 6 | 6 / 0 / 12 of 18 | 6 same of 6 | 18 same of 18 | 24, 0 |
  | held-out, counts only (18 models) | 48 same, 100 half, 112 quarter, 166 other of 426 | 90 same, 108 half, 119 quarter, 313 other of 630 | 67 same, 94 half, 99 quarter, 166 other | 173 same, 80 half, 64 quarter, 313 other | 145, 0 |

  The 58 and 59 doors that are not `same` on the two office files are doors bound to none (76 and 77 of them, 18 of which are `same`), unchanged. The 2018 to 2023 held-out models and the spent 2018 pair (68 doors and 70 windows, none placed by the rule) have no stored transform and keep their default axes, which differ from Revit's as before. The other five open models hold no door or window both exports have (the two terrain office files and the three RE1 services models). The scorer exits 0 on all 52 models that have a Revit export. It is in `main`'s default list from this change, so every unfiltered Measure repeats the claim and the base-against-head diff catches a regression.
- **Nothing else moved.** The run's scorer diff (base against head) holds only: this scorer's output on the ten open models, the held-out models' `door axes:` lines, the `doors and windows` lines of the gate on the six 2025 to 2027 samples that hold any, and the two witness observations. Every other scorer output (the whole default suite: doors, openings, sizes, GlobalIds, types, property sets, quantities, saved and box bodies, the MCP answers) is identical base against head on every model.
- **The semantic diff saw dumps** (the MCP server's answers as the deterministic dump) on all 35 models with one, the open and R2025 models among them, and on Core Interior, RE1, the office files and R2025 model 12 the findings are exactly `RvtElementRecordGeometry.PlacementAxesSource` and `trust_markers.PlacementAxesSource` absent to `stored_axes_reversed_by_flips` and `rotation_degrees` absent to 90, 180 (or -90) on the doors and windows the dump holds; no other element changes.
- **Core Interior's step counts and witness.** In both witness observations `IFCDIRECTION` moves 1037 to 1135 (98: a direction of its own for each of the 92 doors and 6 windows not on +X) and `IFCPROPERTYSINGLEVALUE` 27824 to 27962 (138: `PlacementAxesSource` on 132 doors and 6 windows); nothing else. Both `rvt-rs.json` observations are refreshed from this run (`witness-<artifact>.json`, byte for byte), and `witness-verdict.py` replayed with CI's arguments is PASS for both artifacts (the committed verdict files unchanged, the three witnesses replay `match`). The fixtures' notes carry a dated line.
- **Time.** The export's wall time in `rvt-ifc.log`, base against head, one run each on different runners (runners differ by up to 1.7 times, so a cost under about 2 percent cannot be resolved): open models (10 files) head over base 0.944 to 1.061, mean 1.007, totals 60,141 and 60,282 ms; R2025 models (24) mean 1.014; samples 2024 (10 files) mean 0.978, 2025 0.975, 2026 0.991, 2027 0.990; the three largest files: the office as built 25,692 and 26,034 ms, Snowdon Towers Architectural 25,789 and 25,555 ms, the office at permit 23,035 and 22,932 ms. The added read of the stored transforms is not resolvable from this. Memory was not measured.
- **Box-bodied doors do not move.** Every door and window of the open models goes through a saved mesh by default, so V1 exercised the swapped box only through its openings. The box-body run, Measure [42106731054](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42106731054) (head `c1a2a739`, base `main` at `e9f4c592`, `-f ifc_flags=--no-saved-meshes -f keep_ifc=true -f nordic=true -f models="core-interior nordic-office-*" -f scorers=none`; it judges no gate), exports the same models with the record boxes as bodies: every door and window is an `IfcExtrudedAreaSolid` rectangle (138 of 138, 538 of 538). `placed_bodies_vs_base.py` on its kept IFCs:

  | | doors' axes moved / unchanged | windows' axes moved / unchanged | bodies | openings | identity differences |
  |---|---|---|---|---|---|
  | Core Interior | 92 / 40 | 6 / 0 | 138 paired, 0 moved, largest relative volume change 4.6e-16 | 138 paired, 0 moved | none |
  | office as built | 156 / 113 | 156 / 113 | 538 paired, 0 moved (4.3e-16) | 463 paired, 0 moved | none |
  | office at permit | 155 / 114 | 152 / 113 | 534 paired, 0 moved (4.0e-16) | 457 paired, 0 moved | none |

  The largest vertex difference prints 0.000000 ft, and no pair meshed in neither export. Of the elements whose axes moved, the rectangle's two sides are kept on the half turns and swapped on the quarter turns, by count: Core Interior 39 kept and 59 swapped (the 39 half turns and the 53 doors and 6 windows on a quarter turn), the office as built 179 kept and 133 swapped (76 + 103 half turns, 80 + 53 quarter turns).

## 7. The same-project check on Revit 2025, 2026 and 2027

The rule writes new axes on Revit 2025, 2026 and 2027, where almost no Revit export exists (RE-343 section 6: one 2026 house and layouts whose doors and windows are `00` or `11`; one 2025 model, RE1 Architecture, and the R2025 models). So each 2025, 2026 and 2027 door and window the rule places is held to the 2024 save of the same Autodesk sample project, per element, by `tools/re/bodies_across_saves.py` (a `doors and windows` group): class, GlobalId, storey, `BodySource`, placement within 0.001 ft and its axes within 1e-6, volume and vertices, and the element must be placed by the rule in the 2024 save too. It is the right home, not `readers_across_saves.py` (#1454): it already compares placements and world vertices by Tag, which is what a placement is, and its failure turns Measure red.

Run V2 above holds it: `same-project gates` succeeded on the 9 samples (3 projects, 3 saves each), and every save reads `bodies across saves: PASS`. Run G (Measure [42092410697](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42092410697), head `b452f967`, 2026 and 2027 only) gave the same 2026 and 2027 counts.

| project and save | doors and windows compared with the 2024 save | not on the default axes | in the 2024 save placed by the rule and left on the default axes here, not scored |
|---|---|---:|---:|
| `rac_advanced` 2025 | 107 (83 doors, 24 windows) | 66 | 0 |
| `rac_advanced` 2026 | 79 (61, 18) | 44 | 28 (22, 6) |
| `rac_advanced` 2027 | 65 (47, 18) | 31 | 42 (36, 6) |
| `rac_basic` 2025, 2026 and 2027 | 17 each (10, 7) | 14 each | 0 |
| `rst_basic` 2025, 2026 and 2027 | 0 | 0 | 0 |

**What this exercises, plainly.** Two buildings, each saved by three releases: 124, 96 and 82 doors and windows compared (2025, 2026, 2027), 80, 58 and 45 of them on axes other than the default, the same elements saved by three releases. `rst_basic` holds no door or window the rule places, so it exercises nothing. The 2024 save's own axes are measured against Revit's export only on the models of section 6, so this check holds 2025 to 2027 to a save, not to Revit. The 28 and 42 elements of `rac_advanced` that the 2024 save places by the rule and these saves leave on the default axes are counted, not scored, and gated from above. Their cause is determined: the pass that takes a door's host from its native record (RE-245) runs on `CUT_LOOP_REVIT_VERSIONS` (`fields.rs`), 2024 and 2025 only, so on 2026 and 2027 a nearest wall that is not the door's own fails the along-the-wall test (#1454 widens that list to 2026 and 2027, which should place those 28); and on 2027 the other 14 are set in 8 host walls with no native straight-line row (#1475; RE-360: those walls' records hold a second `GLine`, owned by `WallRunTimeData.m_prevLocLine`, which the one-curve rule counts). The floors are per class, project and year (`OPENINGS_LEAST`: `rac_advanced` 83 doors and 24 windows in 2025, 61 and 18 in 2026, 47 and 18 in 2027; `rac_basic` 10 and 7 in each) and the ceilings `OPENINGS_LEFT_OUT_MOST` (`rac_advanced` 0, 28 and 42; `rac_basic` 0, 0 and 0), so a count that drops moves its floor up and one that shrinks moves its ceiling down in that change. `rme_advanced` and `rme_basic` hold 107 placed doors per file (59 with unequal flips) and their 2026 and 2027 saves are held to nothing; they join this group, with floors, in whichever of #1454 and #1456 lands second, because their pipes do not pass the gate before #1454. The 2025 floors and ceilings are set from run V2's counts after that run, which changes a gate file (`bodies_across_saves.py`), so a samples-only run on the final head writes the verdict.

## 8. Not claimed, not measured

- The origin (#1465), what Revit's origin is, and anything keyed to it.
- Snowdon Towers (local only; 44 mixed-state doors and 2 windows) against Revit's export: not scored. Its 2024 sample is in the runs, and the 27 turned plain-walled instances of section 5 are read from its probe.
- The 160 doors bound to none; a curved wall; a turned instance with unequal flips (none in the samples).
- Revit 2026 and 2027 against a Revit export beyond RE-343's held-out counts: the 2026 house (13 doors and 16 windows in its states) and the small layouts are `same` on all of them (section 6), which exercises the rule on one 2026 house in a mixed state and no 2027 model in one.
- What any consumer does with the axes; the memory of the added read of the stored transforms (its time is below what a run resolves, section 6).

## 9. Reproduce

The models are those of RE-343 section 7 (Core Interior, RE1 Architecture, the BIM4LCA office files, the held-out list).

```bash
gh workflow run measure.yml --ref <branch holding the scorers> -f ref=<sha> -f base=main -f nordic=true -f r2025=true \
  -f holdout=true -f samples=true -f sample_years="2024 2027" -f keep_ifc=true \
  -f probe=probe_re343_door_flips -f probe_scorer=door_placements_vs_ifc    # V2, with every default scorer
gh workflow run measure.yml --ref <branch holding the scorers> -f ref=<sha> -f base=main -f ifc_flags=--no-saved-meshes \
  -f keep_ifc=true -f nordic=true -f models="core-interior nordic-office-*" -f scorers=none   # the box-body run
gh workflow run measure.yml --ref <branch holding the scorers> -f ref=<sha> -f base=none \
  -f samples=true -f sample_years="2024 2027" -f scorers=bodies_across_saves -f models="re1-architecture"   # G, the gate alone
```

`<sha>` is `c1a2a7399d4ee71bb9a10e3cdc4e6927884ca45c` (V2 and the box-body run), `b452f96708aa3564b5376c550b3bd32e198513f5` (V1 and G; V1 is the first command with `-f nordic=true -f holdout=true -f keep_ifc=true`, the probe above, `-f models="core-interior re1-architecture nordic-office-as-built nordic-office-permit holdout-*"` and the nine scorers of section 6) and `eb6de281f64761045ef01fdae6026c0a0b522b0b` (V0, with `-f base=none -f scorers=none`). With the kept IFCs of a run downloaded, base and head of each model:

```bash
python3 tools/re/placed_bodies_vs_base.py base/<model>/model.ifc head/<model>/model.ifc
```

The probe's scorer reads rvt-rs's IFC as `model.ifc` beside the probe output, which only `keep_ifc=true` leaves in place: `python3 tools/re/door_placements_vs_ifc.py probe.txt revit.ifc --ours rvt-rs.ifc`. The default scorer needs only the two exports: `python3 tools/re/door_axes_vs_ifc.py rvt-rs.ifc revit.ifc`.
