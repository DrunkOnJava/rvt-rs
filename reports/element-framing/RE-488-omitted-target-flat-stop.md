# RE-488: A wall end whose join step names an omitted target stops flat at the partner's near face

**Date:** 2026-11-15
**Issue:** #1814 (wall ends at butt joins on the BIM4LCA housing); follows RE-71 (layered butt joins nest like concentric Ls), RE-70 (who runs through), RE-127 (a stop the joined-wall lists decide), RE-252 (`JoinEndGStep`), RE-447 (a non-empty `JoinEndGStep` as a clue).
**Artefacts:** the BIM4LCA housing (concrete and timber, at building permit, Revit 2024) with Revit's own IFC4 exports, for the ends; the BIM4LCA office, Core Interior, RE1, the Revit 2025 models, the Autodesk samples of Revit 2015 to 2027 and the held-out projects (counts only) for what else moves.
**Probe and scorer:** `probe_re252_wall_ends` and `tools/re/wall_ends_list.py`, both on `main`.
**Status:** measurement pending: the rule is written and its full Measure follows (section 4).

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

In `butt_joins`, at a perpendicular layered butt join where the wall stops and its end's `JoinEndGStep` names an omitted target (`WallLine::omitted`, read from the native record, `NativeWall::omitted`), every layer reaches the partner's near face, where one already does. An end whose layers do not reach the near face (wall 1567716) is left as it is.

## 4. Measured

TBD

## 5. The registered rows

Registered on #1814 before the run (2026-11-15, comment 6896106522):

1. **BIM4LCA housing and office.** The 21 scored ends of section 2 become Revit's to 0.001 ft, and no other wall end on these files moves.
2. **The other files with a Revit export** (Core Interior, RE1, the Revit 2025 models, the BIM4LCA terrain files). No wall end moves, or each one that moves becomes Revit's.
3. **Every scorer.** `different` and the values Revit does not write do not rise on any file.
4. **The samples.** Walls that move are counted per release, and `same-project gates` passes.
5. **Held-out.** Counts only, reported.

Read from the unfiltered Measure: TBD.

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
