# RE-488: A wall end whose join step names an omitted target stops flat at the partner's near face

**Date:** 2026-11-15
**Issue:** #1814 (wall ends at butt joins on the BIM4LCA housing); follows RE-71 (layered butt joins nest like concentric Ls), RE-70 (who runs through), RE-127 (a stop the joined-wall lists decide), RE-252 (`JoinEndGStep`), RE-447 (a non-empty `JoinEndGStep` as a clue).
**Artefacts:** the BIM4LCA housing (concrete and timber, at building permit, Revit 2024) with Revit's own IFC4 exports, for the ends; the BIM4LCA office, Core Interior, RE1, the Revit 2025 models, the Autodesk samples of Revit 2015 to 2027 and the held-out projects (counts only) for what else moves.
**Probe and scorer:** `probe_re252_wall_ends` and `tools/re/wall_ends_list.py`, both on `main`.
**Status:** pending. Run 1 met its first row on 10 ends and broke one exact end (section 4); the narrowed rule's run follows.

## 1. Revit draws a layered wall's end layer by layer

Revit's export of the concrete housing at permit draws each wall as one body whose plan outline steps at its ends: each layer ends at its own place. Read from the export's own geometry (IfcOpenShell, each wall in its local frame), at the corners of the six walls #1814 names, a wall's layer `i`, counted from its exterior, ends at a boundary of the partner's layer `i`: the nested Ls of RE-71. `wall_ends_list` measures a wall's end as the furthest point of its body, so the end it compares is the layer that runs furthest.

At an end the partition reader decides by the walls' join lists, it gives each layer that reach (`layer_reaches`, RE-71) and draws the body to the furthest.

## 2. Where the join step names an omitted target, Revit's end is flat

`JoinEndGStep` holds, per end, a weak index, an omitted target (`m_omittedTarget`: a wall and its end) and the exposed-face map (RE-252). `probe_re252_wall_ends` prints them beside the partition reader's `m_wall_join_*_layer_reaches`, and `wall_ends_list` gives rvt-rs's error at each end against Revit's export (Measure [43724083677](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43724083677), `main` at `156e2b25`, the eight BIM4LCA office and housing files).

Every end where the wall stops (it does not run through), its layer reaches spread, the nearest of them is the partner's near face, and its `JoinEndGStep` names an omitted target: 24 ends on four files. Revit's end at each is flat at the partner's near face, where rvt-rs draws it to the furthest reach:

| file | such ends | Revit's end at the near face | rvt-rs past it by | not scored |
|---|---:|---:|---|---:|
| concrete housing, permit | 11 | 10 of 10 | 435, 390 or 255 mm | 1 |
| timber housing, as built | 5 | 5 of 5 | 28 mm | 0 |
| timber housing, permit | 5 | 5 of 5 | 28 mm | 0 |
| timber office, as built | 2 | 1 of 1 | 26 mm | 1 |
| timber office, permit | 1 | - | - | 1 |

The three not scored are walls `wall_ends_list` does not pair with Revit's (1550779 and 3598178). In Revit's export, wall 1566542's start is flat at its partner 1566543's near face: every layer ends there.

The omitted target is enough for a flat end, not needed for one. On the concrete office, Revit's end is at the nearest reach at 9 ends (3 as built, 6 at permit) where the reaches spread 330 or 405 mm and `JoinEndGStep` names nothing; what puts those there is not read here, and they are left as they are. Ends with an omitted target whose reaches do not spread are left as they are: on the concrete housing at permit, wall 1567716's two ends, whose layers reach the partner's far face and which Revit ends at the joint, are off by 195 mm and stay so.

## 3. The rule

In `butt_joins`, at a perpendicular layered butt join where the wall stops, its end's `JoinEndGStep` names an omitted target, and one of its layer reaches is already the partner's near face, every layer reaches the near face. The omitted target is read from the native record (`NativeWall::omitted`) and carried by axis end (`WallLine::omitted`), only where the record holds no `JoinToRoofGStep` (`NativeWall::roof_joined`, section 4).

An end whose layers do not reach the near face (wall 1567716) is left as it is, and so is a T joint (`tee_join`, RE-73), which the rule does not reach.

## 4. Measured

**Run 1** (Measure [43724665649](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43724665649), the first version at `2b10b372` against `main` at `156e2b25`, nordic, r2025, samples and holdout, 120 exports). That version had no roof-join condition:
- **Exports.** 119 of 120 are byte-identical to `main`'s, and so is `diagnostics.json` on 102 of 102. Only the concrete housing at permit moves.
- **Fixed.** On that file, 10 of section 2's ends become Revit's (`wall_ends_list`: walls with an end off by more than 0.001 ft, 71 to 62).
- **Broken.** Wall 1550779's end, Revit's within 0.001 ft on `main`, becomes 435 mm short. Section 2 counted it as not scored: `wall_ends_list` lists only the walls with an end off, and it was not off.
- **Not reached.** The 10 timber housing ends and the timber office's 1 did not move: they are T joints, read by `tee_join`, which the rule did not touch (a diagnostic build at `7f9d484b`, Measure [43733229564](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43733229564)).
- **Fits.** `frames_vs_ifc`'s fitted rotation (-60.5942 to -60.6411 deg) and `wall_heights_vs_ifc`'s median wall-middle residual (0.0093 to 0.0136 ft) move with the 10 long walls. These are fits over every wall; why a residual rises after corrections that are all exact is not read.

**Why 1550779.** In Revit's export, walls 1550779 and 1566548 make the same corner in plan, with the same record data: reaches, join steps, controls, an omitted target and weak index 1. 1566548 (Level_04, 2705 mm tall) ends flat at its partner's near face over its whole height. 1550779 (Level_03, 3100 mm tall) ends flat there from its base to 2705 mm, and over its top 395 mm it runs on, stepped, to the furthest reach. Its partner 1550778 is cut back 405 mm in plan over the same band. The record difference is that 1550777, 1550778 and 1550779 each hold two `JoinToRoofGStep` (a roof or floor joined to the wall), where the Level_04 walls hold none.

The rule therefore leaves out every wall whose record holds a `JoinToRoofGStep`. That is narrower than the evidence needs: of section 2's 21 ends, the 13 on roof-joined walls include 12 that Revit draws flat (1550777's start, 1567742's end, the 10 timber housing T ends and the timber office's 1). Only 1550779 is not drawn flat. What separates it from the other 12 is not read. On the eight files the rule then reaches 8 ends, all on the concrete housing at permit.

**Run 2** (the narrowed rule): TBD.

## 5. The registered rows

Registered on #1814 before the run (2026-11-15, comment 6896106522):

1. **BIM4LCA housing and office.** The 21 scored ends of section 2 become Revit's to 0.001 ft, and no other wall end on these files moves.
2. **The other files with a Revit export** (Core Interior, RE1, the Revit 2025 models, the BIM4LCA terrain files). No wall end moves, or each one that moves becomes Revit's.
3. **Every scorer.** `different` and the values Revit does not write do not rise on any file.
4. **The samples.** Walls that move are counted per release, and `same-project gates` passes.
5. **Held-out.** Counts only, reported.

Run 1 against these rows:
- **Row 1: not met.** 10 of the 21 ends became Revit's, the T joints did not move, and one exact end (1550779) broke.
- **Rows 2 to 5: met.** No other export moved, and the held-out files are unchanged.

**Rows for run 2**, the narrowed rule, registered on #1814 before it ran (2026-11-15):
1. **Concrete housing at permit.** These 8 ends become Revit's to 0.001 ft: 1346762's start, 1346792's end, 1437589's start, 1437595's end, 1550773's start, 1566542's start, 1566546's start and 1566548's end. 1550779 stays Revit's, and no other end on the file moves.
2. **Every other export** is byte-identical to `main`'s, the timber housing and the office included.
3. **Every scorer.** `different` and the values Revit does not write do not rise.
4. **The samples.** No sample export moves, and `same-project gates` passes.
5. **Held-out.** Counts only, unchanged or reported.

Read from run 2: TBD.

## 6. Where it rests

- **Two projects, one exporter.** The ends are the BIM4LCA housing's (20) and office's (1), saves of two buildings by Revit 2024.
- **What the omitted target means is not read.** The rule rests on the observed flat end at every such end of the two files, not on a decoded meaning of `m_omittedTarget`.

## 7. Reproduce

```bash
# the ends, their join steps and rvt-rs's errors (the eight BIM4LCA office and housing files)
gh workflow run measure.yml --ref main -f ref=156e2b257f610c8c57c82828571a892b687ebb77 -f base=none -f nordic=true \
  -f models="nordic-office-as-built nordic-office-permit nordic-office-timber-as-built nordic-office-timber-permit nordic-housing-concrete-as-built nordic-housing-concrete-permit nordic-housing-timber-as-built nordic-housing-timber-permit" \
  -f scorers=wall_ends_list -f probe=probe_re252_wall_ends -f probe_scorer=-
```
