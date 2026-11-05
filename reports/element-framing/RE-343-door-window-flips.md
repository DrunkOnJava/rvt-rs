# RE-343 — A door's or window's hand flip and facing flip

**Date:** 2026-11-05
**Issues:** #1357; follows RE-87 (instance transforms), RE-167 (the type GlobalId's flip, B72) and RE-216 (the current copy of an instance)
**Probe:** `probes/examples/probe_re343_door_flips.rs` (runs A and C below used it under its earlier name, `probe_re342_door_flips`, at the commits named there)
**Scorer:** `tools/re/door_flips_vs_ifc.py`
**Status:** positive, with one limit stated in section 6. The flips are two booleans of the instance's native `FamilyInstance` record, `m_flippedX` and `m_flippedY`. No door or window is stored as a reflection, so nothing is in the transform. Revit's own placement is exactly the stored axes, reversed where the two booleans differ, on every door and window rvt-rs binds to a plain wall whose record it reads (653 of 813 doors and 626 of 626 windows over the open models and Snowdon Towers; counted once, without the permit office file and five of the six copies of the R2025 layout, 456 doors and 346 windows); the 160 doors rvt-rs binds to no plain wall are counted apart, disagree on 49, and are not claimed.

## 0. The two questions

**If the instance transform is stored mirrored, can the flips be read from it?** No instance is stored mirrored. The first orthonormal 3 x 3 in the element data has determinant +1 on all 1,439 doors and windows scored against Revit's export (run A and Snowdon Towers), and on all 1,820 doors and windows of the Autodesk samples of 2024 to 2027 (run C); none has -1. Revit's own placements have determinant 1 on all 1,439, and so does the native `InstanceInfo.m_Trf.m_3x3` on all 1,219 doors and windows of Core Interior, RE1 Architecture, the office files and R2025 model 12 (run D', every one). The reader of RE-87 therefore skips nothing, and the flips are not in the transform. They are in the native record (section 2).

**Does `m_opening_facing` already include the facing flip?** It includes one of the two flips, and not the one the name suggests. `m_opening_facing_x/y` is the stored transform's Y axis (RE-87). That axis is a half turn from the unflipped one exactly where `m_flippedX` is set, and `m_flippedY` does not change it. For an element set in a plain wall rvt-rs binds, the direction the door faces (the +Y of Revit's own placement) is the stored Y axis reversed where `m_flippedX` and `m_flippedY` differ; for a door rvt-rs binds to no plain wall that was not so on 49 of 160 (section 3), and nothing is claimed. The placement's +Y is where IFC4 has the leaf open, which is where it opens only for a family whose swing arcs lie on its +Y side: revit-ifc gives a door a left or right operation only from such arcs, and 76 of the office's 269 doors have none (`NOTDEFINED`). So `m_opening_facing` is the facing direction on the walled doors whose two booleans are equal, and the opposite of it on the others: 287 of the 653 doors and 286 of the 626 windows (counted once: 186 of 456 and 146 of 346), Core Interior 39 of 132 doors, Snowdon Towers 44 of 126. By release, the second case is measured on three 2024 buildings (Core Interior, the office, Snowdon Towers), one 2025 door (RE1 Architecture), one 2026 house (7 doors and 6 windows, held out) and no 2027 element.

## 1. What Revit's exporter does with the flips

(revit-ifc, CLEANROOM.md source 5; read for behaviour, not copied)

- `DoorWindowInfo` takes its `FlippedX` from `FamilyInstance.HandFlipped` and its `FlippedY` from `FacingFlipped`. A door's `OperationType` is its type's "Operation" string, reversed when `FlippedX` differs from `FlippedY`; where the type names none, `CalculateDoorOperationStyle` guesses it from the symbol's plan arcs after `ExporterIFCUtils.GetTransformForDoorOrWindow(.., FlippedX, FlippedY)` (a RevitAPI name; its body is not read). A left or right swing is assigned only from an arc on the +Y side of that frame.
- The type object is split by `FlippedSymbol`, whether the door's geometry centre and its Y axis lie on the same side of the host wall's axis (RE-167), which is false for any element whose host is not a wall with a line or arc axis. It is neither flip alone.
- `FamilyInstanceExporter` multiplies the instance transform by the type's `StyleTransform` (`trf = trf.Multiply(typeInfo.StyleTransform)`), and `PlacementSetter` builds the `IfcLocalPlacement` from that product's origin, `BasisZ` and `BasisX` (`ExporterUtil.CreateLocalPlacement`), so the placement's Y is derived, `Z x X`. For a door or window `StyleTransform` is `doorWindowTrf.Inverse` (`GetTransformForDoorOrWindow` with the creating instance's flips), set once, when the type is created; an instance whose type key is found in the cache reuses it. The type key is the symbol, the level, `FlippedSymbol`, the export type, the material and whether the instance is in an assembly. And `CreateTypeEntityHandle` takes the `DoorWindowInfo` of the instance that creates the type, so a type's operation is its creator's.
- buildingSMART: `SINGLE_SWING_LEFT` has its hinges on the left seen along the placement's +Y, and the leaf opens toward +Y.
- The public Revit API guidance (Scott Conover, AU 2009, on the Building Coder, `0290_abg07_transform`): "If a family instance is flipped in X, its FlippedHand property is true. If it is flipped in Y, its FlippedFacing property is true"; the transform's Y direction is the nominal facing direction, and where exactly one of them is set the transform is left-handed and the direction must be reversed. `FacingOrientation` is that calculation. The Revit API Developer's Guide (Family Instances) says the same of `Mirrored`: false for the two doors placed by default, true for the two made by flipping the hand. The article and the guide are Autodesk's public documentation of its public API; they say what the two API properties mean, not how a file is laid out, and no byte of this report rests on them. They are not one of the five numbered sources of CLEANROOM.md; they are cited here so that the decision on them is the maintainer's.

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

| model (run, head) | doors bound to a plain wall: checked / wrong | windows: checked / wrong | doors bound to none: checked / wrong |
|---|---:|---:|---:|
| Core Interior (A, `f65fc702`) | 132 / 0 | 6 / 0 | 0 |
| RE1 Architecture (A) | 4 / 0 | none | 1 / 1 |
| office as built (A) | 193 / 0 | 269 / 0 | 76 / 21 |
| office at permit (A) | 192 / 0 | 265 / 0 | 77 / 23 |
| R2025 models 12 to 17 (A, each) | 1 / 0 | 3 / 0 | 0 |
| Snowdon Towers (C probe, scorer at `d8b2d2b5`, local) | 126 / 0 | 68 / 0 | 6 / 4 |
| total | 653 / 0 | 626 / 0 | 160 / 49 |

**Rule 2: against the host wall, the stored axes are a half turn from the wall's direction and left exactly where `m_flippedX` is set, and Revit's placement exactly where `m_flippedY` is set.** 0 wrong on the same 653 doors and 626 windows. In words: with `m_flippedY` false, Revit's placement faces left of its wall's curve and its X runs along it; stored axes do so with `m_flippedX` false. This is what the first table's states do to the axes: `01` turns the placement and not the stored axes, `10` turns the stored axes and not the placement, `11` turns both.

**What is left out.** The 160 doors rvt-rs binds to no plain wall follow Rule 1 on 111 and not on 49, so nothing is claimed for them. 159 of the 160 have no host in rvt-rs's own export (an element whose only candidate is a curtain wall keeps none, `bind_opening_hosts`; why the others have none is not measured); one has a host whose wall record is not read. Every window is bound to a wall. The fields in section 5 are written for these doors too, as the record holds them, but no rule links those to Revit's placement.

The 49 are exact inversions of Rule 1 (the placement is the stored axes turned the other way), and they are uniform by type. In the two office files, 60 of 60 `IfcDoorType` entities that hold such a door have every such door right or every one wrong (as built 29 of 29, 9 of them wrong); only one type holds both a door bound to a wall and one bound to none, and none of its doors is wrong. They fall in states `00` (8), `01` (30) and `10` (6), and in `11` in none of the 44 on the two office files, RE1's one door `10` and Snowdon Towers' four `00`. That fits revit-ifc's mechanism (section 1): the placement is the instance's transform times its type's `StyleTransform`, which comes from the instance that created the type, and the type key's `FlippedSymbol` is false for every element not in a wall with a line or arc axis, so one type holds doors of several flip states and an instance whose flips differ from its type creator's gets an inverted placement. It is not shown: which instance created each type is not recorded in the file, and `GetTransformForDoorOrWindow` is a Revit call not read here. Nor is it explained why the doors in a wall never show it. As a cross-check, RE-87's 49 reversed doors on Snowdon Towers are 44 walled mixed-state doors, 1 unwalled one and the 4 inverted unwalled ones.

**Held-out models** (run E at `7d346f1f`, counts only; Revit's exports of 18 models of 2018 to 2027). 426 doors and 630 windows read their booleans. The 2024 to 2027 models (41 doors, 105 windows) have a transform and a host wall: 0 wrong on Rule 1 and on Rule 2. Only one of them has a mixed state, a 2026 house (13 doors: 2 / 2 / 5 / 4, 16 windows: 9 / 0 / 6 / 1 in the order of the table above), so 7 doors and 6 windows test the facing rule; all the other 2024 to 2027 held-out doors and windows are `00` except one window, `11`, and prove the rule no more than the R2025 models do. The 2018 to 2023 models (385 doors and 525 windows, every state populated on the larger ones) have no transform, so Rules 1 and 2 cannot be checked on them; the operation side below holds on all 5 symbols with doors in both relations across the held-out models: the 2018 model 1 of 1, the 2020 one 1 of 1, the 2021 one 2 of 2 and the 2026 house 1 of 1.

**Operation side.** Within one symbol, a left or right `OperationType` differs between two doors exactly when the flags differ (`m_flippedX xor m_flippedY`), as revit-ifc's `FlippedX ^ FlippedY` says:

| model | pairs of doors of one symbol (against the symbol's first), `m_flippedX xor m_flippedY` agrees | `m_flippedX` alone | `m_flippedY` alone | stored Y's side of the wall | symbols with doors in both relations, one side in each and the sides differ |
|---|---:|---:|---:|---:|---:|
| office as built | 161 of 162 | 102 of 162 | 86 of 162 | 89 of 159 | 13 of 14 |
| office at permit | 161 of 161 | 101 of 161 | 86 of 161 | 88 of 159 | 13 of 13 |
| RE1 Architecture | 1 of 1 | 0 of 1 | 1 of 1 | not scored | 1 of 1 |
| Snowdon Towers | 108 of 108 | 78 of 108 | 70 of 108 | 87 of 105 | 9 of 9 |

The one exception is a symbol of the as-built office with two doors, `01` and `00`, that Revit's export gives the same `SINGLE_SWING_RIGHT`. The two sit in different `IfcDoorType` entities of the symbol, which revit-ifc splits by `FlippedSymbol` and level, and a door's operation in the export is its type's, taken from the instance that created the type (section 1): so a pair's operations follow the types' creators, not the pair's own flags, and the pair is the case where that differs. That is a reading of the source, not shown for this pair. Core Interior's export (Revit 24.0.20) gives all 132 doors and 10 door types `NOTDEFINED`, so it says nothing of operation; RE1's two Single-Flush doors (`00`: `SINGLE_SWING_RIGHT`, `01`: `SINGLE_SWING_LEFT`) are one pair. A flag alone, or the stored Y's side of the wall, agrees with the operation's parity on little more than half the pairs, which is why the hinge side cannot be had from `m_opening_facing` alone.

## 4. Which boolean is the hand and which the facing

Revit's API names are `HandFlipped` and `FacingFlipped`; neither is in the export on its own. The assignment `m_flippedX` = hand, `m_flippedY` = facing rests on three things, and no more:

1. The public API guidance quoted in section 1 (flipped in X is the hand, in Y the facing), and revit-ifc's own `FlippedX`/`FlippedY` naming.
2. Applying that guidance's rule (reverse the transform's Y where exactly one flip is set) to the stored Y gives a direction equal to Revit's placement +Y on all 653 doors and 626 windows of Rule 1, in every state the models hold; that direction is left of the wall's curve exactly where `m_flippedY` is false. So `m_flippedY` alone decides which side of the wall the door faces, which is what `FacingFlipped` records.
3. The operation side changes with the xor of the two (the table above), as `FlippedX ^ FlippedY` does in revit-ifc.

Not shown: any export field, or any Revit file written both ways, that names the hand on its own. Which of the two a consumer should call the hand is the API's `m_flippedX`; if that reading is wrong the facing rule of section 0 is unchanged, since it uses only the xor and `m_flippedY`'s side.

## 5. What is exposed

Two fields on the decoded Door and Window records, from the element's current native record and present only where it holds both booleans (`partition_schema_mvp/openings.rs`):

- `m_hand_flipped`: `m_flippedX`
- `m_facing_flipped`: `m_flippedY`

They are booleans and carry no claim the rules above do not measure. The IFC rvt-rs writes does not read them and is not changed. What was measured is Revit's placement against the stored axes (the "reversed" doors of RE-87, which the flags now explain: section 3); what rvt-rs writes for a door's or window's local axes is not measured here (it takes the stored X for a turned box only, RE-87), and writing Revit's would change every such placement in the IFC, so it is not done here. #1357's follow-up issue is where that is measured and decided. The typed `Door` view (`flip_hand`, `flip_facing`) reads the new fields; before it matched names no record carries, so it was null on every door. Measured at `18954f33` (runs D'' and F'', section 7; the probe prints the view's values, `typed_flip_hand` and `typed_flip_facing`, and the scorer counts those equal to the booleans): the typed Door view carries both flips, equal to the native booleans, on 132 of 132 doors of Core Interior, 5 of 5 of RE1 Architecture, 1 of 1 of R2025 model 12, 268 of the 268 doors of the office as built and 268 of the 268 at permit that carry the fields, and 1,588 of 1,588 doors of the 2024 to 2027 samples; on none of the 1,540 doors of 2015 to 2023. The fields themselves are as at `e2f8accc`: Core Interior 132 of 132 doors and 6 of 6 windows, RE1 5 of 5, the office 268 of 269 doors and 269 of 269 windows as built and 268 of 269 and 265 of 265 at permit, 1,820 of 1,820 of the 2024 to 2027 samples and none of the 1,909 of 2015 to 2023; both rules have 0 wrong on every model.

Measured at `7d346f1f`, where a pass of its own read the booleans (runs D, E and F, section 7): the decoded fields equal the native booleans on every element that carries them.

- Core Interior 132 of 132 doors and 6 of 6 windows carry them; RE1 Architecture 5 of 5; R2025 model 12 1 of 1 and 3 of 3; the office as built 268 of 269 doors and 269 of 269 windows, at permit 268 of 269 and 265 of 265. The one door of each office file that carries none (it has native flags, `01`) is exported as an `IfcDoor` from an element that is not in the decoded Door list; it carries `m_opening_facing` no more than the fields.
- The 58 samples: 1,820 of 1,820 doors and windows of 2024 to 2027 carry both fields and `m_opening_facing`, equal to the native booleans; none of 2015 to 2023.
- The held-out models, counts only: every door and window of the 2024 to 2027 ones (41 doors, 105 windows) carries them, equal; none of the 2018 to 2023 ones.

**The read is in an existing pass (head `e2f8accc`).** `instance_cuts_and_symbols` (RE-215, RE-245) already reads each door's and window's `FamilyInstance` record on 2024 and 2025 for its symbol and host; it now also takes `m_flippedX` and `m_flippedY` from that record, for every door and window (those that are an opening alone too, as before), and `attach_opening_flips` makes no pass of its own there. On 2026 and 2027 (and on those releases only), which no pass reads, the dedicated pass (`graph_classes` limited to `FamilyInstance`, the doors' and windows' ids) remains: one `extract_graphs` over the file. Both take the element's one current record (`native_document` allows one per channel and id and fails on a second), so there is no second record to compare. Measured on `e2f8accc` (runs U, T, D', E' and F', section 7):

- **The fields and the rules are as before.** D' (the open models), E' (the held-out models: 41 of 41 doors and 105 of 105 windows of 2024 to 2027 carry the fields, equal; 426 and 630 read natively) and F' (the samples: 1,820 of 1,820 of 2024 to 2027, 0 of 1,909 of 2015 to 2023) repeat D, E and F count for count, and both rules have 0 wrong on every model.
- **The IFC is `main`'s.** In the unfiltered run (U: head `e2f8accc` against `main` at `f8612274`, `-f nordic=true -f r2025=true -f holdout=true -f samples=true`) `ifc.sha256` is the same on head and base on all 111 models: 34 reference, office and R2025 models, 19 held-out and spent models and 58 samples. No scorer output differs, and the semantic diff of the MCP dumps is empty on every model that has one.
- **Time and peak memory** (T: `rvt-ifc` on each of 64 files, base `main` against head, one run each, each shard on one runner of the kinds `timing-runners.txt` lists):

| files | time, head / base: min to max (mean) | peak memory, head / base |
|---|---|---|
| Core Interior, RE1 x 4 and Einhoven (the cut-loop pass reads the flips on all but Einhoven, 2023, where no pass runs) | 0.989 to 1.012 (1.002) | 1.000 on all 6 |
| samples 2024, 10 files (largest: Snowdon Towers Architectural, 26,209 to 26,288 ms) | 0.990 to 1.004 (0.998) | 1.000 on all 10 (Snowdon 1,108 MiB both) |
| samples 2025, 5 files | 0.991 to 1.001 (0.997) | 0.996 to 1.000 (at most 1 MiB) |
| samples 2026, 5 files (a pass of its own) | 0.996 to 1.016 (1.004) | 1.000 on all 5 |
| samples 2027, 5 files (a pass of its own) | 0.999 to 1.008 (1.003) | 1.000 to 1.002 (at most 1 MiB) |
| samples 2015 to 2023, 33 files (no Door or Window record, so no pass; nothing here changed) | 0.929 to 1.074 (0.997) | 0.989 to 1.010 (at most 1 MiB) |

  No file's peak moves by more than 1 MiB, and none of the 2024 to 2027 files' by more than 0.2%. On time, the folded releases are within 1.2% of `main` on every file, and the files of 2026 and 2027 within 1.6%: the worst is `2026-racadvancedsampleproject`, 3,548 to 3,606 ms (124 doors and windows), the rest of those ten +0.8% or less. The 33 files of 2015 to 2023, where the change does nothing, vary by 7% either way, so a run of this kind cannot resolve a cost under about 2% on a file of a few seconds: 2026 and 2027 add one pass over the file, and what it costs there is at most that, and about 0.3% on average over those ten files. The 64 files together: 202,606 ms on `main`, 202,492 ms on the head.

**The merges of `main`.** `main` gained #1426 and #1428 (`aae3b779`), then #1439 and #1367 (`a0639777`) and #1436 (`afec5e37`) after the runs of `e2f8accc`; the branch is merged with each, signed (the conflicts were all in the appended addenda of `docs/format-reconnaissance.md`, every addendum kept). The code of the merged head differs from what U measured in two ways: the second review's edits to this change (the typed Door view, the gate, the current record, `InstanceCuts`), and everything `main` gained, so U is no longer the run for it. G (above) measured this change's second-review head against `main` at `a0639777` on the two office files and the 58 samples. A final unfiltered run was dispatched on the merged head `80a9d72c4947fff0fec5b5027cd53180ec242d20` (`-f base=main -f nordic=true -f r2025=true -f holdout=true -f samples=true`): [42040585454](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42040585454). **Not read when this report was written.**

## 6. Not claimed, not measured

- The hand/facing naming beyond section 4; the hinge side (the family's native hinge side is not read: only its parity with the flags is shown); the swing side of a door whose type names an Operation or whose export gives `NOTDEFINED`.
- The 160 doors without a host wall record; any wall that is not straight; a wall record that is not read.
- Revit 2026 and 2027 against an export: the open models are 2024 and 2025. Revit 2026 has the held-out house (7 doors and 6 windows in a mixed state, 0 wrong); Revit 2027 only held-out models whose doors and windows are `00` or `11` (7 doors, 22 windows), which agree and do not test the rule. The first held-out run (B) produced no count lines: the scrub of the workflow's own ref, `main`, did not know them.
- Snowdon Towers is local only; its figures are the run C probe output scored on the Mac against Revit's IFC4 export (sha256 `ecfcb04e3e818090cf818873fe76fba8b447742bbf4d19dad6d590f09cbb985a`), counts only.
- Independence. The two office files are one building and R2025 models 12 to 17 are one layout six times; counted once, the walled doors and windows are 456 and 346, and the mixed-state ones 186 and 146 (the office alone 102 doors and 144 windows of them). The second case is measured on a single 2025 door, a single 2026 house and no 2027 element; Rules 1 and 2 hold on those, which is the whole of the evidence for those releases.
- `InstanceInfo.m_Trf.m_3x3` of Snowdon Towers and of the held-out models was not scored.

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
- **U, T, D', E', F', read:** the head `e2f8acccf946e5263b06088603ee2cfda11895e1`. U [42024095189](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42024095189) is the unfiltered Measure against `main` (`-f base=main -f nordic=true -f r2025=true -f holdout=true -f samples=true`), for the IFC being byte for byte `main`'s on every model; T [42024099050](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42024099050) is the timing run (`-f timing=true -f samples=true -f scorers=none -f base=main`); D' [42024103319](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42024103319), E' [42024107131](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42024107131) and F' [42024111444](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42024111444) repeat D, E and F. All five finished with success and are read in section 5.
- **G, D'', F'', read:** the code after the second review, `18954f33f5c081f0861229a18de336563295e936` (the condition in the rustdoc, the flips taken from the current record only, the own pass gated on 2026 and 2027, the typed Door view wired, `InstanceCuts` a struct): G [42037262675](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42037262675) is a filtered byte-identity run against `main` (`-f base=main -f nordic=true -f samples=true -f scorers=none -f models="nordic-office-as-built nordic-office-permit"`): the office files and the 58 samples; D'' [42037268099](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42037268099) and F'' [42037272808](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42037272808) repeat D' and F' for the fields and the typed view (section 5). G: `ifc.sha256` of the head equals `main`'s (`a0639777`) on all 60 models, the two office files and the 58 samples, and the semantic diff has no finding; the scorer diff's one line is a file `main`'s script writes only on the head side (`areas_across_saves.txt`), not an IFC.

```bash
gh workflow run measure.yml --ref main -f ref=f65fc702c03798f838619ee20cb060ed71b1899b -f base=none -f scorers=none \
  -f nordic=true -f r2025=true -f probe=probe_re342_door_flips -f probe_scorer=door_flips_vs_ifc \
  -f models="core-interior einhoven re1-architecture nordic-office-* r2025-1*"
gh workflow run measure.yml --ref main -f ref=d8b2d2b54c54335a956a64c49f4cd3a02470b874 -f base=none -f scorers=none \
  -f samples=true -f probe=probe_re342_door_flips -f models="sample-*"
```

The runs of the head (D', E', F', U, T) were dispatched with `--ref research/1357-door-window-flips`, a branch that goes when this lands. From a commit on `main` that holds this change, `<sha>`, the same runs are:

```bash
gh workflow run measure.yml --ref main -f ref=<sha> -f base=none -f scorers=none -f nordic=true -f r2025=true \
  -f probe=probe_re343_door_flips -f probe_scorer=door_flips_vs_ifc \
  -f models="core-interior einhoven re1-architecture nordic-office-as-built nordic-office-permit r2025-12*"        # D'
gh workflow run measure.yml --ref main -f ref=<sha> -f base=none -f scorers=none -f holdout=true \
  -f probe=probe_re343_door_flips -f probe_scorer=door_flips_vs_ifc -f models="holdout-*"                         # E'
gh workflow run measure.yml --ref main -f ref=<sha> -f base=none -f scorers=none -f samples=true \
  -f probe=probe_re343_door_flips                                                                               # F'
gh workflow run measure.yml --ref main -f ref=<sha> -f base=main -f nordic=true -f r2025=true -f holdout=true -f samples=true   # U
gh workflow run measure.yml --ref main -f ref=<sha> -f base=main -f timing=true -f samples=true -f scorers=none               # T
```

Until it lands, replace `--ref main` by `--ref research/1357-door-window-flips` and `<sha>` by `e2f8accc` or a later commit of the branch.

Snowdon Towers, on a Mac with its files: download run C's `sample-2024-snowdon_towers_sample_architectural/probe.txt`, then

```bash
python3 tools/re/door_flips_vs_ifc.py probe.txt "Snowdon Towers Sample Architectural_IFC4.ifc"
```

Runs D, E and F use the probe's final name, `probe_re343_door_flips`.
