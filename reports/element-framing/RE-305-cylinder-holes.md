# RE-305 — A cylinder face trimmed by several loops

**Date:** 2026-11-03
**Issues:** #1247, #255; follows RE-224 and RE-304
**Run:** Measure [41573475107](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41573475107) (the change at 0f377661 against `main` at 89793841, `--saved-meshes`, `mesh_bodies_vs_ifc` and `schema`)
**Result:** negative. Meshing a cylinder face whose trim has holes draws 30 more BIM4LCA office bodies from their saved meshes, all of them one family's wall-mounted fittings and none of them Revit's body. The change stays on `research/1247-cylinder-holes` at 0f377661.

## 1. The gap

RE-304 meshes a cylinder face trimmed by any single loop, in strips across its angle, and still rejects a face with several loops (`multiple cylinder trim loops unsupported`), which keeps the whole element's box (RE-224). After RE-304 that keeps 45 elements as boxes on the office as built, 30 for it alone (`probe_re224_saved_mesh_status`, Measure [41572297689](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41572297689)).

## 2. The reading

Every loop of the face is read in UV from its edges, as RE-304 reads the one. A loop inside another is a hole; each outer loop, with the loops directly inside it as holes, is earcut in arc-length coordinates `(r·u, v)`, and each triangle, being convex, is cut exactly into strips across U at the U values RE-304 uses, each piece fanned. The pieces must cover the outer loops' area less the holes' to within 1e-6 of it, or the loops are taken as self-intersecting and the face rejected; a loop inside a hole rejects the face (`nested cylinder trim loops unsupported`).

## 3. Measured

Against Revit's own export (run 41573475107):

| file | saved-mesh bodies scored, before → after | new bodies within 1% of Revit's volume | within 0.01 ft |
|---|---|---:|---:|
| office as built | 1,650 → 1,680 | 0 of 30 | 0 of 30 |
| office at permit | 1,738 → 1,768 | 0 of 30 | 0 of 30 |

The 30 are all `Stay_Wall_Mounted_:VAR-1` (`IfcFurniture`). The scorer lists the farthest 15 and the 10 with the largest volume difference: their volumes are 1.109 to 1.120 of Revit's (no fitting is over 1.1201), Revit's vertices lie within 0.0047 to 0.0051 ft of rvt-rs's surface, and rvt-rs's at most 0.0118 ft from Revit's. The error is surface rvt-rs draws and Revit does not, in some shells and not others. Of the five shells of each of the 10 fittings the scorer lists by shell, ranked by volume, the largest and the third are within 3% of Revit's (0.0276 against 0.0268 cubic ft, 0.0099 against 0.0099); the second is 29% over (0.0239 against 0.0185), and the two smallest are over at the scorer's precision (0.0009 or 0.0010 against 0.0005, and 0.0005 against 0.0004). The office as built goes from 568,965 to 822,135 triangles, about 8,400 a fitting. Core Interior, RE1 and the 24 R2025 models are unchanged. Of Autodesk's 2024 samples, unscored, 6 of the 10 change (rac advanced, rme advanced and basic, and Snowdon Towers architectural, electrical and HVAC, which alone draws 407 more bodies).

RE-302's toilets (1.027 to 1.047 of Revit's volume where the scorer lists them) and these fittings look alike: Revit's surface lies on rvt-rs's to about 0.005 ft while rvt-rs draws more, in particular shells. That is not specific to them (`main`'s kitchen sinks show Revit's surface within 0.0039 ft of rvt-rs's), and this change does not use RE-302's edge trace, so no shared cause is shown. What draws the extra surface is open for both.

## 4. Not claimed

- Any element drawn by this change: it is not on `main`.
- That the change is wrong on the samples, which no Revit export scores.
- That the extra surface comes from the holes rather than from what RE-302 also meets.

## 5. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=0f377661 -f base=89793841 -f ifc_flags=--saved-meshes \
  -f nordic=true -f r2025=true -f samples=true -f sample_years="2024 2024" \
  -f models="core-interior re1-* nordic-office-* r2025-* sample-2024-*" -f scorers="mesh_bodies_vs_ifc schema"
# the census on main that §1 counts: an element is kept where its `mesh` line is
# partial_saved_graphics with a nonzero diagnostics field and a message other than
# the nested-detail and view-component ones
gh workflow run measure.yml --ref main -f ref=89793841 -f base=none -f nordic=true \
  -f probe=probe_re224_saved_mesh_status -f models=nordic-office-as-built -f scorers=none
```
