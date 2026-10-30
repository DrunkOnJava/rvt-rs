# RE-165: a pipe fitting's connector records carry its nominal diameter

**Issue:** #35. **Backlog:** B44. **Status:** positive. **Shipped in:** #651 (2026-10-04), which wrote the value in `Pset_PipeFittingTypeCommon`. #822 moved it to rvt-rs's own `RvtElementRecordGeometry` set, because IFC4's `Pset_PipeFittingTypeCommon` has no `NominalDiameter`.

**Written:** 2026-10-23, from the record of #651, because the code cited RE-165 without a report. Nothing was re-measured for this report.

**Evidence:** Measure runs 37211684944 and 37212205913; red run 37212982834 of the real-file target `tests/fitting_nominal_diameter.rs` on draft #649 (42 of 42 RE1 Plumbing fittings without Revit's value); the two readers that failed on the way (run 37214338189, connector records alone, 42 of 42 missing; run 37215748458, parameter pairs alone); green run 37217026749 (42 of 42), as `docs/archive/BACKLOG.md` B44 records. No probe: the target is the check.

**Files:** RE1 Plumbing (`ifc-ecs-re1-plumbing.rvt`, sha256 `03689df2…`) against Revit's IFC2X3 export of it (`ifc-ecs-re1-plumbing.ifc`, `294490b2…`). Full hashes are in `research/witness-registry.json`.

## 1. The record

A pipe fitting's own data object (RE-153) holds one record per connector: a unit direction, an origin, the radius, a further length, and the diameter, twice the radius.

## 2. Measured

Revit's export writes a `NominalDiameter` on all 42 of RE1 Plumbing's fittings (elbows, tees, a cap, a vent tee; 10 to 50 mm). The connector records' diameters give it, and on a fitting with no connector record (a cap) its smallest nominal parameter pair does. A fitting whose connectors disagree (a reducer) is given no value rather than a guess.

Revit's IFC2X3 exports of the RE1 models write the value as an `IfcPropertyListValue` of one length, and #651 wrote it the same way (`PropertyValue::List`). Since #822 rvt-rs writes it as one length in `RvtElementRecordGeometry`.

## 3. Not established

- Reducers: which connector's diameter Revit writes for a fitting whose connectors differ is not measured, so none is written.
- Releases before 2024: the data objects are read on Revit 2024 and later only.
