# RE-361 — Doors with no opening: their origin and their axes

**Date:** 2026-11-05
**Issues:** #1494; follows RE-357 (the origin of a walled door, #1465), RE-347 (the axes of a walled door, #1447), RE-343 (the flips) and RE-87 (instance transforms)
**Probe:** `probes/examples/probe_re343_door_flips.rs` (now also reads each element's stored host, `stored_host`, and that host's native wall record)
**Scorer:** `tools/re/door_unwalled_vs_ifc.py`
**Status:** in progress. The origin rule is written (section 2) and the axes are being measured (section 3); no figure here is measured on a Measure head yet.

## 0. The question

#1494: 49 of the doors rvt-rs binds to no plain wall have Revit axes the reverse of RE-343's Rule 1, uniformly by `IfcDoorType`, and where Revit's export gives a door no opening its origin is the stored origin (RE-357: 153 of 153 in Measure run 42127619521). Can rvt-rs tell those doors from the file, and is there a rule with 0 wrong for their origin and for their axes?

## 1. What was measured earlier

From the kept probe output and exports of Measure run 42127619521 (RE-357), counted again for this change: in Core Interior, RE1 Architecture and the BIM4LCA office as built and at permit, the doors rvt-rs writes no opening for are the doors Revit's export gives none, with no door in one set and not the other (153 in both, 0 and 0 in the two directions), and the stored origin is Revit's on all 153.

## 2. The origin

A door rvt-rs binds to no host wall (no `OPENING_HOST_FIELD`, after `attach_cut_loop_sizes` has made the host final), whose stored transform is read and upright and whose type draws geometry, is placed at the stored origin: `location_feet` is `InstanceTransform.origin`, the body stays the record box where it is in the world (the box's rectangle as a placed extrusion, in the element's own frame, so a saved mesh and the family's extrusions, taken in that frame afterwards, follow), and `RvtElementRecordGeometry.PlacementOriginSource` is `stored_origin_no_opening`. The pass is last, after the storeys are bound by the box's base and the openings are cut. Nothing else changes: no GlobalId, host or opening.

## 3. The axes

Under measurement.

## 4. Not claimed, not measured

Everything above awaits the run of this change.
