# RE-166: a curtain mullion's or panel's type holds its material at a fixed offset

**Backlog:** B66. **Status:** positive on Revit 2024 to 2026. **Shipped in:** #657 (2026-10-04).

**Written:** 2026-10-23, from the record of #657, because the code cited RE-166 without a report. Nothing was re-measured for this report.

**Evidence:** red run 37215541120 of the real-file target `tests/curtain_part_materials.rs` on draft #655 (12 of 12 curtain parts without Revit's material; the output is in the test commit of #657) and green run 37216785023 in Corpus health, as `BACKLOG.md` B66 records. No probe: the target is the check.

**Files:** RE1 Architecture (`ifc-ecs-re1-architecture.rvt`, sha256 `4a78bdd6…`) against Revit's IFC2X3 export of it (`ifc-ecs-re1-architecture.ifc`, `a9b5d366…`), and the `SysMullionFamSym` objects of the 2024 to 2026 sample projects. Full hashes are in `research/witness-registry.json`.

## 1. The offsets

- A mullion type's data object (`SysMullionFamSym`) holds its material's ElementId at byte 135, on every such object of the 2024 to 2026 samples and RE1.
- RE1's panel type (`SysPanelFamSym`) holds its material at byte 139.

## 2. Measured

With those materials, every `IfcMember` and `IfcPlate` of RE1 Architecture has the material set Revit's export gives it: 12 of 12, where `main` before #657 had none. A curtain part that already had a type material keeps it.

## 3. Not established

- The panel offset rests on RE1's panel types only.
- Releases before 2024 are not read.
