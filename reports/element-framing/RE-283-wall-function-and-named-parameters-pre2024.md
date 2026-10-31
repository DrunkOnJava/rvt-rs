# RE-283: a wall's IsExternal from its type's Function, and the parameters named for a property, on Revit 2015 to 2023

**Date:** 2026-10-30
**Issues:** #1158; follows RE-185 and RE-193 (a wall type's Function and `IsExternal` on 2024 and 2025), RE-194 and RE-227 (the parameters named for a property), RE-281 (saved parameters on 2015 to 2022) and RE-282 (an ElementId in a 4-byte `Identifier`)
**Artefacts:** Autodesk's sample projects of 2015 to 2027 (`research/autodesk-sample-projects.tsv`, fetched by hash and never stored); BIMcollab's Revit 2015 structural example (`02_BIMcollab_Example_STR.rvt`, sha256 `8cd1f122…e193`, with Revit 2015's own IFC2X3 export; no licence stated, so only counts left the runner); modelo_bim (saved in Revit 2023, with the export Revit 26.2 made; counts only); the reference, BIM4LCA, R2025 and held-out models of the unfiltered run.
**Runs:** Measure [41138408822](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41138408822) (the unfiltered run, `-f base=main -f nordic=true -f r2025=true -f samples=true -f bimcollab=true -f holdout=true`, this change at `9a133e1f` against `main` at `3811a654`, with the 2015-to-2023 pairs), [41138447208](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41138447208) (the 2023 samples against their 2024 copies), [41130869879](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41130869879) (the first head, `420b6886`, which reads the Function alone) and [41146044699](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41146044699) (two 2015 samples with their exports kept, diffed).
**Probe:** `probes/examples/probe_re283_wall_function_pairs.rs`.
**Status:** positive, experimental on 2015 to 2022. A wall's type Function (`BasicWallType.m_function`) reads on Revit 2015 to 2023 as it does on 2024 and 2025, and gives the wall's `IsExternal` as it does there (RE-193); a project or shared parameter named `IsExternal`, `<set>.IsExternal` or for a property of one of IFC4's standard sets is read from the saved parameters (RE-281) as the partition data gives it on 2024 and later. Against the same projects saved by a later release the Function is equal on 1,982 of 1,983 walls (24 pairs of 2015 to 2022 against 2023); the one that differs is an edit between the saves. Against Revit's own exports: 114 of 114 of BIMcollab's walls (Revit 2015) and 4 of 4 of modelo_bim's (Revit 2023 file), none different, where Revit's value was missing in full; BIMcollab's `LoadBearing` on its 114 walls, which a parameter named for it gives, is equal on 114.

## 1. What changed

- `native_wall_types::wall_type_functions` names `NATIVE_RELEASES` and reads the wall's type id through `native_json::identifier_i64`, so it finds it in the 4-byte `Identifier` of 2015 to 2023. It reads no other ElementId; the other readers keep reading `/m_id/m_id64` and stay refused on 2015 to 2022 (RE-282).
- `WALL_TYPE_FUNCTION_REVIT_VERSIONS` is 2015 to 2025 (it was 2024 and 2025).
- `attach_native_named_parameters` (`partition_schema_mvp/parameters.rs`), on the releases in `NATIVE_NAMED_PARAMETER_REVIT_VERSIONS` (2015 to 2023), gives each record-backed element, from `native_element_parameters::read`:
  - the Yes/No parameters named `IsExternal` or `<set>.IsExternal` (`NAMED_PARAMETER_FIELD_PREFIX`; for the plain name also the type's, `TYPE_NAMED_PARAMETER_FIELD_PREFIX`), and
  - the parameters named `<set>.<property>`, or the property alone for each standard set that defines it, for a property of one of IFC4's sets, a qualified name before a bare one, `REVIT_OMITTED_PARAMETERS` left out, each as the field 2024's `attach_named_parameters` gives it. Instance parameters only, as there.

  It reads every parameter that has a name, whatever its kind (family, shared, project): revit-ifc looks a parameter up by name on the element, so a family parameter named for a property is read there too (CLEANROOM.md source 5, read and cited).
- `attach_record_is_external` gives a record-backed wall, or any element a parameter named for `IsExternal` was read for, the value `calculated_is_external` finds: the parameter, then the type's Function. No element gets the calculation's last step, not external, on these releases. A window, door or slab with no parameter gets none: their Function is a saved type parameter (`FUNCTION_PARAM`, RE-158) that is not read on these releases.
- `probe_re283_wall_function_pairs` and `tools/ci/scrub-holdout-results.sh` (which keeps the probe's count lines for a model without a licence).

## 2. Against the same project in a later release

`probe_re283_wall_function_pairs` reads the type Function of every wall of a sample and of its copy.

| older release | pairs | walls read in both | equal | different |
|---|---:|---:|---:|---:|
| 2015 | 3 | 240 | 239 | 1 |
| 2016 to 2022 | 21 | 249 each | 249 each | 0 |
| 2024 to 2027 (against the 2023 copy) | 12 | 249 each | 249 each | 0 |
| 2023 (against the 2024 copy) | 3 | 249 | 249 | 0 |

The one difference is a 2015 `rac_basic` wall whose type has Function 0 (interior) in 2015 and 1 (exterior) in the copy: an edit between the saves, as RE-281's parameters differ in 243 of 932,888. Both values occur: the `rac_advanced` has 122 walls of Function 0 and 58 of Function 1, and BIMcollab's 138 walls 48 and 90. The 2015 `rst_basic` is another file under the same name (RE-280 §7); its 0 shared walls are not counted.

## 3. Against Revit's exports

| model | property | before | after |
|---|---|---|---|
| BIMcollab (Revit 2015) | `Pset_WallCommon.IsExternal`, 114 | 114 missing | 114 equal |
| BIMcollab | `Pset_WallCommon.LoadBearing`, 114 | 114 missing | 114 equal |
| modelo_bim (Revit 2023) | `Pset_WallCommon.IsExternal`, 4 | 4 missing | 4 equal |

The property census as a whole: BIMcollab 3 equal and 817 missing before, 231 equal and 589 missing after, 225 different both times; modelo_bim 97 equal and 110 missing before, 101 and 106 after, none different. In BIMcollab's export the walls' values are two Yes/No parameters on the walls, `IsExternal` (true on 71, false on 43) and `LoadBearing` (true on 114): on the walls with a type Function the Function alone gives 113 of 114 (run 41130869879, 1 different, where the parameter says otherwise) and the parameter 114 of 114, which is revit-ifc's order (a parameter named for the property first, RE-193). modelo_bim's four walls have no such parameter and a type Function of 1, so they test the Function path alone, with one value.

## 4. What moved

36 of the 94 exports of the unfiltered run: the 33 samples of 2015 to 2023, Einhoven, BIMcollab and modelo_bim. The other 58 are byte-identical to `main`'s (every file of 2024 to 2027, both offices, the R2025, reference and other held-out models). What a moved sample gains is one `IsExternal` per exported wall in its `Pset_WallCommon`, from its type's Function, and nothing else: the exports of the six 2015 samples, kept and diffed (41146044699), differ by exactly those properties (180, 56, 166, 166, 6 and 4 on the `rac_advanced`, `rac_basic`, both MEP projects, `rst_advanced` and `rst_basic`), and a wall that had no `Pset_WallCommon` gains one. So none of those six gives a parameter named for a standard property a value.

## 5. What this does not claim

- **The Function path on a Revit 2015 to 2022 export beyond BIMcollab's.** Its parameters decide every wall, so the Function path itself is tested on a Revit export only by modelo_bim (4 walls, all exterior). The 2015 to 2022 Function values rest on the later copies.
- **`IsExternal` of anything but walls**, and the parameters named for a property on elements other than the ones whose class a record names: a door's, window's or slab's Function is not read here, and the elements on which a named parameter is read are those of `recover_2023_records` (walls, doors, windows, floors, rooms, columns and other products).
- **That a named parameter is Revit's value on a 2015 to 2022 export.** The only one measured is BIMcollab's two on the walls; the samples' named parameters rest on the copies (RE-281).
- **Curtain walls, mullions and panels**: no Function is read for them on these releases and they get no `IsExternal` unless a parameter gives one.

## 6. Reproduce

```text
gh workflow run measure.yml --ref re/283-wall-function-pre2024 -f ref=9a133e1f59d93a6183b62906507171cb3e01bb57 -f base=3811a6545aff148b14b6e69c28be2e26501c812b \
  -f nordic=true -f r2025=true -f samples=true -f bimcollab=true -f holdout=true \
  -f probe=probe_re283_wall_function_pairs -f probe_scorer=- -f probe_pair_release=2023
gh workflow run measure.yml --ref re/283-wall-function-pre2024 -f ref=9a133e1f59d93a6183b62906507171cb3e01bb57 -f base=none \
  -f samples=true -f sample_years="2023 2024" -f probe_pair_release=2024 -f models=einhoven -f scorers=none \
  -f probe=probe_re283_wall_function_pairs -f probe_scorer=-
gh run download <run id>   # measure-head/sample-*/probe-pair.txt; bimcollab-str-2015/counts.txt
```
