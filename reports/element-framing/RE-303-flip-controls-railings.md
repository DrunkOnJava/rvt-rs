# RE-303 — A saved flip control holds no geometry, but marks railings whose saved mesh is not whole

**Date:** 2026-11-03
**Issues:** #1247, #255; follows RE-224, RE-300 and RE-301
**Probe:** `probes/examples/probe_re303_biflip_controls.rs`
**Runs:** Measure [41562037317](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41562037317) (the probe), [41563145096](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41563145096) (the rule tried, at 74eabc3b, `--saved-meshes`; its head scored against `main` at 3e0ad51d from Measure [41560811584](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41560811584), the same commit, flags and models)
**Result:** negative. The saved graphics class `GBiFlipControl` holds no geometry. Leaving it out as a branch with no surface, as curves are, draws the 14 BIM4LCA office bodies it alone kept as boxes, and none of them is Revit's: all are railings, and 12 cover only about 40% of Revit's volume. The class stays a decode failure, which keeps those railings' boxes.

## 1. What a `GBiFlipControl` holds

The probe reads every saved graphics record of the office and four 2024 samples: 1,489 `GBiFlipControl` objects (738 on the office, on 490 owners). Every one has the same four fields and no pointer: `m_GInfo` (category 192 or 193, a control command, flags 557124 on all but 7 of rst basic's 23), `m_origin` and `m_base` (3-vectors) and `m_length` (16.0 on every one printed). Their parents are `GFilter`s (usually beside a `GConditionDir` or `GConditionInt`), `GGroup`s and `GElement`s. It is a flip control, drawn in a view; it holds nothing of a body.

## 2. The rule tried, and why it is withdrawn

Leaving every `GBiFlipControl` out as a branch with no surface (74eabc3b), measured with `--saved-meshes` against `main`:

| file | bodies drawn, before → after | kept as boxes, before → after |
|---|---|---|
| BIM4LCA office as built | 10,924 → 10,938 | 279 → 265 |
| BIM4LCA office at permit | 11,041 → 11,055 | 279 → 265 |
| Snowdon Towers architectural (2024 sample) | 2,303 → 2,335 | 314 → 282 |
| rac basic (2024 sample) | 294 → 300 | 39 → 33 |
| the others | unchanged | unchanged |

Against Revit's export, the office's scored bodies rise from 1,495 to 1,509, and the exact ones, those within 0.01 ft and those within 1% of volume do not move: none of the 14 new bodies is Revit's. All are `IfcRailing`:

- 12 "Railing:KAI-4" and "KAI-5": rvt-rs's surface lies on Revit's (within 0.0002 ft), but Revit's body reaches up to 1.0 ft beyond it, at 0.39 to 0.54 of Revit's volume. Parts of the railing are not in its saved mesh.
- 2 "Railing:KAI-3": Revit's surface lies on rvt-rs's (within 0.0042 ft), and rvt-rs's reaches 0.062 ft beyond it, at 1.19 of Revit's volume.

So on this file the flip control sits only in railings whose saved mesh is not their body, and the failure it raises is what keeps their boxes. Leaving it out would write those meshes, partial or overgrown, under the maintainer's rule that no partial mesh is drawn (2026-11-02). It stays a failure.

## 3. Not claimed

- Why a railing's saved mesh lacks parts of Revit's body (its rails and balusters may be drawn by other elements' graphics). That is the open question for railings, not the flip control's.
- The samples' 38 bodies the rule would draw: they have no Revit export to score.

## 4. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=6219046d -f base=none -f probe=probe_re303_biflip_controls \
  -f nordic=true -f samples=true -f sample_years="2024 2024" \
  -f models="nordic-office-as-built sample-2024-racbasicsampleproject sample-2024-racadvancedsampleproject sample-2024-snowdon_towers_sample_architectural sample-2024-rmebasicsampleproject" \
  -f scorers=none
gh workflow run measure.yml --ref main -f ref=74eabc3b -f base=3e0ad51d -f ifc_flags=--saved-meshes \
  -f nordic=true -f samples=true -f sample_years="2024 2024" \
  -f models="core-interior re1-* nordic-office-* sample-2024-*" -f scorers="mesh_bodies_vs_ifc schema"
```
