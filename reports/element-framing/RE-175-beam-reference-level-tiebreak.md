# RE-175 — A beam's Reference Level when its record lists two Levels

**Date:** 2026-10-13
**Follows:** RE-174.
**Artefacts:** Autodesk's public Revit 2024 samples and the BIM4LCA Nordic models (CC BY-SA 4.0), each with Revit's own IFC export as the oracle.
**Clean-room provenance:** public sample files and Revit's own IFC export only. No SDK source and no decompiled or disassembled code.
**Result:**
- RE-174 reads a beam's or brace's Level from the element record's id lists and writes nothing when the chosen list names more than one recovered Level. Where it does, the other Level(s) are below the Reference Level: **the Reference Level is the highest of them** by the recovered Level elevation. Written only when every named Level has a recovered elevation and the highest is unique, and only as an `IfcBeam` / `IfcMember` `Reference Level`. Other classes keep RE-174's single-Level rule: applied to them, the highest Level was the wrong `Base Level` for 21 stairs whose association record is not decoded (Technicalschool 5, rmeadvanced 8, rmebasic 8; their list names Base and Top Level, and Revit's `Base Level` is the lower), so it is not used.
- **Additive.** Every other property is unchanged (survey below, all files).

Measured with `level_parameters_survey_are_revits` (every model in `RVT_REVIT2024_SAMPLES_DIR` with its export), `Reference Level` written / equal to Revit / Revit has it:

| file | before (RE-174) | after | lowest-Level control |
|---|---:|---:|---:|
| racadvanced | 35 / 35 / 38 | 37 / 37 / 38 | 2 wrong |
| Technicalschool | 35 / 35 / 38 | 37 / 37 / 38 | 2 wrong |
| rstadvanced | 395 / 395 / 399 | 397 / 397 / 399 | 2 wrong |
| rstbasic | 339 / 339 / 370 | 370 / 370 / 370 | 31 wrong |

Zero wrong values in any property on any file. Not written: racadvanced 1 and racbasic 1 (`IfcRoof`), rstadvanced 2 (`IfcElementAssembly` trusses) — no beams are left without a Reference Level on these four files.

The BIM4LCA exports carry no `Constraints` set, so Revit's storey containment is the reference there: Office_Timber As-Built and BuildingPermit go from 36 to 83 `Reference Level` each, 83 of 83 equal to Revit's storey (lowest-Level control: 36 of 83). The other Nordic files gain nothing.

**Limits.** Revit 2024 only; elevations are the recovered Levels' (RE-118 path). DACH and Snowdon export no elements here. Those 21 stairs' `Base Level` (the lower listed Level in 21 of 21) is not written; the stair rule would need its own check.
