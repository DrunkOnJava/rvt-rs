# RE-301 — A saved cylinder with a left-handed basis is the same cylinder

**Date:** 2026-11-02
**Issues:** #1247 (saved meshes that v0.6.1 drew and v0.12.0 draws as boxes), #255; follows RE-224
**Runs:** Measure [41540910235](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41540910235) (`probe_re224_saved_mesh_status` on `main` at 895109d9: the census below), Measure [41541977595](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41541977595) (this change at e59213a1 against `main`, `--saved-meshes`, `mesh_bodies_vs_ifc`)
**Result:** positive. `native_saved_mesh::cylinder_face` rejected a `CylSurf` whose `m_xVec × m_yVec` is `−m_zVec`, which keeps the whole element's box (RE-224). Such a basis describes the same cylinder; only the parametric normal ∂P/∂u × ∂P/∂v points inward instead of outward. Reading it as on a plane (the normal is the parametric one, turned by `m_orientFlag` and the face flag) restores 100 bodies on the BIM4LCA office, 66 of them exact to 0.001 ft against Revit's export, and none further from Revit's than 0.011 ft.

## 1. Why meshes became boxes

#1247 reports that v0.12.0 draws as 12-triangle boxes many elements v0.6.1 drew from their saved meshes (`--saved-meshes`). Measure [41537426493](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41537426493) reproduces it on our files (v0.6.1 against v0.12.0, both with `--saved-meshes`): Snowdon Towers electrical (Revit 2024, an Autodesk sample) draws 1,420 bodies from saved meshes with v0.6.1 and 907 with v0.12.0, the architectural sample keeps 754 boxes, the office 357. Every element lost is still a record box in v0.12.0, kept by RE-224's gate: some part of its saved graphics did not decode. The census of what did not decode, on the office, RE1 and the 2024 samples (3,020 such elements):

| what did not decode | elements | with nothing else |
|---|---:|---:|
| graphics class `GImposter` | 1,813 | 115 |
| `GPolyMesh` normal binding (always with `GImposter`) | 1,463 | 0 |
| left-handed cylinder basis | 526 | 484 |
| non-rectangular cylinder trim | 473 | 222 |
| `GEllipse` profile curve | 405 | 6 |
| graphics class `GBiFlipControl` | 117 | 52 |

This report takes the left-handed cylinder. `GImposter`, which holds every Snowdon Towers light fixture, is RE-300.

## 2. The reading

`cylinder_face` evaluates `P(u, v) = centre + r(cos u·x + sin u·y) + v·z`, valid for any orthonormal basis. ∂P/∂u × ∂P/∂v is `r(cos u·x + sin u·y)` when `x × y = z` and its negative when `x × y = −z`. The grid's triangles are wound along ∂P/∂u × ∂P/∂v, turned by the orientation as before, so they need no change; the per-vertex normals now take the same sign, as a plane's `x × y` does. Vertex positions do not depend on the basis's handedness at all.

## 3. Measured

Measure 41541977595, this change against `main`, both with `--saved-meshes`:

| file | bodies drawn from saved meshes, before → after | kept as boxes, before → after |
|---|---|---|
| BIM4LCA office as built (2024) | 10,781 → 10,881 | 357 → 322 |
| BIM4LCA office at permit (2024) | 10,896 → 10,998 | 359 → 322 |
| RE1 Architecture (2025) | 51 → 52 | 5 → 4 |
| Snowdon Towers architectural (2024 sample) | 1,799 → 1,971 | 754 → 646 |
| Snowdon Towers plumbing (2024 sample) | 2,500 → 2,516 | 137 → 121 |
| rme basic and advanced (2024 samples) | 5,074 → 5,096 and 5,078 → 5,099 | 566 → 551 |
| rac basic (2024 sample) | 270 → 284 | 49 → 49 |
| rst basic (2024 sample) | 63 → 67 | 0 → 0 |
| Core Interior, RE1 Electrical, Mechanical, Plumbing | unchanged | unchanged |

Snowdon Towers electrical keeps 907; its count of partial meshes rises from 513 to 556, because 43 fixtures whose only surfaces were left-handed cylinders had no mesh at all before and now have one that their `GImposter` leaves partial.

Against Revit's own export (`mesh_bodies_vs_ifc`, the bodies scored):

| file | scored, before → after | exact (0.001 ft) | within 0.01 ft | volume within 1% |
|---|---|---|---|---|
| office as built | 1,352 → 1,452 | 1,294 → 1,360 | 1,345 → 1,438 | 1,340 → 1,434 |
| office at permit | 1,438 → 1,540 | 1,376 → 1,444 | 1,427 → 1,522 | 1,425 → 1,521 |
| RE1 Architecture | 51 → 52 | 45 → 45 | 49 → 50 | 51 → 52 |

The office's doors drawn from saved meshes rise from 48 scored (46 at permit) to 114, all 114 exact; the default export draws them, so this change reaches it as well as `--saved-meshes`. The new bodies that are not within 0.01 ft are seven "Sink Kitchen-Single" at 0.0106 ft (Revit's vertices within 0.0040 ft of rvt-rs's surface, volume within 1.3%), a tessellation difference; RE1's new chair is within 0.0021 ft. The IFC4 schema check is unchanged but for 100 fewer swept solids.

The meshes cost size: with `--saved-meshes` the office as built goes from 331,302 to 447,784 triangles and its IFC from 70.6 MB to 75.1 MB, and Snowdon Towers architectural from 648,933 to 974,573 triangles (12.8 MB more).

## 4. Not claimed

- The other kinds in §1; `GImposter` is RE-300.
- The normals' direction is not scored: the scorers test vertex positions and volume.

## 5. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=e59213a1 -f base=main -f ifc_flags=--saved-meshes \
  -f nordic=true -f samples=true -f sample_years="2024 2024" \
  -f models="core-interior re1-* nordic-office-* sample-2024-*" -f scorers="mesh_bodies_vs_ifc schema"
```
