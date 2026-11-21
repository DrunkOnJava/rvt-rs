# RE-496: The edge two cylinder faces share is refined to half the faces' chord

**Date:** 2026-11-20
**Issues:** #2111 (this change, #2116); #2107 (RE-495), #2104, #2114.
**Artefacts:** the BIM4LCA office and housing files (VTT, CC BY-SA 4.0; Revit 2024), each with Revit's own IFC4 export; RE1 Architecture and Plumbing (Revit 2025, MIT); the held-out house; Autodesk's sample projects of Revit 2015 to 2027 and Snowdon Towers' 2024 models, which have no Revit export in Measure. Outside Measure, kept local: Revit's IFC4 export of Snowdon Towers Architectural (bldrs-ai/test-models, no licence).
**Probes:** `probe_2098_face_cracks` with the cylinder standoff of #2111 (`cylsag`), on each bound and on `main` at `26a6b0e1` (`e1f9649d`); `probe_2111_sliver_faces`. Not landed; kept as `refs/archive/2026-11-20/measure-2111-chord-cand`, `-chord-cand2`, `-head`, `-base`, `-sliver` and `-sliver-base`. The local reads are `research-tools/2111/` on `refs/archive/2026-11-20/measure-2111-reads`.
**Status:** positive. At half the faces' chord every boundary vertex between two cylinders stays within 0.001 ft on the 77 files the crack probe reads, and 182,090 of the 297,784 cylinder triangles #2107 added go (each graph counted once, 61 per cent); on the 13 files whose triangle totals #2107 raised by more than 10 per cent, 48 to 100 per cent of its rise goes. On 147 graphs the farthest cylinder standoff rises, by at most 0.000318 ft, within 0.001 ft. Griffin chose this over #2107's finer edge (2026-11-20). The rows were registered on #2111 before each run was read.

## 1. The rule

#2107 refined an edge two cylinder faces share until the chord between neighbouring points was within `EDGE_SAGITTA` (0.00025 ft), a quarter of the 0.001 ft chord the cylinder faces are meshed to (`CylinderTessellationProfile::chord_error`, the default profile both callers of `cylinder_face` use). It is now refined to half that chord, 0.0005 ft. `EdgeLimits` carries the bound, so `bisect_crossing_edge` takes it from its caller; the edges a cylinder shares with a plane keep `EDGE_SAGITTA`.

At the whole chord (`42cf34ad`), two downpipes on each BIM4LCA housing file (`Syöksytorvi:HLV-3.ST70`) kept two vertices over 0.001 ft between the shared points, the farthest 0.001871 ft; on the concrete files, before #2107, 22 of their vertices were over, with the same farthest. Halving the bound closes them.

Read before this change ([#2111](https://github.com/DrunkOnJava/rvt-rs/issues/2111#issuecomment-7048394440)): on the Snowdon Towers toilet's and sink's cylinder faces, #2107's slivers stand off their cylinder by at most 0.000080 and 0.000098 ft, where the triangles `main` drew nearest the same Revit vertices stood off 0.000714 and 0.000950 ft; the ratios fit the squares of the strip widths. So a coarser edge trades closeness to the cylinder for triangles.

## 2. Measured

**At the whole chord** ([reading](https://github.com/DrunkOnJava/rvt-rs/issues/2111#issuecomment-7049619398); probe 44793236259, Measure 44793243838): rows 1 and 2 failed on the downpipes above; rows 3, 4, 5, 7 and 8 held, and the triangles fell almost to where they were before #2107.

**At half the chord** ([reading](https://github.com/DrunkOnJava/rvt-rs/issues/2111#issuecomment-7050804405); probe [44801424565](https://github.com/DrunkOnJava/rvt-rs/actions/runs/44801424565), Measure [44807747546](https://github.com/DrunkOnJava/rvt-rs/actions/runs/44807747546), against `26a6b0e1`): every row holds.

- No boundary vertex between two cylinders over 0.001 ft on any file; the downpipes' farthest gap over every pair is 0.000920 ft, as with #2107. 1,265 bodies with a cylinder pair have a farthest gap above #2107's, the highest 0.000466 ft.
- No other pair of surfaces gains a vertex over 0.001 ft, and no body does.
- The farthest cylinder standoff stays within 0.001 ft on all 1,492 graphs (0.000997), rises on 147 by at most 0.000318 ft (median 0.000281), and ends above its figure before #2107 on none. The cylinder triangles standing off more than 0.0005 ft are the same as #2107's on every graph (193,887); that count is coarse.
- The cylinder triangles, each graph counted once: 1,385,188 before #2107, 1,682,972 with it, 1,500,882 now. On the 13 files whose totals #2107 raised by more than 10 per cent, in the crack probe's totals:

  | file | before #2107 | #2107 | half chord | of #2107's rise taken back |
  | --- | ---: | ---: | ---: | ---: |
  | `rme_basic` (each of five years) | 232,384 | 313,576 | 257,048 | 70% |
  | `rme_advanced` (each of four) | 224,315 | 289,859 | 238,815 | 78% |
  | terrain housing (each of two) | 1,160 | 1,512 | 1,160 | 100% |
  | RE1 Plumbing | 4,773 | 5,973 | 5,181 | 66% |
  | Snowdon Towers HVAC | 141,520 | 175,792 | 159,232 | 48% |

  The 2024 saves' exports: `rme_basic` 106.0, 112.7 and 108.0 MiB; `rme_advanced` 102.1, 107.4 and 103.3; Snowdon Towers HVAC 69.2, 71.8 and 70.5; Plumbing 129.9, 134.2 and 130.9.
- No body appears, disappears or is newly refused in the probe; all 102 `diagnostics.json` are identical; no `psets_vs_ifc` or `qtos_vs_ifc` line moves; the same 31 exports differ as for #2107, and every one of the 2,668 bodies that move on the 30 kept is on a `cylpair` line; the same-project gates pass on `ae2d0d91`.
- The only count rows that move are RE1 Architecture's shower head and its class row, the scorer's sample (#2114): read with every vertex, its farthest vertex is 0.010066 ft on both sides. Over the 62 Snowdon Towers Architectural bodies #2107 moved, Revit's vertices beyond 0.001 ft are 23,427 (23,511 with #2107, 23,243 before), and beyond 0.01 ft 8,809 (8,809 and 8,806); why the first falls is not read.
- **Read with every vertex, as the scorer's sample cannot (#2114):** on RE1 Architecture and Plumbing and on the BIM4LCA housing's concrete as-built and terrain concrete files, against Revit's exports (fetched by `tools/ci/fetch-bim4lca.py` from `research/bim4lca-measure.tsv`), none of the 40 bodies whose mesh changes is worse than with #2107 by Revit's vertices beyond 0.001 ft or 0.01 ft, or by rvt-rs's farthest vertex from Revit's surface (`fullcheck.py`, `refs/archive/2026-11-20/measure-2111-reads` at `b060ca4b`). Revit's vertices beyond 0.001 ft go from 168 to 164 on RE1 Architecture and are the same on the other three.

## 3. Reproduce

```text
gh workflow run measure.yml --ref main -f ref=2784f5bb9c741fb0e6369d7d6e6d2ec07a44e0e6 -f base=e1f9649dd7d03ca8ce5b6b790f8c867456c2d459 -f nordic=true -f samples=true -f probe=probe_2098_face_cracks -f scorers=none
gh workflow run measure.yml --ref main -f ref=ae2d0d914557fc9d57270dc3d2c04fae5984cbde -f base=26a6b0e12ec84d5695429ae2ae5fa2a0840dbb0a -f nordic=true -f r2025=true -f samples=true -f holdout=true -f keep_ifc=true
```

## 4. Not claimed

- **Viewer load or speed:** not measured; the export sizes above are the measure.
- **A pair's gap below its body's farthest:** the probe prints a pair's farthest only past 0.001 ft.
- **The held-out house:** its export differs and is not kept, so its bodies are not read.
