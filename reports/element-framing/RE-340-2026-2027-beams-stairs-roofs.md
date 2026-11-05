# RE-340: Revit 2026 and 2027 beams, stair runs, landings and roof slopes

**Date:** 2026-11-04
**Issues:** #1190 (geometry beyond the box on 2015 to 2023, 2026 and 2027); follows RE-124 and RE-179 (2026 and 2027 element records are 2025's).
**Artefacts:** Autodesk's own sample projects, pinned in `research/autodesk-sample-projects.tsv` and fetched by `tools/fetch-autodesk-samples.sh`, never stored: `rac_advanced` 2024 (`ff791052…e5afc`), 2025 (`512e9188…e25d1`), 2026 (`f5fb3712…83ace`) and 2027 (`a88eb9f8…024ed`); `rac_basic` 2025 (`41f1893a…d4293`), 2026 (`bc547fe9…d99dc`) and 2027 (`c9ce4d01…91a01`); `rst_basic` 2024 (`aab838f8…c616e`), 2025 (`5089b2c6…31acb`), 2026 (`e7cf23a9…75418`) and 2027 (`01685786…d1453`).
**Scorer:** `tools/re/save_pair_bodies.py`.
**Status:** positive against the same samples saved by Revit 2024 or 2025. **No Revit 2026 or 2027 IFC export is available**, so nothing here is measured against Revit's own export of a 2026 or 2027 file. The oracle is the same model saved by an earlier release, whose export is measured against Revit's (RE-50, RE-56, #357, #824).

## 1. Before

On `main`, 2026 and 2027 files get the 2025 records (RE-124, RE-179): walls with their layers and joins, family instances' boxes and saved meshes are the same as on 2025. Six readers are still limited to 2024 or 2025 by their release lists, so on 2026 and 2027 these are drawn as their record boxes:

| reader | release list on `main` |
|---|---|
| a beam's axis (`partition_beam_axes`, RE-49) | 2024 |
| a sketch line's segment (`scan_sketch_bounded_lines`, RE-50, RE-199) | 2024, 2025 |
| a stair run from its native records (`native_stairs`, #357) | 2025 |
| a stair run's shape (#323) | 2024, 2025 |
| a landing's top and thickness (#824) | 2024 |
| a roof's slope (`partition_roof_slopes`, RE-56) | 2024, 2025 |

Every body source the samples' exports report (`exported.body_sources`) on `main`:

| body source | `rac_advanced` 2024 / 2025 / 2026 / 2027 | `rac_basic` 2025 / 2026 / 2027 | `rst_basic` 2024 / 2026 / 2027 |
|---|---|---|---|
| `partition_beam_axis` | 29 / 0 / 0 / 0 | | 345 / 0 / 0 |
| `partition_beam_box_along_axis` | 8 / 0 / 0 / 0 | | 9 / 0 / 0 |
| `partition_stair_run_sketch` | 5 / 10 / 0 / 0 | 2 / 0 / 0 | 1 / 0 / 0 |
| `native_landing_tread` | 5 / 0 / 0 / 0 | | |
| `partition_roof_slope` | 1 / 1 / 0 / 0 | | |

## 2. The change

The six lists gain 2026 and 2027. The records these readers read are the 2025 ones on 2026 and 2027 (RE-124, RE-179), and nothing else in the readers changes.

A wall's profile openings (RE-151) checked the beam reader's list, not one of their own. They are measured on 2024 only (Core Interior's wall 55840), so they now have their own list, 2024, and take no openings on a release that only the beam reader gains.

2025 is not widened. A 2025 file's beams and landings would change as well, and the only Revit 2025 exports here that hold them are local, unpublished exports, so they cannot measure it.

## 3. The 2026 and 2027 saves against the earlier ones

`rvt-ifc FILE -o out.ifc` on each save, scored with `tools/re/save_pair_bodies.py` against the save named. Elements are paired by ElementId (`Tag`), and a pair is the same where the world boxes agree to 3 mm and the volumes to 0.1%.

**The bodies these readers now draw**, against the save that draws them on `main`:

| candidate | reference | bodies | same |
|---|---|---:|---:|
| `rst_basic` 2026 and 2027 | `rst_basic` 2024 | 355 each (345 beams by their axis, 9 along it, 1 stair run) | 355 each |
| `rac_advanced` 2026 and 2027 | `rac_advanced` 2024 | 43 each (37 beams, 5 landings, 1 roof) | 43 each |
| `rac_advanced` 2026 and 2027 | `rac_advanced` 2025 | 10 stair runs each | 10 each |
| `rac_basic` 2026 and 2027 | `rac_basic` 2025 | 2 stair runs each | 2 each |

`rac_advanced`'s 5 other stair runs are drawn from their sketch on 2025 and as their boxes on 2024, so they are scored against 2025. On `main` the 2025 save's sloped roof (201262) is already not its 2024 save's: 12 triangles and 6.88 m³, against 532 triangles and 5.33 m³. The 2026 and 2027 roofs are the 2024 body.

**Every body in the file**, against the reference save, `main` then this change:

| candidate | reference | different on `main` | different here |
|---|---|---:|---:|
| `rst_basic` 2026 | `rst_basic` 2024 | 35 of 453 | 1 of 453 |
| `rst_basic` 2027 | `rst_basic` 2024 | 35 of 453 | 1 of 453 |
| `rac_advanced` 2026 | `rac_advanced` 2025 | 64 of 5,527 | 63 of 5,527 |
| `rac_advanced` 2027 | `rac_advanced` 2025 | 64 of 5,527 | 63 of 5,527 |
| `rac_basic` 2026 | `rac_basic` 2025 | 27 of 438 | 23 of 438 |
| `rac_basic` 2027 | `rac_basic` 2025 | 27 of 438 | 23 of 438 |

Which bodies change, element for element:

- **`rst_basic` 2026 and 2027:** 34 become 2024's (32 beams, a slab and a stair run) and none stops being it. The one left is a column, as on `main`.
- **`rac_basic` 2026:** 4 become 2025's (2 stair runs and 2 slabs drawn from their sketch) and none stops being it.
- **`rac_advanced` 2026:** 10 stair runs become 2025's. 9 bodies stop being 2025's and are 2024's instead: 2 beams and 5 landings, which 2025 still draws as boxes, and a ceiling (147534) and a floor (139803) that the sketch lines now draw as on 2024 (2025's are 0.99 m³ and 0.60 m³ smaller). The rest left on `rac_advanced` and `rac_basic` are walls and coverings, as on `main`.

2027's counts are 2026's on every sample.

## 4. Nothing else moves

107 files were exported with `main` (77d71cbb) and with this change, `SOURCE_DATE_EPOCH=1700000000`. 101 are byte for byte the same, IFC and diagnostics. They include every 2015 to 2025 file here: Autodesk's samples of 2015, 2016 and 2019 to 2025, Core Interior, the four RE1 models, Einhoven, the BIM4LCA office and terrain housing, and puzzbobb's Revit 2025 models. The 6 that change are the 2026 and 2027 saves of `rac_advanced`, `rac_basic` and `rst_basic`.

## 5. Not measured

- Revit 2026 and 2027 against Revit's own export: none is available.
- 2025's beams (RE-49) and landings (#824): left for a Revit 2025 export that can be published.
- 2015 to 2023: these readers stay at 2024 and later.
- A wall's profile openings on 2025 to 2027 (RE-151): left at 2024.

## 6. Reproduce

```bash
tools/fetch-autodesk-samples.sh samples 2024 2025 2026 2027
for f in samples/*.rvt; do rvt-ifc "$f" -o "${f%.rvt}.ifc"; done
python3 tools/re/save_pair_bodies.py samples/2027-rstbasicsampleproject.ifc samples/2024-rstbasicsampleproject.ifc
python3 tools/re/save_pair_bodies.py samples/2026-racadvancedsampleproject.ifc samples/2024-racadvancedsampleproject.ifc \
    --sources partition_beam_axis,partition_beam_box_along_axis,native_landing_tread,partition_roof_slope
```
