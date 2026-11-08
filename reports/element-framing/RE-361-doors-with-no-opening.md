# RE-361 — Doors with no opening: their origin and their axes

**Date:** 2026-11-05
**Issues:** #1494; follows RE-357 (the origin of a walled door, #1465), RE-347 (the axes of a walled door, #1447), RE-343 (the flips) and RE-87 (instance transforms)
**Probe:** `probes/examples/probe_re343_door_flips.rs` (now also reads each element's stored host, `stored_host`, and that host's native wall record)
**Scorer:** `tools/re/door_unwalled_vs_ifc.py`
**Status:** origin positive, with 0 wrong on the 3 models that hold such doors; axes negative, no rule with 0 wrong was found and none is written.

## 0. The question

#1494: 49 of the doors rvt-rs binds to no plain wall have Revit axes the reverse of RE-343's Rule 1, uniformly by `IfcDoorType`, and where Revit's export gives a door no opening its origin is the stored origin (RE-357: 153 of 153 in Measure run 42127619521). Can rvt-rs tell those doors from the file, and is there a rule with 0 wrong for their origin and for their axes?

## 1. What was measured earlier

From the kept probe output and exports of Measure run 42127619521 (RE-357), counted again for this change: in Core Interior, RE1 Architecture and the BIM4LCA office as built and at permit, the doors rvt-rs writes no opening for are the doors Revit's export gives none, with no door in one set and not the other (153 in both, 0 and 0 in the two directions), and the stored origin is Revit's on all 153.

## 2. The origin

A door rvt-rs binds to no host wall (no `OPENING_HOST_FIELD`, after `attach_cut_loop_sizes` has made the host final), whose stored transform is read and upright and whose type draws geometry, is placed at the stored origin: `location_feet` is `InstanceTransform.origin`, the body stays the record box where it is in the world (the box's rectangle as a placed extrusion, in the element's own frame, so a saved mesh and the family's extrusions, taken in that frame afterwards, follow), and `RvtElementRecordGeometry.PlacementOriginSource` is `stored_origin_no_opening`. The pass is last, after the storeys are bound by the box's base and the openings are cut. Nothing else changes: no GlobalId, host or opening.

## 3. The axes: a measured negative

Measure run 42523791605 (head `b01dfc90`, `-f probe=probe_re343_door_flips -f probe_scorer=door_unwalled_vs_ifc -f keep_ifc=true`, with the office, the R2025 models and the held-out models). Three models hold doors with no opening: the office as built (75) and at permit (77) and RE1 Architecture (1), 153 in all. No rule read from the file gives their axes with 0 wrong:

- Revit's X is the stored X reversed where the two flips differ (RE-343's Rule 1): wrong on 45 of 153 (21, 23 and 1).
- Along the wall, on the 137 doors with a stored host the probe read (67, 69 and 1), Revit's X against the wall's direction: reversed where `m_flippedY` is set agrees on 96, where `m_flippedX` is set on 75, where the flips differ on 67, never reversed on 60.
- Reversal is uniform by `IfcDoorType` (no type entity holds both a door whose flips differ and one whose do not, on 28 entities as built and 31 at permit), as #1494 found, but no byte read here tells which type entity is reversed.

So the change writes no axes for these doors. They stay as RE-343's rule gives them, and #1494's axes remain open.

## 4. Measured

**The origin**, same run, each model counted once:

| model | doors checked | opening in both | in neither | rvt-rs only | Revit only | origin rule written | further than 0.001 ft from Revit's | written where Revit gives an opening |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| office as built | 269 | 194 | 75 | 0 | 0 | 74 | 0 | 0 |
| office at permit | 269 | 192 | 77 | 0 | 0 | 76 | 0 | 0 |
| RE1 Architecture | 5 | 4 | 1 | 0 | 0 | 1 | 0 | 0 |

Across every model of the run, 681 doors are checked, with 0 in rvt-rs only and 0 in Revit only, and all 153 doors with no opening stand within 0.001 ft of Revit's origin. No window lacks an opening or carries the marker. One door on each office file has no `PlacementOriginSource`: `apply_placement_origins` leaves a door with a shape of its own, a profile override or a representation map. On the file read, it is a curtain `System Panel`, whose origin is already within 1e-6 ft of the stored one.

**The whole export**, Measure run 42520016421 (head `b01dfc90`, main's side `ab5f8e1c`, unfiltered with samples, held-out models, `racadv` and BIMcollab): 98 of 118 exports are byte for byte main's and 20 move, the office files, RE1 Architecture and the 2024 to 2027 samples that hold such doors. `same-project gates` passes. The only scorer output that moves is one door of RE1 Architecture, whose saved mesh agrees with Revit's more closely (2.95e-06 to 1.993e-06 ft), and the sign of a zero. The advisory semantic check passes.

## 5. Not claimed, not measured

- Core Interior holds none of these doors (132 of 132 have an opening in both), and neither do the R2025 models, so section 1's list of models overstates what exercises the rule: the 3 above do.
- The rule on any release but 2024 and 2025 against a Revit export. The 2026 and 2027 samples move and have no export to score them.
- The doors' axes (section 3).
