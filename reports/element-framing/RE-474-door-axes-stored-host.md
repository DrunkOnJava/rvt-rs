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

Every door bound to none has a stored host, the wall its native record's `m_hostId` names (RE-361). Its native record gives a straight line and `m_isFlipped` on 67 of the office as built's 75. It is an ordinary wall, the same kind as the walled doors' hosts. Split by that flip, the 21 doors the flips get wrong are exactly the 19 in flipped walls plus 2 with no wall record. Of the doors in walls whose flip is read:

| file | doors bound to none | with a read host wall | flips alone | flips xor wall flip |
|---|---:|---:|---:|---:|
| office as built | 75 | 67 | 48 | **67** |
| office at permit | 77 | 69 | 48 | **69** |
| timber office as built | 76 | 68 | 47 | **68** |
| timber office at permit | 76 | 68 | 47 | **68** |
| RE1 Architecture | 1 | 1 | 0 | **1** |

That is 273 of 273 doors with 0 wrong, where the flips alone are wrong on 86. The doors bound to none in the housing files (8 per file) and 8 per office file have no such host record, so they are outside the rule.

For the doors rvt-rs binds to a plain wall, the wall's flip is not part of the rule. The flips alone are exact there (RE-347), and "xor the wall's flip" would be wrong on 33 to 70 of them per file. These are the doors RE-357 found Revit's export gives no opening: 75, 77 and 1. So the difference is between a door Revit cuts into its wall and one it does not. Why revit-ifc's transform differs there is not read; the rule is the measurement.

## 4. The rule

A door or window that rvt-rs binds to no plain wall is placed on these axes, under three conditions:
- its native record's `m_hostId` names a wall that is not a curtain wall, whose native record gives one straight level line and a flip;
- its two flips are read;
- its stored transform is upright, its X runs along that wall, and that X is within `INSTANCE_TURN_MIN_RADIANS` of a model axis. These are the same checks as RE-347.

The axes are the stored X, reversed where the flips differ or the wall's `m_isFlipped` is set, but not both. The wall keeps its flip beside its native direction (`m_wall_native_flipped`). The door is marked `m_opening_placement_from_stored_host`, and the writer says so with `PlacementAxesSource` `stored_axes_reversed_by_flips_and_host_wall_flip`.

## 5. Measured on the rule

To come.
