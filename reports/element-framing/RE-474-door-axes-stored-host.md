# RE-474: a door rvt-rs binds to no plain wall turns with its own host wall's flip

**Date:** 2026-11-15
**Issues:** #1494; follows RE-343 (the hand and facing flips), RE-347 (the axes of a door or window set in a plain wall) and RE-357 (the origin of a door Revit's export gives no opening).
**Scorer:** `tools/re/door_type_creator_vs_ifc.py`, commit `71a9172a`, archived as `refs/archive/2026-11-15/cited/71a9172ab4e7`, not on `main`. It reads `probe_re343_door_flips` and Revit's own IFC4 exports.
**Status:** positive, and the rule is written.

## 1. The question

RE-347 places a door or window set in a plain wall rvt-rs binds at its stored X, reversed where its hand flip and facing flip differ. It is right on 1,061 of 1,061. The doors rvt-rs binds to no plain wall (154 to 160 in the open models) kept the default axes, because that rule is wrong on 49 of them, "uniformly by `IfcDoorType`". #1494 asked for the mechanism behind that uniformity.

## 2. What revit-ifc points at, and what it does not explain

revit-ifc (CLEANROOM.md source 5; `FamilyInstanceExporter.cs` and `DoorWindowInfo.cs`, read at `7fe314fd`) builds a door type's geometry once, from the first instance it exports with that type key. That instance's transform comes from its hand and facing flips (`GetTransformForDoorOrWindow`). Every later instance is then placed at its own transform times the inverse of that one (`typeInfo.StyleTransform`). The key holds `FlippedSymbol`, which only a door with a real wall host can set. So the hypothesis was that within one type entity, a door's axes follow the creating instance's flips.

The scorer tests that. For every door and window with a stored transform, it reads whether Revit reverses the stored X (`p`). It then tests four predictors:
- the door's own flips;
- one value per type entity;
- the flips of the type's first instance in Revit's file;
- the flips of the instance with the least ElementId.

The result (Measure [43683865183](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43683865183): Core Interior, RE1 Architecture and the BIM4LCA housing and office files):
- **One value per type entity holds everywhere.** 0 mixed type entities in any file.
- **The creating instance explains nothing more.** On the office as built's 75 doors bound to none, the first-in-file and least-id predictors score 54, exactly what the door's own flips score.
- That is because the inverted type entities are inverted whole. The same symbol in the same flip state is split across type entities that are reversed and ones that are not: symbol 2808209 has a type of five `01` doors that is inverted and one of five that is not.

## 3. The host wall's flip

Every door bound to none has a stored host, the element its native record's `m_hostId` names (RE-361). On 67 of the office as built's 75 that host's native record gives a straight line and `m_isFlipped`. rvt-rs decodes 66 of those hosts as `CurtainWall` (`probe_1494_door_axes_trace`, Measure [43687911646](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43687911646)). These are doors set in curtain walls, which is why Revit's export cuts no opening for them (RE-357) and rvt-rs binds them to no plain wall. Split by that flip, the 21 doors the flips get wrong are exactly the 19 in flipped walls plus 2 with no wall record. Of the doors in walls whose flip is read:

| file | doors bound to none | with a read host wall | flips alone | flips xor wall flip |
|---|---:|---:|---:|---:|
| office as built | 75 | 67 | 48 | **67** |
| office at permit | 77 | 69 | 48 | **69** |
| timber office as built | 76 | 68 | 47 | **68** |
| timber office at permit | 76 | 68 | 47 | **68** |
| RE1 Architecture | 1 | 1 | 0 | **1** |

That is 273 of 273 doors with 0 wrong, where the flips alone are wrong on 86. The doors bound to none in the housing files (8 per file) and 8 per office file have no such host record, so they are outside the rule.

For the doors rvt-rs binds to a plain wall, the wall's flip is not part of the rule. The flips alone are exact there (RE-347), and "xor the wall's flip" would be wrong on 33 to 70 of them per file. These are the doors RE-357 found Revit's export gives no opening: 75, 77 and 1. So the difference is between a door set in a wall Revit cuts and a door set in a curtain wall. Why revit-ifc's transform differs there is not read; the rule is the measurement.

## 4. The rule

A door or window that rvt-rs binds to no plain wall is placed on these axes, under three conditions:
- its native record's `m_hostId` names a curtain wall whose native record gives one straight level line and a flip;
- its two flips are read;
- its stored transform is upright, its X runs along that wall, and that X is within `INSTANCE_TURN_MIN_RADIANS` of a model axis. These are the same checks as RE-347.

The axes are the stored X, reversed where the flips differ or the wall's `m_isFlipped` is set, but not both. The wall keeps its flip beside its native direction (`m_wall_native_flipped`). The door is marked `m_opening_placement_from_stored_host`, and the writer says so with `PlacementAxesSource` `stored_axes_reversed_by_flips_and_host_wall_flip`.

## 5. Measured on the rule

The rule is in two commits. The first (`dfeb9fad`) read a door's stored host among plain walls only and wrote nothing: its full Measure, [43686098590](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43686098590), is byte-identical on all 120 exports. A trace (`probe_1494_door_axes_trace`, commit `a99a0028`, archived as `refs/archive/2026-11-15/cited/a99a0028e503`) showed why. The 66 hosts with a read line and flip on the office as built are decoded as `CurtainWall`, and the plain-wall map leaves them out. The second (`1be73ffb`) reads them among curtain walls.

On `1be73ffb` there are two runs: [43688985749](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43688985749) (`probe_re343_door_flips` and `door_placements_vs_ifc`, `keep_ifc`) and the full Measure [43688981702](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43688981702) against the merge base `4b1d8f19`. Rows, as registered in the pull request before they were read:

1. **Every door the rule places gets Revit's axes. Held.**

   | file | doors bound to none | placed by this rule | same as Revit |
   |---|---:|---:|---:|
   | office as built | 75 | 66 | 66 |
   | office at permit | 77 | 68 | 68 |
   | timber office as built | 76 | 67 | 67 |
   | timber office at permit | 76 | 67 | 67 |
   | RE1 Architecture | 1 | 1 | 1 |

   That is 269 of 269. The housing files' 8 doors per file name no curtain wall and are left as they were. So are 9 per office file: the 8 with no host record, and one per file whose transform fails the along-the-wall or near-axis check. `door_axes_vs_ifc` agrees: the doors placed on Revit's axes go from 194 to 260 on the office as built and from 4 to 5 on RE1, with 0 different from Revit everywhere. On the office as built, the doors whose axes are Revit's go from 212 to 261 of 269.
2. **Nothing the RE-347 rule places changes. Held.** Its doors and windows read as before on every file, the held-out ones included (counts only), each `same` as Revit's.
3. **Bodies and openings keep their place. Held.** No `diagnostics.json` changes (102 of 102), and the box keeps its world extents as in RE-347.
4. **Only exports with such doors move, and the same-project gates pass. Held.** The 22 that move are:
   - the four office files and RE1;
   - Autodesk's `rac_advanced`, `rac_basic`, `rme_advanced` and `rme_basic` saves of 2024 to 2027;
   - Snowdon Towers Architectural.

   The samples have no Revit export. The same-project check compares each of their doors with the same project's 2024 save, and it passes: `rac_advanced`, `rme_advanced` and `rme_basic` compare 124 doors where they compared 107, and `rac_basic` 22 where it compared 17. The gates are a success on `1be73ffb`.

What this does not show: that a door in a curtain wall of another building turns the same way. The doors are those of one office, in four files, and one door of RE1. The samples hold the rule to themselves across releases, not to Revit.
