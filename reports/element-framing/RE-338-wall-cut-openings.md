# RE-338 — The openings a cabinet, a door and a curtain wall cut in a wall

**Date:** 2026-11-04
**Issues:** #1165 (unfilled openings), W4, W6 and W7 of [`docs/release-1.0-criteria.md`](../../docs/release-1.0-criteria.md); follows [RE-321](RE-321-unfilled-openings.md), which wrote the rectangular wall openings and measured these three kinds without writing them
**Artefacts:** the BIM4LCA office as built and at building permit (Revit 2024, Revit 24.0.5's IFC4 exports, CC BY-SA 4.0), as `research/bim4lca-measure.tsv` pins them:

| file | sha256 |
|---|---|
| `ARK_NordicLCA_Office_Concrete_As-Built.rvt` | `747120005b4b35c7cb66ec61158aab50ded95e052e172cb387b86cd15b31d906` |
| `ARK_NordicLCA_Office_Concrete_As-Built_Revit.ifc` | `b2f530eba8b4424c4542b98e9a2c7bbcc726367f21b2740f3074e4a6ce917382` |
| `ARK_NordicLCA_Office_Concrete_BuildingPermit.rvt` | `1c12c2058128315edbd65a8bf97f60da971aee8625f21233b8477d9c6165c1d1` |
| `ARK_NordicLCA_Office_Concrete_BuildingPermit_Revit.ifc` | `bc3bd3366a3c413ce74840300a590f59102b740118ef37898e906253b6b7c08d` |

**Scorer:** `tools/re/unfilled_openings_vs_ifc.py` (RE-321)
**Status:** measurement pending; this section is filled from the Measure runs below.

## 0. Result

To be filled from the runs.

## 1. What `main` writes and Revit's export holds

On `main` at 240c7d7c the office as built has 220 unfilled openings in Revit's export and rvt-rs writes 75, 58 of them Revit's. Of the 162 Revit's export holds that rvt-rs does not write, by what cuts them:

| kind (cutter in host) | Revit's, as built |
|---|---:|
| a curtain wall in a wall | 70 |
| a door in a wall (a second wall the door's opening reaches) | 39 |
| a fire hose cabinet in a wall | 39 |
| a door in a door | 4 |
| a floor's sketch void in a floor | 9 (RE-151, not this report) |
| a wall in a wall | 1 |

Core Interior (63 of 63) and RE1 Architecture hold none of the three kinds, so the office is the only reference.
