# RE-276: a beam's Cross-Section Rotation is the rotation of its curve driver

**Date:** 2026-10-30
**Issues:** W7 ([release 1.0 criteria](../../docs/release-1.0-criteria.md))
**Artefacts:** the BIM4LCA office as built and at building permit (Revit 2024, CC BY-SA 4.0) with Revit's own IFC4 exports. Read on GitHub's hosted runners with the Measure workflow, run 41113408569.
**Probe:** `probes/examples/probe_w7_office_gaps.rs`.
**Status:** positive for the location; the value is measured at 0 only.

## 1. The question

Revit's IFC4 export writes `Pset_BeamCommon.Roll` on each of the office's 8 beams (`Putkipalkki - suorakaide:PA-1`), 0.0 on all. revit-ifc's `RollCalculator` (CLEANROOM.md source 5) takes it from the beam's `STRUCTURAL_BEND_DIR_ANGLE` parameter, the Cross-Section Rotation, in radians, and returns true even for 0. The file's saved parameters of a beam list no built-in angle (a built-in keeps its negative id and no name, #255). Where is the rotation?

## 2. Result

A structural framing record's `FamInstCurveDriver` object holds `m_rotation`, a radian value: 0.0 on each of the 8 beams in both files. It sits beside the driver's other instance settings (`m_flip`, `m_hJustification`, `m_vJustification`, `m_insertionZ`, `m_setbacks`). No other member of the record is a rotation: the `FamilyInstance` object holds `m_RefDir` and `m_zAxis`, the beam's reference direction and up axis, and the `InstanceInfo` its transform's origin.

## 3. What this changes

rvt-rs reads `m_rotation` of each beam's `FamInstCurveDriver` and writes `Pset_BeamCommon.Roll` 0 for a beam whose rotation is exactly 0 and which it writes an extrusion length for. A beam with a non-zero rotation gets no roll: all 8 measured beams are unrotated, so that `m_rotation` carries the parameter's value for a rotated beam is not measured. A model with rotated beams and Revit's export would decide it.

## 4. Reproduce

```text
gh workflow run measure.yml --ref main -f ref=8193128ea56b7fb098c1472eacb0536b9fe09c32 -f base=none -f nordic=true -f models="nordic-office-*" -f scorers=none -f probe=probe_w7_office_gaps -f probe_scorer="-"
```
