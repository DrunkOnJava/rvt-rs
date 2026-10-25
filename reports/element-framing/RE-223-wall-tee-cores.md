# RE-223: At a T joint a layered wall's core runs through the other wall's layers outside its core

**Date:** 2026-10-25
**Issues:** W6. Follows RE-73 (a wall stops layer by layer at a T joint, by layer priority), RE-222 (a wall's T joints from its native record) and RE-53 (a wall type's layers).
**Artefacts:** the BIM4LCA office as built and at building permit (Revit 2024) and Revit's IFC4 exports of both, as RE-222 lists them, with Core Interior and the four RE1 models.
**Probes:** `examples/probe_wall_type_core.rs` (a wall type's native record, object by object), `examples/probe_wall_curves.rs` (now also each wall type's layers and core). **Scorer:** `tools/re/wall_curve_ends_vs_ifc.py` (the T rule with the stop at the other wall's core).
**Runs:**
- [40212216582](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40212216582): three wall types' native records;
- [40213758793](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40213758793): the stop at the other wall's core against Revit's ends, on every wall;
- [40217761238](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40217761238): that stop for every T end, a measured negative (§4);
- [40219887062](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40219887062): this change against `main` (59e929f9).

**Status:** positive.

## 1. What was left

After RE-222, 81 of the office's 374 T ends (location-line key 0, as built) still ended away from Revit's. 59 of them were walls of type 460609 (layers 0.0427, 0.2428 and 0.0427 ft) whose body Revit runs 0.0426 ft further than half the other wall's thickness short. Six were walls of type 3347182 (0.0853, 0.2165, 0.0853 ft), which Revit runs 0.0853 ft further. Each offset is one outer layer of the wall they meet.

## 2. The core boundary

The partition data gives a wall type's layers' widths, materials and functions (RE-53), not its core. On the office every layer of these partition types is marked structure (function 1), so RE-73's priority rule stops every layer at the other wall's face.

The type's native `BasicWallType` record holds a `CompoundStructure` (run 40212216582):
- `m_layers`, exterior first, each with `m_layerWidth` and `m_layerFunction`;
- `m_numShellLayersExt` and `m_numShellLayersInt`, how many of them lie outside the core on each side.

| type | layers, ft | outside the core, exterior / interior |
|---|---|---|
| 460609 | 0.0427, 0.2428, 0.0427 | 1 / 1 |
| 3347182 | 0.0853, 0.2165, 0.0853 | 0 / 0 |
| 1495207 (US-1) | 0.0984, 0.1640, 0.0984, 0.5741, 0.4921 | 2 / 0 |

## 3. Where Revit ends the wall

Revit's wall stops at the other wall's core: half its thickness short, less its layers outside the core on the side the wall comes from. Against Revit's bodies, applied to every T end (run 40213758793):

| file | T ends (key 0) | that stop | rvt-rs after RE-222 |
|---|---:|---:|---:|
| office as built | 374 | 351 | 295 |
| office at building permit | 360 | 305 | 249 |
| Core Interior | 137 | 137 | 137 |

The ends that stop is wrong on, where rvt-rs was right, are single-layer walls. Revit stops those at the other wall's face, as RE-73 found on Snowdon Towers.

## 4. Every T end, a measured negative

Applied to every T end in place of the face (run 40217761238), the stop costs walls on the office: 428 to 421 walls with both ends right as built, 377 to 375 at building permit. The ends it gains are layered walls, which RE-73 draws layer by layer and the override did not reach. The ends it loses are single-layer walls.

## 5. What rvt-rs does

`native_walls::native_walls` reads each wall type's `CompoundStructure` in the same pass as the walls: every `BasicWallType` record by class. It gives each wall the width of its type's layers outside the core on its exterior and its interior side (`NativeWall::shells`). `attach_wall_butt_joins` hands them on as `WallLine::shells`.

RE-73's `tee_join` keeps its priority rule, with one addition. A layer inside the joining wall's core reaches at least the other wall's core: half its thickness short of the curve's end, less its layers outside the core on the near side. A single-layer wall, and any layer outside the core, end as before.

## 6. Measured

Run [40219887062](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40219887062), this change against `main`:

| | office as built | | office at building permit | |
|---|---:|---:|---:|---:|
| | `main` | RE-223 | `main` | RE-223 |
| walls with both ends within 0.001 ft (`wall_bodies_vs_ifc`) | 428 of 607 | 482 | 377 of 589 | 422 |
| ends within 0.001 ft (`wall_ends_cv_vs_ifc`) | 1,011 | 1,073 | 926 | 978 |

Layer sets and layer centres are unchanged, and no other scorer moves. Core Interior, Einhoven, the four RE1 models and the 23 R2025 models export byte-identical IFC. Snowdon Towers, on which RE-73 was measured, is local only and was not measured again.

## 7. What it does not cover

- Corners, and walls whose data copies disagree (RE-222 §4, §5).
- T ends that stop at a column rather than the other wall's core (1436343: 0.94 ft further back).
- A type with no native `CompoundStructure`: its walls end by RE-73's priority rule alone.
