# RE-222: A wall's native record gives its T joints and, where the partition data has none, its orientation

**Date:** 2026-10-25
**Issues:** W6 (every body Revit's own shape). Follows RE-53 and RE-54 (a wall's line, orientation and layers from its partition data), RE-70, RE-71 and RE-73 (a wall's ends from its join lists), RE-127 (joined-wall lists) and RE-216 (data copies in several partitions).
**Artefacts:**

| file | sha256 | licence |
|---|---|---|
| `ARK_NordicLCA_Office_Concrete_As-Built.rvt` (Revit 2024) | `747120005b4b35c7cb66ec61158aab50ded95e052e172cb387b86cd15b31d906` | CC BY-SA 4.0, VTT (`research/bim4lca-measure.tsv`) |
| `ARK_NordicLCA_Office_Concrete_As-Built_Revit.ifc` (Revit's IFC4 export) | `b2f530eba8b4424c4542b98e9a2c7bbcc726367f21b2740f3074e4a6ce917382` | CC BY-SA 4.0 |
| `ARK_NordicLCA_Office_Concrete_BuildingPermit.rvt` (Revit 2024) | `1c12c2058128315edbd65a8bf97f60da971aee8625f21233b8477d9c6165c1d1` | CC BY-SA 4.0 |
| `ARK_NordicLCA_Office_Concrete_BuildingPermit_Revit.ifc` (Revit's IFC4 export) | `bc3bd3366a3c413ce74840300a590f59102b740118ef37898e906253b6b7c08d` | CC BY-SA 4.0 |
| `2024_Core_Interior.rvt` and the four RE1 models | see `research/witness-registry.json` | MIT |

Measure fetches all of them and stores none.
**Probes:** `examples/probe_wall_curves.rs` (each wall's native curve and end joins), `examples/probe_wall_native_orientation.rs` (native against partition line and orientation), `examples/probe_wall_type_layers.rs` (which wall types get no centreline). **Scorers:** `tools/re/wall_ends_list.py` (each wall's two ends against Revit's, now in Measure), `tools/re/wall_curve_ends_vs_ifc.py` (Revit's, rvt-rs's and a rule's end against the native curve, per kind of end).
**Runs:**
- [40189190921](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40189190921): `wall_ends_list` on `main`: 195 of the office's 607 walls end away from Revit's body;
- [40191317754](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40191317754) and [40202682753](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40202682753): every wall's native curve and joins, and Revit's, rvt-rs's and the join-order rule's ends against it;
- [40203889405](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40203889405): native against partition line and orientation;
- [40204859227](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40204859227): the wall types whose walls get no centreline;
- [40204804355](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40204804355): the native line for walls whose copies disagree (§5, a measured negative);
- [40206135950](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40206135950): the T rule alone against `main`;
- [40206225586](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40206225586): this change against `main` (0e8ed49e).

**Status:** positive at T ends and for walls the partition data gives no orientation. Negative for a corner rule from the join order (§4) and for the native line where a wall's copies disagree (§5).

## 1. What was wrong

Core Interior's 360 walls end exactly where Revit's do. On the BIM4LCA office as built, 195 of 607 walls end away from Revit's body, by up to 1.65 ft:
- the exterior walls of type US-1 are cut back 1.33 ft short at a corner;
- partition walls run past an end Revit stops at.

The office's export writes no `IfcRelConnectsPathElements`, so Revit's joins are read from the file.

## 2. The native record

A wall's native `SWall` record (read by member name, `native_document::extract_graphs`) holds:
- the curve the wall is drawn along, a `GLine`: `m_origin`, the unit `m_dirVec` and `m_endParams`;
- `m_isFlipped` and `m_wallCrossSection`;
- `VWallDriver.m_controlJoinsSet`, one entry per join at its ends. Each entry's `m_JoinInfo.m_info` lists the walls in the join, this one included, each with the end it joins by: `m_joinedEnd` 0 its start, 1 its end, 2 or 3 its middle (a T joint).

Against the partition data (run 40203889405), on every wall that has both:
- the native curve is the partition line, to 0.000000 ft;
- `m_isFlipped` is the orientation's flip and `m_wallCrossSection` its word.

| file | walls with both | flip equal | word equal |
|---|---:|---:|---:|
| office as built | 591 | 591 | 591 |
| office at building permit | 548 | 548 | 548 |
| Core Interior | 359 | 359 | 359 |
| RE1 Architecture | 7 | 7 | 7 |

## 3. Where Revit ends a wall

`wall_curve_ends_vs_ifc` measures each end of Revit's body against the native curve. Where another wall meets this wall's end with its middle, Revit's body stops half that wall's thickness short of the curve's end. Per T end, against Revit's body to 0.001 ft (run 40202682753):

| file | T ends | that rule | the join lists' reach |
|---|---:|---:|---:|
| office as built | 396 | 315 | 289 |
| office at building permit | 380 | 271 | 265 |
| Core Interior | 269 | 269 | 269 |
| RE1 Architecture | 3 | 3 | 0 |

The ends the rule misses mostly stop further back, at a column or a layer face; 1436343, whose record names columns 1926725 and 1926759, stops 0.94 ft further back than the rule puts it.

## 4. Corners: the join order, a measured negative

At a corner where walls meet end to end, the wall the record lists first in the join runs half the second wall's thickness past its curve's end, and every later wall stops half the first wall's thickness short. On the first walls of the office's two-wall corners, among the 400 walls `wall_curve_ends_vs_ifc` first listed, that is Revit's end on 135 of 142. It is not used. As a rule for every corner it is Revit's end on fewer ends than the join lists' reach already gives (run 40202682753, office as built):
- first of two: 143 against 142;
- later of two: 86 against 134;
- later of three or more: 103 against 90.

Layered walls stop at a layer face instead.

## 5. The native line where copies disagree, a measured negative

The partition data gives no line for a wall whose data copies disagree (RE-216): 41 walls on Core Interior, 70 at building permit and 5 as built. Taking the native curve for them gives the office more walls with Revit's ends (431 against 412). But it moves 7 Core Interior walls 0.333 ft from Revit's (360 to 353 of 360, run 40204804355), the same 7 RE-216 found with the current copy's line. It is not used.

## 6. Orientation

13 of the office's 17 US-1 walls, 12 of the 24 of type 1753279 and both of 1760031 have a partition line and their type's layers but no partition orientation (run 40204859227), so they were drawn as their record box cut back at their joins. Such a wall now takes its native word and flip, so it is drawn from its centreline, takes part in the join lists (RE-70, RE-71) and is drawn in its layers.

## 7. What rvt-rs does

`native_walls::native_walls` reads each wall's curve, word, flip and end joins in one pass over the walls' records. It drops a wall whose records in several partitions disagree.

`attach_wall_layers`:
- takes the native word and flip where the partition data gives no orientation;
- after the join lists, at each end where exactly one other wall meets the end with its middle, sets the end's reach to half that wall's thickness short (`native_walls::tee_reach`). This replaces the join lists' reach there.

An end whose layers each end on their own line (RE-71, RE-74) keeps them.

## 8. Measured

Run [40206225586](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40206225586), this change against `main`:

| | office as built | | office at building permit | |
|---|---:|---:|---:|---:|
| | `main` | RE-222 | `main` | RE-222 |
| walls with both ends within 0.001 ft (`wall_bodies_vs_ifc`) | 412 of 607 | 428 | 374 of 589 | 377 |
| ends within 0.001 ft (`wall_ends_cv_vs_ifc`) | 982 of 1,212 | 1,011 | 914 | 926 |
| layer sets written, all of Revit's | 711 of 712 | 733 of 734 | 595 of 595 | 627 of 627 |
| walls drawn in layers, every layer centre within 0.001 ft | 561 of 576 | 583 of 598 | 487 of 487 | 519 of 519 |
| layer names in Revit's order | 1,505 | 1,559 | 1,168 | 1,266 |

Core Interior, Einhoven, the four RE1 models and the 23 R2025 models export byte-identical IFC.

The T rule alone (run 40206135950) gives 430 and 378 walls with both ends right. With the orientation, two walls on the as-built file and one at building permit lose it, while the per-end counts stay or rise. Those are walls now drawn in layers whose bands end where the joins put them, not where the record box did.

## 9. What it does not cover

- Corners (§4) and the 41 + 70 walls whose copies disagree (§5).
- An end that stops at a column or a layer face rather than half the other wall's thickness short.
- Curved walls: the native curve is read only where it is a `GLine`.
