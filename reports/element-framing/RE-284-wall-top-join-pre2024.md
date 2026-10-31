# RE-284: whether a wall's top is joined to a roof or floor, from its native record on Revit 2015 to 2023

**Date:** 2026-10-30
**Issues:** #1158; follows RE-228 (`ExtendToStructure` on 2024 and 2025), RE-280 and RE-281 (the native records of 2015 to 2022)
**Artefacts:** Autodesk's sample projects of 2015 to 2027 (`research/autodesk-sample-projects.tsv`, fetched by hash and never stored); BIMcollab's Revit 2015 structural example (`02_BIMcollab_Example_STR.rvt`, sha256 `8cd1f122…e193`, with Revit 2015's own IFC2X3 export; no licence stated, so only counts left the runner); modelo_bim (saved in Revit 2023, with the export Revit 26.2 made; counts only); the reference, BIM4LCA, R2025 and held-out models of the unfiltered run.
**Runs:** Measure [41137770745](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41137770745) (the unfiltered run, `-f base=main -f nordic=true -f r2025=true -f samples=true -f bimcollab=true -f holdout=true`, this change at `51e74588` against `main` at `3811a654`, with the probe and the 2015-to-2023 pairs) and [41138445109](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41138445109) (the 2023 samples against their 2024 copies).
**Probe:** `probes/examples/probe_re284_wall_top_join_pairs.rs`.
**Status:** positive, experimental on 2015 to 2022. A wall's `ExtendToStructure` is read from its native record on Revit 2015 to 2023 as it is on 2024 and 2025: its `SWall` or `ArcWall` holds a `JoinToRoofGStep` with `m_baseOrTop` 1. Against the same projects saved by a later release the value is equal on all 1,983 walls both files hold (24 pairs, 2015 to 2022 against 2023), on all 249 walls of the 2023 samples against their 2024 copies, and on the 2024 to 2027 copies against the 2023 ones. Against Revit's own exports it is equal on all 114 of BIMcollab's walls (Revit 2015) and all 4 of modelo_bim's (Revit 2023 file, Revit 26.2 export), which both hold only the value false: the value true is measured on 2024 and 2025 (RE-228) and, on 2015 to 2022, only against the copies.

## 1. What changed

- `native_walls::wall_top_joins` reads the flag for the walls it is given, and nothing else of `native_walls_and_top_joins`, with `Options::releases` set to `NATIVE_RELEASES`. The predicate is the one `native_walls_and_top_joins` used (`is_top_joined`); a wall's flag needs no ElementId, so the 4-byte `Identifier` of 2015 to 2023 does not matter to it (RE-282).
- `attach_wall_top_joins` gives each wall `WALL_TOP_JOINED_FIELD` on the releases in `WALL_TOP_JOIN_RECORD_REVIT_VERSIONS` (2015 to 2023), the releases whose walls come from their element records (`recover_2023_records`). Nothing else of what `attach_wall_layers` reads from the walls' native records on 2024 and later (their height, their top cut, their side areas and volumes: RE-266, RE-267, RE-270) is read on these releases: none is measured on them.

## 2. Against the same project in 2023

`probe_re284_wall_top_join_pairs` reads the flag of every declared element of a sample and of its copy and compares the walls both hold.

| older release | pairs | walls read in both | equal | different |
|---|---:|---:|---:|---:|
| 2015 | 3 | 240 | 240 | 0 |
| 2016 | 3 | 249 | 249 | 0 |
| 2017 | 3 | 249 | 249 | 0 |
| 2018 | 3 | 249 | 249 | 0 |
| 2019 | 3 | 249 | 249 | 0 |
| 2020 | 3 | 249 | 249 | 0 |
| 2021 | 3 | 249 | 249 | 0 |
| 2022 | 3 | 249 | 249 | 0 |
| 2024 to 2027 (against the 2023 copy) | 12 | 249 each | 249 each | 0 |
| 2023 (against the 2024 copy) | 3 | 249 | 249 | 0 |

The 2015 `rst_basic` is another file under the same name (RE-280 §7) and shares no wall with its copy. Both values are exercised: the 2015 `rac_advanced` has 56 walls without a top join and 124 with, the `rac_basic` 44 and 16, and the 2015 `rst_advanced` 6 with; the two MEP projects hold 166 walls each, none joined.

## 3. Against Revit's exports

| model | Revit values | equal | different | missing | values read |
|---|---:|---:|---:|---:|---|
| BIMcollab, Revit 2015 (export of Revit 2015) | 114 | 114 | 0 | 0 | 138 walls, all false |
| modelo_bim, Revit 2023 (export of Revit 26.2) | 4 | 4 | 0 | 0 | 4 walls, all false |

Before this change both rows were missing in full (`psets_vs_ifc`, Measure 41117344693 and the base side of 41137770745).

## 4. What moved

Of the 94 exports of the unfiltered run, 58 are byte-identical to `main`'s and 36 moved, all of them files of Revit 2015 to 2023 that have walls: the 33 samples of 2015 to 2023, Einhoven, BIMcollab and modelo_bim. Every file of 2024 to 2027, the offices, the R2025 models and the reference models is identical. A moved export gains one `ExtendToStructure` per wall in its `Pset_WallCommon`.

## 5. What this does not claim

- That the value true is exact on a Revit 2015 to 2022 export: no export of such a file holds a wall joined at its top (BIMcollab's walls are all false). The value true is exact on 2024 and 2025 (RE-228) and equal to the copies on 2015 to 2022.
- That a wall's height, top cut or side quantities are read on these releases (RE-266, RE-267, RE-270): they are not.
- The reader's time was not measured on its own: the pass reads the wall records of the file, as `wall_type_functions` does (RE-283).

## 6. Reproduce

```text
gh workflow run measure.yml --ref main -f ref=51e745884b367c2d0b6c406323656c5d27827fe0 -f base=3811a6545aff148b14b6e69c28be2e26501c812b \
  -f nordic=true -f r2025=true -f samples=true -f bimcollab=true -f holdout=true \
  -f probe=probe_re284_wall_top_join_pairs -f probe_scorer=- -f probe_pair_release=2023
gh workflow run measure.yml --ref main -f ref=51e745884b367c2d0b6c406323656c5d27827fe0 -f base=none \
  -f samples=true -f sample_years="2023 2024" -f probe_pair_release=2024 -f models=einhoven -f scorers=none \
  -f probe=probe_re284_wall_top_join_pairs -f probe_scorer=-
gh run download <run id>   # measure-head/sample-*/probe-pair.txt; bimcollab-str-2015/counts.txt
```
