# RE-286: what Revit's LoadBearing, a slab's IsExternal and ThermalTransmittance are made of, and why none is admitted to Revit 2015 to 2022

**Date:** 2026-10-30
**Issues:** #1158; follows RE-283 (the parameters named for a property), RE-285 (beams' and columns' LoadBearing) and RE-185 (a wall type's Function)
**Artefacts:** BIMcollab's Revit 2015 structural example (`02_BIMcollab_Example_STR.rvt`, sha256 `8cd1f122…e193`, with Revit 2015's own export; no licence stated, so only counts left the runner), and the held-out models (`research/holdout-models.tsv`: the flowbim.ee house, saved in Revit 2026 and exported by Revit 26.4; modelo_bim, saved in 2023 and exported by Revit 26.2; teste_export_2025), the BIM4LCA office, RE1 and puzzbobb's R2025 models.
**Runs:** Measure [41134253852](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41134253852) (every field of the wall, floor and type records against Revit's flags), [41135131460](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41135131460) (the same, with the fields a scan found printed on every class), [41136244246](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41136244246) (a wall's structural flag against its type Function, jointly), [41136498155](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41136498155) (the floor's parameters) and [41137406596](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41137406596) (the parameters named for a flag).
**Probes and scorers:** `probes/examples/probe_re285_flag_candidates.rs` and `tools/re/native_flags_vs_ifc.py` (the saved parameters), `probes/examples/probe_re286_native_fields.rs` and `tools/re/native_fields_vs_ifc.py` (the scalar fields of the native records).
**Status:** mixed, and nothing here is admitted to 2015 to 2022. The rules are found for Revit's newer exporters and each is exact on the held-out models that exercise it; the one export of a 2015 to 2022 file, BIMcollab's, was made by Revit 2015's exporter, which writes `LoadBearing` and `IsExternal` for these classes from other data, so it cannot confirm them. `ThermalTransmittance` is not a field of any record. `Span` and `Slope` of beams need the beam's axis, which only the Revit 2024 partition layout reads.

## 1. What BIMcollab's export writes, and from what

BIMcollab's export writes `LoadBearing` and `IsExternal` on these elements (Revit's counts, from the property census; the 16 footings and 3 air terminals carry neither):

| class | `LoadBearing` | `IsExternal` | `ThermalTransmittance` |
|---|---|---|---|
| IfcWallStandardCase (114) | true on 114 | true on 71, false on 43 | 66 values, 3 distinct |
| IfcSlab (5) | true on 5 | false on 5 | 5 values, 2 distinct |
| IfcBeam (91) | true on 91 | false on 91 | none |
| IfcColumn (15) | true on 15 | false on 15 | none |

- **The walls' values are two Yes/No parameters on the walls.** All 114 walls hold an instance parameter named `LoadBearing` (true on 114) and an instance parameter named `IsExternal` (true on 71, false on 43): exactly Revit's values, wall by wall in count (41137406596). Revit's exporter reads a parameter named for the property before anything else (RE-193), so no other data decides these walls, and the rules below (a wall type's Function, the Structural box) cannot be tested on them. RE-283 reads the parameter.
- **The beams' and columns' values are the category's** (revit-ifc's `LoadBearingCalculator`, CLEANROOM.md source 5: structural columns and structural framing are load bearing before any parameter is read; the last step of `IsElementExternal` is not external): 106 of 106 `LoadBearing` and 106 of 106 `IsExternal` equal (RE-285).
- **The slabs' are not what a newer exporter would write.** `FLOOR_PARAM_IS_STRUCTURAL`, the saved instance parameter `-1001954`, is 0 on all 5, and Revit's 2015 export writes `LoadBearing` true on all 5. Their types hold no `FUNCTION_PARAM` (`-1001006`) in the saved parameters, so a slab's Function is not read on Revit 2015, and its export writes `IsExternal` false.

## 2. The rules for the newer exporters

| flag | where it is stored | measured against |
|---|---|---|
| a wall's `LoadBearing` | `SWall.m_isStructuralSignificant`, the Structural box (`m_wallStructuralUsage` is 1 on every wall of every model) | the flowbim house, 59 of 59 walls with both values (35 true, 24 false); modelo_bim 4 of 4 false; teste_export_2025 3 of 3 false; RE1 Architecture 7 of 7 false; `01_Walls` of the R2025 models 5 of 5 false. The office has its own parameters named `LoadBearing`, which decide |
| a slab's `LoadBearing` | the saved instance parameter `-1001954` (`FLOOR_PARAM_IS_STRUCTURAL`) | the flowbim house, 8 of 8 slabs (2 true, 6 false); modelo_bim 1 of 1 true; teste_export_2025 1 of 1 false |
| a slab's `IsExternal` | the saved type parameter `-1001006` (`FUNCTION_PARAM`), 0 interior | the flowbim house 8 of 8 false; teste_export_2025 1 of 1 false; modelo_bim 1 of 6 slabs hold the parameter, 0 and false |
| a wall's `IsExternal` | its type's `m_function`, not interior (RE-185, RE-283) | modelo_bim 4 of 4, the flowbim house 59 of 59 true |

BIMcollab does not confirm any of the first three. In its export the Structural box of the walls does not decide `LoadBearing` (the one wall whose box is 0 has `LoadBearing` true, and every wall holds the parameter), and the slabs' parameter is 0 where Revit 2015 writes true. So a rule from the Structural box or the slab parameter would be exact on every export of Revit 2023 to 2026 measured and wrong on the only one of Revit 2015: it is not written on any release until a Revit 2015 to 2022 export that agrees exists, or the maintainer decides that the newer exporter's rule is the target.

## 3. What is not a field

- **`ThermalTransmittance`.** On BIMcollab 66 walls and 5 slabs have a value (3 and 2 distinct), on the flowbim house 59 walls (11 distinct) and 8 slabs (4). No scalar field of a wall, floor or type record, and no saved double parameter, is a constant multiple of Revit's value on every element that has one. Revit computes it from the layers' thermal properties, which the records hold in the materials. Only a parameter named for it (RE-194, and RE-283 on 2015 to 2023) gives a value.
- **Beams' `Span` and `Slope`** (91 each on BIMcollab). They are read from the extrusion length of the beam's axis (RE-49), which `partition_beam_axes` reads on the Revit 2024 layout only (`BEAM_AXIS_SUPPORTED_REVIT_VERSIONS`). The 32-bit layout of 2015 to 2023 has no measured reader for a beam's axis.

## 4. What this does not claim

- That the newer exporter's rules are wrong for a 2015 file: they are what Revit 26.x writes for a file it opens, and no export of such a file is held here.
- That a slab's or wall's flag could not be read: the fields and parameters above are on all 2015 to 2027 records read, and the scorers print their values on any model with an export.

## 5. Reproduce

```text
gh workflow run measure.yml --ref <branch> -f ref=<commit> -f base=none -f nordic=true -f r2025=true -f bimcollab=true -f holdout=true -f scorers=none \
  -f probe="probe_re285_flag_candidates probe_re286_native_fields" -f probe_scorer="native_flags_vs_ifc native_fields_vs_ifc"
gh run download <run id>   # measure-head/<model>/probe_score*.txt, counts.txt (BIMcollab and the held-out models: counts only)
```
