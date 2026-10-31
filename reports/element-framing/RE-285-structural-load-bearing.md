# RE-285: structural beams' and columns' LoadBearing, and their IsExternal on Revit 2015 to 2023

**Date:** 2026-10-30
**Issues:** #1158; follows RE-193 (`IsExternal`), RE-194 (parameters named for a property), RE-283 (the same on Revit 2015 to 2023) and RE-281 (BIMcollab's missing values)
**Artefacts:** BIMcollab's Revit 2015 structural example (`02_BIMcollab_Example_STR.rvt`, sha256 `8cd1f122…e193`, with Revit 2015's own IFC2X3 export; no licence stated, so only counts left the runner); modelo_bim (saved in Revit 2023, with the export Revit 26.2 made; counts only); Autodesk's sample projects of 2015 to 2027 (fetched by hash, never stored); the BIM4LCA office, the R2025, reference and held-out models of the unfiltered run.
**Runs:** Measure [41138533136](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41138533136) (the unfiltered run, `-f nordic=true -f r2025=true -f samples=true -f bimcollab=true -f holdout=true`, this change at `a5b7dd8b` against RE-283's `9a133e1f`, with `probe_re285_flag_candidates` and its scorer) and [41142127054](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41142127054) (the as-built office with `-f keep_ifc=true`, both sides diffed).
**Probe and scorer:** `probes/examples/probe_re285_flag_candidates.rs`, `tools/re/native_flags_vs_ifc.py`.
**Status:** positive. `LoadBearing` of a structural column or structural framing element is true, and its `IsExternal` is false on Revit 2015 to 2023, as revit-ifc's `LoadBearingCalculator` and the last step of `CategoryUtil.IsElementExternal` give them (CLEANROOM.md source 5, read and cited). Against Revit's exports: BIMcollab (Revit 2015's exporter) 91 of 91 beams and 15 of 15 columns for each property, none different; modelo_bim (a Revit 2023 file, Revit 26.2's exporter) 8 of 8 beams and 8 of 8 columns for `IsExternal` and 8 of 9 for `LoadBearing` of each, none different, the ninth beam's and ninth column's missing. `LoadBearing` is written on Revit 2024 to 2027 too; the offices give it by parameters they define and it is unchanged where a parameter says.

## 1. What changed

- `LOAD_BEARING_FIELD` is set on every `StructuralColumn` and `StructuralFraming` element (the categories `OST_StructuralColumns` and `OST_StructuralFraming`) on every release, and `add_load_bearing` writes it as `LoadBearing` in the common property set of the entity the element is exported as (`Pset_BeamCommon`, `Pset_ColumnCommon`, `Pset_MemberCommon`) where IFC4's set defines the property. A parameter named for it replaces it (`add_named_parameter_sets`), as every named parameter does.
- `IS_EXTERNAL_LAST_STEP_RECORD_CLASSES` names the same two classes: on Revit 2015 to 2023 `attach_record_is_external` gives them the calculation's last step, not external, where no parameter or type Function says otherwise. Any other class not read by a parameter gets none on these releases, where on 2024 and 2025 the last step reaches every class that is not a roof or a Function class. The framing systems and trusses `LoadBearingCalculator` also names are not read.

## 2. What Revit's exports write

`native_flags_vs_ifc.py` counts Revit's `LoadBearing` and `IsExternal` by IFC class (counts only for BIMcollab and modelo_bim):

| export | class | `LoadBearing` | `IsExternal` |
|---|---|---|---|
| BIMcollab (Revit 2015) | IfcBeam, 91 | true on 91 | false on 91 |
| BIMcollab | IfcColumn, 15 | true on 15 | false on 15 |

The property census (`psets_vs_ifc.py`), before this change (RE-283's base) and after:

| property | BIMcollab before | BIMcollab after | modelo_bim before | modelo_bim after |
|---|---|---|---|---|
| `Pset_BeamCommon.IsExternal` | 91 missing | 91 equal | 8 missing | 8 equal |
| `Pset_BeamCommon.LoadBearing` | 91 missing | 91 equal | 9 missing | 8 equal, 1 missing |
| `Pset_ColumnCommon.IsExternal` | 15 missing | 15 equal | 8 missing | 8 equal |
| `Pset_ColumnCommon.LoadBearing` | 15 missing | 15 equal | 9 missing | 8 equal, 1 missing |

No value is different. The ninth beam and ninth column of modelo_bim with a `LoadBearing` are not written: they are not elements of these two classes, or not written at all, and the census does not say which.

On the BIM4LCA office, whose structural columns and beams carry parameters named `LoadBearing`, the as-built model's `Pset_ColumnCommon.LoadBearing` goes from 278 equal and 11 missing to 279 equal and 10 missing: one column that holds no such parameter gets the category's value Revit writes. The permit model's census is unchanged. Nothing is different on either. The diff of the as-built export (41142127054) is that column's one property and a different order of the `LoadBearing` property within the sets of columns whose parameter replaces it.

## 3. What moved

58 of the 94 exports of the unfiltered run moved against RE-283's: BIMcollab, modelo_bim, the two offices, all 33 samples of 2015 to 2023 and 21 of the 25 samples of 2024 to 2027 (each gains `LoadBearing` on its structural columns and framing, and the samples of 2015 to 2023 `IsExternal` too). The 36 that did not are the four Snowdon Towers samples (architectural, electrical, HVAC, plumbing), Core Interior, Einhoven, the flowbim house, teste_export_2025, RE1's four models and the 24 R2025 models, which have no structural column or framing. No Revit export of a sample exists, so the samples' new values are the rule's, measured on the exports above.

## 4. What this does not claim

- **Walls', slabs' and other categories' `LoadBearing`.** BIMcollab's walls carry parameters named for it (RE-283), its slabs are `true` where the parameter that decides on a newer exporter is 0, and the architectural `OST_Columns` category has no rule here, not measured here.
- **`IsExternal` of any other class on Revit 2015 to 2023.** A slab's, door's or window's needs its type's Function,, not read on these releases.
- **Beams' `Span` and `Slope`**, 91 each on BIMcollab, need the beam's axis, which only the 2024 layout reads.
- That an exporter of another release writes the same: BIMcollab's is Revit 2015's, modelo_bim's Revit 26.2's.

## 4a. Measured again on `main` (2026-10-31)

Measure [41206921249](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41206921249), this change rebuilt on `main` (`888c043a` against `5d863fba`, unfiltered with the samples, BIMcollab and the held-out models). BIMcollab gains 212 equal values (`IsExternal` and `LoadBearing` on 91 beams and 15 columns). modelo_bim gains 32: 8 of 8 beams' and columns' `IsExternal`, and `LoadBearing` on all 9 of each, where the ninth is on the type objects RE-286 writes. The office as built gains its one column. Nothing that differed before differs, and "rvt-rs values Revit does not write" is unchanged on every model. 58 exports move, as in §3.

## 5. Reproduce

```text
gh workflow run measure.yml --ref main -f ref=a5b7dd8bfc909065726e3463a5732f5ba68219e1 -f base=9a133e1f59d93a6183b62906507171cb3e01bb57 \
  -f nordic=true -f r2025=true -f samples=true -f bimcollab=true -f holdout=true \
  -f probe=probe_re285_flag_candidates -f probe_scorer=native_flags_vs_ifc
gh workflow run measure.yml --ref main -f ref=a5b7dd8bfc909065726e3463a5732f5ba68219e1 -f base=9a133e1f59d93a6183b62906507171cb3e01bb57 \
  -f nordic=true -f models=nordic-office-as-built -f scorers=none -f keep_ifc=true
gh run download <run id>   # measure-head/<model>/psets_vs_ifc.txt or counts.txt; model.ifc
```
