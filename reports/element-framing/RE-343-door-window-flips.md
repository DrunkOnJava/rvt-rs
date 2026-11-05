# RE-343 — A door's or window's hand flip and facing flip

**Date:** 2026-11-05
**Issues:** #1357; follows RE-87 (instance transforms), RE-167 (the type GlobalId's flip, B72) and RE-216 (the current copy of an instance)
**Probe:** `probes/examples/probe_re343_door_flips.rs` (runs A and C below used it under its earlier name, `probe_re342_door_flips`, at the commits named there)
**Scorer:** `tools/re/door_flips_vs_ifc.py`
**Status:** positive, with one limit stated in section 6. The flips are two booleans of the instance's native `FamilyInstance` record, `m_flippedX` and `m_flippedY`. No door or window is stored as a reflection, so nothing is in the transform. Revit's own placement is exactly the stored axes, reversed where the two booleans differ, on every door and window whose host wall's record is read (653 of 813 doors and 626 of 626 windows over the open models and Snowdon Towers); the 160 doors without such a record are counted apart and not claimed.

## 0. The two questions

**If the instance transform is stored mirrored, can the flips be read from it?** No instance is stored mirrored. The first orthonormal 3 x 3 in the element data has determinant +1 on all 1,439 doors and windows scored against Revit's export (run A and Snowdon Towers), and on all 1,820 doors and windows of the Autodesk samples of 2024 to 2027 (run C); none has -1. Revit's own placements have determinant 1 on all 1,439. The reader of RE-87 therefore skips nothing, and the flips are not in the transform. They are in the native record (section 2).

**Does `m_opening_facing` already include the facing flip?** It includes one of the two flips, and not the one the name suggests. `m_opening_facing_x/y` is the stored transform's Y axis (RE-87). That axis is a half turn from the unflipped one exactly where `m_flippedX` is set, and `m_flippedY` does not change it. The direction the door faces (the +Y of Revit's own placement, which is where IFC4 has a door open) is the stored Y axis reversed where `m_flippedX` and `m_flippedY` differ. So `m_opening_facing` is the facing direction on the doors whose two booleans are equal, and the opposite of it on the others: 379 of the 813 doors and 286 of the 626 windows in this report's models are in the second case (Core Interior 39 of 132 doors, Snowdon Towers 45 of 132).

## 1. What Revit's exporter does with the flips

(revit-ifc, CLEANROOM.md source 5; read for behaviour, not copied)

- `DoorWindowInfo` takes its `FlippedX` from `FamilyInstance.HandFlipped` and its `FlippedY` from `FacingFlipped`. A door's `OperationType` is its type's "Operation" string, reversed when `FlippedX` differs from `FlippedY`; where the type names none, `CalculateDoorOperationStyle` guesses it from the symbol's plan arcs after `ExporterIFCUtils.GetTransformForDoorOrWindow(.., FlippedX, FlippedY)` (a RevitAPI name; its body is not read). A left or right swing is assigned only from an arc on the +Y side of that frame.
- The type object is split by `FlippedSymbol`, whether the door's geometry centre and its Y axis lie on the same side of the host wall's axis (RE-167). It is neither flip alone.
- `FamilyInstanceExporter` and `PlacementSetter` build the instance's `IfcLocalPlacement` from the instance transform's origin, `BasisZ` and `BasisX` (`ExporterUtil.CreateLocalPlacement`), so the placement's Y is derived, `Z x X`.
- buildingSMART: `SINGLE_SWING_LEFT` has its hinges on the left seen along the placement's +Y, and the leaf opens toward +Y.
- The public Revit API guidance (Scott Conover, AU 2009, on the Building Coder, `0290_abg07_transform`): "If a family instance is flipped in X, its FlippedHand property is true. If it is flipped in Y, its FlippedFacing property is true"; the transform's Y direction is the nominal facing direction, and where exactly one of them is set the transform is left-handed and the direction must be reversed. `FacingOrientation` is that calculation.

## 2. The carrier

The instance's native `FamilyInstance` record has root booleans `m_flippedX` and `m_flippedY`, beside `m_flippedFromToRoom`, `m_workPlaneFlipped` and `m_insertOrientation`. The last two are false and 0 on every door and window scored in run A and on Snowdon Towers.

- **Read on every one.** All 58 Autodesk sample files of 2015 to 2027, 39 of them with doors or windows (3,729 in all; run C, `d8b2d2b5`): no door or window lacks `m_flippedX` or `m_flippedY`. The same project saved by every release has the same state counts on all of them (`rac_advanced` and `rac_basic` 13 releases, `rme_advanced` and `rme_basic` 6, one state table each).
- **Native read of 2015 to 2022 uses `NATIVE_RELEASES`.** The default of `native_document` is 2023 to 2027 and the probe names the full range (RE-280).
- **Decoded Door and Window records exist from 2024.** The fields of section 5 are carried by 1,820 of the 1,820 doors and windows of the 2024 to 2027 samples and by none of the 1,909 of 2015 to 2023 (run F, `7d346f1f`): those releases' doors and windows are not decoded as Door and Window records, so the booleans are read and have nowhere to go.
- **Not the other booleans.** `m_flippedFromToRoom` is true on 95 of Core Interior's 138 doors and windows and predicts nothing below.

State is `m_flippedX m_flippedY`, written `00`, `01`, `10`, `11`. The doors and windows of each model with Revit's export:

| model | release | doors: 00 / 01 / 10 / 11 | windows: 00 / 01 / 10 / 11 |
|---|---:|---|---|
| Core Interior | 2024 | 27 / 24 / 15 / 66 | 0 / 0 / 0 / 6 |
| RE1 Architecture | 2025 | 2 / 1 / 1 / 1 | none |
| BIM4LCA office as built | 2024 | 51 / 80 / 67 / 71 | 26 / 142 / 2 / 99 |
| BIM4LCA office at permit | 2024 | 52 / 79 / 67 / 71 | 26 / 140 / 0 / 99 |
| R2025 models 12 to 17 (each) | 2025 | 0 / 0 / 0 / 1 | 2 / 0 / 0 / 1 |
| Snowdon Towers Architectural (local only) | 2024 | 51 / 31 / 14 / 36 | 56 / 0 / 2 / 10 |

Every door state is populated in Core Interior, RE1, the office and Snowdon Towers. The R2025 models, the only ones made to isolate doors and windows, hold one door each and are no test of a state: they show only `11` and, for windows, `00` and `11`.

## 3. The rules, measured against Revit's placements

For every door and window, Revit's `IfcLocalPlacement` is moved into Revit's internal frame through the export's site placement (`tools/re/ifc_common.py`'s frame, rotation only), and its X, Y and Z axes are compared with the stored transform's. The host wall is the one rvt-rs's native `SWall` record (RE-222) gives for the wall Revit's export voids; "left" is the left of that wall's curve, and 184 of the doors and 488 of the windows below stand in walls whose `m_isFlipped` is set.

**Rule 1: Revit's placement axes are the stored axes, reversed where `m_flippedX` differs from `m_flippedY`.** On every state, the three axes are the same or all (X and Y) reversed with Z the same:

| model (run, head) | doors with a host wall record: checked / wrong | windows: checked / wrong | doors without one: checked / wrong |
|---|---:|---:|---:|
| Core Interior (A, `f65fc702`) | 132 / 0 | 6 / 0 | 0 |
| RE1 Architecture (A) | 4 / 0 | none | 1 / 1 |
| office as built (A) | 193 / 0 | 269 / 0 | 76 / 21 |
| office at permit (A) | 192 / 0 | 265 / 0 | 77 / 23 |
| R2025 models 12 to 17 (A, each) | 1 / 0 | 3 / 0 | 0 |
| Snowdon Towers (C probe, scorer at `d8b2d2b5`, local) | 126 / 0 | 68 / 0 | 6 / 4 |
| total | 653 / 0 | 626 / 0 | 160 / 49 |

**Rule 2: against the host wall, the stored axes are a half turn from the wall's direction and left exactly where `m_flippedX` is set, and Revit's placement exactly where `m_flippedY` is set.** 0 wrong on the same 653 doors and 626 windows. In words: with `m_flippedY` false, Revit's placement faces left of its wall's curve and its X runs along it; stored axes do so with `m_flippedX` false. This is what the first table's states do to the axes: `01` turns the placement and not the stored axes, `10` turns the stored axes and not the placement, `11` turns both.

**What is left out.** The 160 doors with no host wall record (no wall record of the host is read for them; why is not measured) follow Rule 1 on 111 and not on 49, so nothing is claimed for them. Every window has a wall record. The fields in section 5 are written for them too, as the record holds them, but no rule links those to Revit's placement.

**Held-out models** (run E at `7d346f1f`, counts only; Revit's exports of 18 models of 2018 to 2027). 426 doors and 630 windows read their booleans. The 2024 to 2027 models (41 doors, 105 windows) have a transform and a host wall: 0 wrong on Rule 1 and on Rule 2. Only one of them has a mixed state, a 2026 house (13 doors: 2 / 2 / 5 / 4, 16 windows: 9 / 0 / 6 / 1 in the order of the table above), so 7 doors and 6 windows test the facing rule; all the other 2024 to 2027 held-out doors and windows are `00` except one window, `11`, and prove the rule no more than the R2025 models do. The 2018 to 2023 models (385 doors and 525 windows, every state populated on the larger ones) have no transform, so Rules 1 and 2 cannot be checked on them; the operation side below holds on all 5 symbols with doors in both relations across the held-out models: the 2018 model 1 of 1, the 2020 one 1 of 1, the 2021 one 2 of 2 and the 2026 house 1 of 1.

**Operation side.** Within one symbol, a left or right `OperationType` differs between two doors exactly when the flags differ (`m_flippedX xor m_flippedY`), as revit-ifc's `FlippedX ^ FlippedY` says:

| model | pairs of doors of one symbol (against the symbol's first), `m_flippedX xor m_flippedY` agrees | `m_flippedX` alone | `m_flippedY` alone | stored Y's side of the wall | symbols with doors in both relations, one side in each and the sides differ |
|---|---:|---:|---:|---:|---:|
| office as built | 161 of 162 | 102 of 162 | 86 of 162 | 89 of 159 | 13 of 14 |
| office at permit | 161 of 161 | 101 of 161 | 86 of 161 | 88 of 159 | 13 of 13 |
| RE1 Architecture | 1 of 1 | 0 of 1 | 1 of 1 | not scored | 1 of 1 |
| Snowdon Towers | 108 of 108 | 78 of 108 | 70 of 108 | 87 of 105 | 9 of 9 |

The one exception is a symbol of the as-built office with two doors, `01` and `00`, that Revit's export gives the same `SINGLE_SWING_RIGHT`; it is not explained. Core Interior's export (Revit 24.0.20) gives all 132 doors and 10 door types `NOTDEFINED`, so it says nothing of operation; RE1's two Single-Flush doors (`00`: `SINGLE_SWING_RIGHT`, `01`: `SINGLE_SWING_LEFT`) are one pair. A flag alone, or the stored Y's side of the wall, agrees with the operation's parity on little more than half the pairs, which is why the hinge side cannot be had from `m_opening_facing` alone.

## 4. Which boolean is the hand and which the facing

Revit's API names are `HandFlipped` and `FacingFlipped`; neither is in the export on its own. The assignment `m_flippedX` = hand, `m_flippedY` = facing rests on three things, and no more:

1. The public API guidance quoted in section 1 (flipped in X is the hand, in Y the facing), and revit-ifc's own `FlippedX`/`FlippedY` naming.
2. Applying that guidance's rule (reverse the transform's Y where exactly one flip is set) to the stored Y gives a direction equal to Revit's placement +Y on all 653 doors and 626 windows of Rule 1, in every state the models hold; that direction is left of the wall's curve exactly where `m_flippedY` is false. So `m_flippedY` alone decides which side of the wall the door faces, which is what `FacingFlipped` records.
3. The operation side changes with the xor of the two (the table above), as `FlippedX ^ FlippedY` does in revit-ifc.

Not shown: any export field, or any Revit file written both ways, that names the hand on its own. Which of the two a consumer should call the hand is the API's `m_flippedX`; if that reading is wrong the facing rule of section 0 is unchanged, since it uses only the xor and `m_flippedY`'s side.

## 5. What is exposed

Two fields on the decoded Door and Window records, present only where the native record holds both booleans (`attach_opening_flips`, `partition_schema_mvp/openings.rs`):

- `m_hand_flipped`: `m_flippedX`
- `m_facing_flipped`: `m_flippedY`

They are booleans and carry no claim the rules above do not measure. The IFC rvt-rs writes does not read them and is not changed. rvt-rs's own placement of a mixed-state door is a half turn from Revit's (the "reversed" doors of RE-87, which the flags now explain: 379 of the 813 doors here); changing that would change the export and is not done here.

Measured at `7d346f1f`, where a pass of its own read the booleans (runs D, E and F, section 7): the decoded fields equal the native booleans on every element that carries them.

- Core Interior 132 of 132 doors and 6 of 6 windows carry them; RE1 Architecture 5 of 5; R2025 model 12 1 of 1 and 3 of 3; the office as built 268 of 269 doors and 269 of 269 windows, at permit 268 of 269 and 265 of 265. The one door of each office file that carries none (it has native flags, `01`) is exported as an `IfcDoor` from an element that is not in the decoded Door list; it carries `m_opening_facing` no more than the fields.
- The 58 samples: 1,820 of 1,820 doors and windows of 2024 to 2027 carry both fields and `m_opening_facing`, equal to the native booleans; none of 2015 to 2023.
- The held-out models, counts only: every door and window of the 2024 to 2027 ones (41 doors, 105 windows) carries them, equal; none of the 2018 to 2023 ones.

**The head `e2f8accc` reads them in an existing pass.** `instance_cuts_and_symbols` (RE-215, RE-245) already reads each door's and window's `FamilyInstance` record on 2024 and 2025 for its symbol and host; it now also takes `m_flippedX` and `m_flippedY` from that record, for every door and window (those that are an opening alone too, as before), and `attach_opening_flips` makes no pass of its own there. On 2026 and 2027, which no pass reads, the dedicated pass (`graph_classes` limited to `FamilyInstance`, the doors' and windows' ids) remains: one `extract_graphs` over the file. Its cost against `main`, the unfiltered comparison and the fields on this head are not yet measured: runs U, T, D', E' and F' (section 7).

## 6. Not claimed, not measured

- The hand/facing naming beyond section 4; the hinge side (the family's native hinge side is not read: only its parity with the flags is shown); the swing side of a door whose type names an Operation or whose export gives `NOTDEFINED`.
- The 160 doors without a host wall record; any wall that is not straight; a wall record that is not read.
- Revit 2026 and 2027 against an export: the open models are 2024 and 2025. Revit 2026 has the held-out house (7 doors and 6 windows in a mixed state, 0 wrong); Revit 2027 only held-out models whose doors and windows are `00` or `11` (7 doors, 22 windows), which agree and do not test the rule. The first held-out run (B) produced no count lines: the scrub of the workflow's own ref, `main`, did not know them.
- Snowdon Towers is local only; its figures are the run C probe output scored on the Mac against Revit's IFC4 export (sha256 `ecfcb04e3e818090cf818873fe76fba8b447742bbf4d19dad6d590f09cbb985a`), counts only.
- `InstanceInfo.m_Trf.m_3x3` has determinant 1 on the first 40 doors and windows by ElementId of Core Interior and of the office as built, whose objects the probe printed (all four states in each); the other doors' were not printed in run A.

## 7. Reproduce

| file | sha256 |
|---|---|
| `2024_Core_Interior.rvt` | `c805df445d613b408e37337765572021265e3f5dfdc7d1fa53b22ba1600b8014` |
| `2024_Core_Interior_slim.ifc` | `bfdf36ffb0bb768f3409d818403990e64d4c262c6780603be87f8077387ad86d` |
| `RE1-Architecture.rvt` | `4a78bdd68e7f4dd7806060db07c222da9a810e20b3f303c17a443294b91c8ce4` |
| `RE1-Architecture.ifc` | `a9b5d36677aa6a8bb91b77d8bc354ed9028a7ca3e9e2491a47ba14cfd8e26200` |
| `R2025_Models.zip` (models 10 to 18 read) | `8673d803fb0b9b6aa20d08448eeed16eb34222566bfaf7a2dc761c064dc30d6a` |
| the BIM4LCA office files | `research/bim4lca-measure.tsv` |
| Snowdon Towers Sample Architectural.rvt (local only) | `33271010919acb2a0d0d040851393a511d86ea7fd9a1f4c054797ae03e5e44a1` |

Runs, with the commit of the code each built and the workflow's own ref (`main` at `f8612274`):

- **A:** Measure [42014420180](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42014420180), `f65fc702c03798f838619ee20cb060ed71b1899b`, probe `probe_re342_door_flips`: Core Interior, Einhoven, RE1 Architecture, the four office files and R2025 models 10 to 18.
- **C:** Measure [42015182768](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42015182768), `d8b2d2b54c54335a956a64c49f4cd3a02470b874`, the same probe on the 58 Autodesk sample files (no export: states and determinants only).
- **B:** Measure 42014433323, `f65fc702`, held-out models: no count lines (section 6), not used.
- **D, E, F, read:** the code before the fold, `7d346f1f0b3eff6a9a1d4fb5daa75d70c53f19f4`, dispatched with `--ref research/1357-door-window-flips` so the scorer and the held-out scrub are the branch's, and the probe is `probe_re343_door_flips`: D [42021277032](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42021277032) (Core Interior, Einhoven, RE1 Architecture, the two office files and R2025 model 12), E [42021280392](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42021280392) (the held-out models, counts only) and F [42021284465](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42021284465) (all 58 samples again: the `models` filter does not narrow the samples; for the fields on the decoded records).
- **U, T, D', E', F', pending:** the head `e2f8acccf946e5263b06088603ee2cfda11895e1`. U [42024095189](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42024095189) is the unfiltered Measure against `main` (`-f base=main -f nordic=true -f r2025=true -f holdout=true -f samples=true`), for the IFC being byte for byte `main`'s on every model; T [42024099050](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42024099050) is the timing run (`-f timing=true -f samples=true -f scorers=none -f base=main`); D' [42024103319](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42024103319), E' [42024107131](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42024107131) and F' [42024111444](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42024111444) repeat D, E and F. **None read when this report was written.**

```bash
gh workflow run measure.yml --ref main -f ref=f65fc702c03798f838619ee20cb060ed71b1899b -f base=none -f scorers=none \
  -f nordic=true -f r2025=true -f probe=probe_re342_door_flips -f probe_scorer=door_flips_vs_ifc \
  -f models="core-interior einhoven re1-architecture nordic-office-* r2025-1*"
gh workflow run measure.yml --ref main -f ref=d8b2d2b54c54335a956a64c49f4cd3a02470b874 -f base=none -f scorers=none \
  -f samples=true -f probe=probe_re342_door_flips -f models="sample-*"
```

Snowdon Towers, on a Mac with its files: download run C's `sample-2024-snowdon_towers_sample_architectural/probe.txt`, then

```bash
python3 tools/re/door_flips_vs_ifc.py probe.txt "Snowdon Towers Sample Architectural_IFC4.ifc"
```

Runs D, E and F use the probe's final name, `probe_re343_door_flips`.
