# RE-347 — The local axes rvt-rs writes for a door or window, against Revit's

**Date:** 2026-11-05
**Issues:** #1447; follows RE-343 (the hand and facing flips, #1357), RE-87 (instance transforms) and RE-84 and RE-215 (openings hang off the filler's placement)
**Probe:** `probes/examples/probe_re343_door_flips.rs`
**Scorer:** `tools/re/door_placements_vs_ifc.py`
**Status:** positive measurement, no rule written. rvt-rs writes the default axes (X along the model's +X, Z up) on every door and window it exports, on every model measured. Revit's axes are the same as rvt-rs's on 115 of the 521 doors and 226 of the 540 windows that rvt-rs binds to a plain wall in the open models (counted as run, the permit office file included), exactly where Revit's X happens to be the model's +X. The difference is not invisible: rvt-rs writes no `OperationType`, so the placement's axes are all its IFC says of which way a door faces. A rule that gives Revit's axes on every bound element exists (RE-343's Rule 1) and would move most placements, so it is proposed here and not written (section 5).

## 0. The question

RE-343 measured Revit's `IfcLocalPlacement` axes against the axes the instance stores, and left open what rvt-rs writes for the same elements (it takes the stored X only for a box turned off the model's axes, `turned_box`, RE-87). #1447 asks for that measurement, tabulated by the two flips and by whether the element is bound to a plain wall, and then either a rule with 0 wrong on walled elements or a closure as invisible.

## 1. What was measured

`tools/re/door_placements_vs_ifc.py` pairs, by Tag, each door and window of the probe with the `IfcDoor` or `IfcWindow` of Revit's export and of rvt-rs's own export. It composes each element's `IfcLocalPlacement` chain, moves the X, Y and Z axes into Revit's internal frame through that file's own `IfcSite` placement (rotation only; rvt-rs's site is the identity on every model here), and gives one verdict per element:

- `same`: the three axes the same;
- `half turn`: X and Y reversed, Z the same;
- `quarter turn`: X and Y a right angle from Revit's, Z the same;
- `other`: anything else (a tilt, or a building turned off the model's axes, where Revit's X is neither of the four plan directions).

The binding is RE-343's Rule 1 subset: `bound to a plain wall` where the probe read the stored transform and the wall rvt-rs binds to has a native record with a curve; `bound to none` where it read the transform and no such wall; `no stored transform` where it read none (Revit 2023 and older). The state is `m_flippedX m_flippedY` of the native record.

Run: Measure [42066757105](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42066757105), measured head `eb6de281f64761045ef01fdae6026c0a0b522b0b` (the code of `main` at `d6f69d97` and the scorer; no `src/` change), dispatched from `main`: `-f base=none -f scorers=none -f keep_ifc=true -f nordic=true -f holdout=true`, the probe and scorer above, `-f models="core-interior re1-architecture nordic-office-as-built nordic-office-permit holdout-*"`. Every job that ran succeeded. The held-out lines are counts the scrub kept (`door flips:` in `probe_score*.txt`).

**The denominators are RE-343's.** The elements bound to a plain wall are Core Interior 132 doors and 6 windows, RE1 Architecture 4 doors, the office as built 193 doors and 269 windows, the office at permit 192 doors and 265 windows, and the doors bound to none are RE1's 1, the office's 76 and 77: the figures of RE-343's Rule 1 table, so the probe's host read is the same on this head. Every door and window of every model, the held-out ones included, is in both exports (0 missing from either), and the `Ifc` class agrees on all of them.

## 2. rvt-rs writes the default axes everywhere

`rvt-rs axes are the default axes` (X along +X, Z up) equals the number checked on every line of the open models: 521 of 521 bound doors, 154 of 154 doors bound to none, 540 of 540 bound windows. In the held-out models of 2024 to 2027, 41 of 41 bound doors and 104 of 105 bound windows; the one window that is not default is not looked at further (`turned_box`, RE-87, is the only code that writes a rotation, so it is the likely case, not checked), and the 385 doors and 525 windows of the held-out models of 2018 to 2023 are default on all of them. rvt-rs's Z axis is up on every element of every model.

## 3. Against Revit's, bound to a plain wall

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

## 5. The rule, not written

For a Door or Window that rvt-rs binds to a plain wall (RE-343's Rule 1 subset: the stored transform read, the host wall's record with a curve), write the `IfcLocalPlacement` axes as Z up, X the stored X and Y the stored Y, both reversed where `m_hand_flipped` differs from `m_facing_flipped`. RE-343 measured this at 0 wrong on all 653 doors and 626 windows it covers, and section 3 reproduces it on 521 and 540. The inputs are decoded on all of them (`decoded record carries both flips and the opening facing` equals `checked` on every bound line of the open models, 2024 and 2025; and on every bound element of the held-out 2024 to 2027 models). The origin stays the record box's centre.

Left out, so they keep what they write today (fail closed):

- the doors rvt-rs binds to no plain wall (160 in RE-343), 49 of which are inverted relative to the rule, uniformly by `IfcDoorType`, with the type-creator mechanism not shown;
- anything without a stored transform: Revit 2023 and older, and a record whose flips are not read;
- an element that already takes the stored X for a turned box (RE-87), until the two are shown to agree.

What the rule would move, the part this PR does not do:

- the `IfcLocalPlacement` of about 78% of the bound doors and 58% of the bound windows of the open models (406 of 521 and 314 of 540), and of 19 of 41 and 83 of 105 in the held-out models of 2024 to 2027;
- the local representation of each: a quarter turn swaps the box's width and depth in its profile (the world box stays), and a symbol's cut loop or a mapped item (RE-215) is expressed in the new frame;
- the opening elements, which hang off the filler's placement (RE-84, RE-215): their relative placement is recomputed so their world position stays;
- the type GlobalIds must not change: the writer's flip against the host wall (`door_symbol_flipped`, B72, RE-167) reads the element's box centre, the host's direction and the stored facing, and not the axes the door's own placement is given, so the rule does not reach it by construction; `tests/door_type_global_ids.rs` and `tools/re/global_ids_vs_ifc.py` check it, and the unfiltered run is where it is measured;
- the scorers that compose a door's or window's placement: `door_bodies_vs_ifc.py`, `symbol_cuts_vs_ifc.py` and `turned_instances_vs_ifc.py` read it, and `opening_frames_vs_ifc.py` and `window_openings_vs_box.py` read the frame of its openings. Each is to be read before the change, not after.

On Revit 2026 and 2027 the rule writes new axes where almost no Revit export exists (one 2026 house, 7 doors and 6 windows in a mixed state, and layouts whose elements are `00` or `11`). The same-project gate applies: each door's and window's axes held to the earlier save of the same project per element, with the number of elements compared stated in the verdict.

This is a rule that moves most placements in the IFC of every model with doors, so it needs an unfiltered Measure (`-f base=main -f nordic=true -f r2025=true -f holdout=true -f samples=true`) expecting only door and window placements, their representations and their openings to move; `ifc.sha256` will move on every model that holds a door or window.

## 6. Not measured

- Snowdon Towers (local only; 44 mixed-state doors and 2 windows): not scored. Its rvt-rs IFC needs a `samples=true` run, and the answer does not depend on it.
- R2025 models 12 to 17: not run (one `11` door and `00` or `11` windows each, so no mixed state).
- Revit 2026 and 2027 against an export beyond the held-out counts above.
- What any consumer does with the axes; the body in world space after a change (the body scorers measure it); the origin (section 4).
- The hinge side and the swing of a door: only the placement is compared (RE-343 section 3, operation side).

## 7. Reproduce

The models are those of RE-343 section 7 (Core Interior, RE1 Architecture, the BIM4LCA office files, the held-out list).

```bash
gh workflow run measure.yml --ref main -f ref=<sha> -f base=none -f scorers=none -f keep_ifc=true \
  -f nordic=true -f holdout=true -f probe=probe_re343_door_flips -f probe_scorer=door_placements_vs_ifc \
  -f models="core-interior re1-architecture nordic-office-as-built nordic-office-permit holdout-*"
```

`<sha>` is `eb6de281f64761045ef01fdae6026c0a0b522b0b` or any commit of `main` that holds the scorer. The scorer reads rvt-rs's IFC as `model.ifc` beside the probe output, which only `keep_ifc=true` leaves in place. Offline, with a probe output and both exports: `python3 tools/re/door_placements_vs_ifc.py probe.txt revit.ifc --ours rvt-rs.ifc`.
