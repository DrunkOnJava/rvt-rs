# RE-300 — A light source's saved graphics are a filter of their own, left out of the body

**Date:** 2026-11-02
**Issues:** #1247 (light fixtures and other saved meshes that v0.6.1 drew and v0.12.0 draws as boxes), #255; follows RE-224 and RE-301
**Probe:** `probes/examples/probe_re300_graphics_imposters.rs`
**Runs:** Measure [41542687542](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41542687542) and [41546059869](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41546059869) (the probe), [41544207306](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41544207306) (the first reading, withdrawn), [41547683034](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41547683034) (the rule at 949131ca against `main` before RE-301), [41550667212](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41550667212) (this change at 6689533d against `main` at a8ad9dd5, with RE-301, `--saved-meshes`, `mesh_bodies_vs_ifc`)
**Result:** positive. The saved graphics class `GImposter` that stops 1,813 of the 3,020 saved meshes RE-224 keeps as boxes on the office, RE1 and the 2024 samples is a light source. It sits in a filter of its own, with the shape and lines that draw the light. Leaving that filter out of the body draws the 43 office and 2 RE1 Electrical bodies it kept as boxes, all exact to 0.001 ft against Revit's export, and 417 more of Snowdon Towers electrical's bodies.

## 1. What a `GImposter` holds

The probe reads every saved graphics record of the office, RE1 Electrical and the ten 2024 samples: 281 `GImposter` objects in all (3 on the office, 2 on RE1 Electrical, 276 on six of the samples; 100 of them on Snowdon Towers electrical, on 85 symbols that its 511 light fixtures share).

- Every one has the same three fields: `m_GInfo` (flags 557060; category −1 on every one printed in full), `m_trf` (a transform) and `m_oAsset`.
- `m_oAsset` points to an `Asset` of scalar properties and nothing else within 8 pointer steps: `APropertyBoolean`, `APropertyFloat`, `APropertyEnum`, `APropertyDouble1`, `APropertyDouble3`, `APropertyString`, `APropertyDistance`, `APropertyInteger`. This is a rendering asset; no `Geometry` or `GPolyMesh` lies under it.
- Every one's parent is a `GFilter` with the same flags, 557060. Beside the `GImposter`, that filter holds a `Geometry` on every file, often curves (`GLine`, `GArc`, `GPoint`, `GNurbSpline`), and on 164 of them a `GPolyMesh`; the 35 the probe prints in full all have `m_normalsFlag` 2 and one normal per facet. That `GPolyMesh` is where RE-224's "unsupported normal binding" came from: it appears only beside a `GImposter`.

So the filter is the light source: its asset, and the shape and lines that draw it in a view.

## 2. The first reading, withdrawn

The first change (1123e476) skipped only the `GImposter` and read the binding-2 `GPolyMesh`. Measure 41544207306 drew 39 more office bodies, none of them Revit's. They were 15 "LEX Exit Sign" fixtures (and the rest of the same families) at 11.7 to 12.2 times Revit's volume, with Revit's every vertex on rvt-rs's surface (within 2.5e-6 ft) and rvt-rs's furthest 0.44 ft from Revit's. The light's shape had been drawn with the fixture. It is also why RE-224 found the office's 43 `GImposter` meshes all inexact: drawn partly, they still held that shape.

## 3. What rvt-rs does

`native_graphics_traversal` leaves out a `GFilter` that holds a `GImposter` and otherwise only leaves (`Geometry`, `GPolyMesh`, curves), counting it as a branch with no surface, as it counts a filter of curves; all 281 observed filters are such. A filter with any other child, and a `GImposter` anywhere else, are walked as before: the `GImposter` is recorded as a decode failure, and the element keeps its box. The `GPolyMesh` normal binding is not read: it is only met inside these filters.

## 4. Measured

Measure 41550667212, this change (6689533d) against `main` at a8ad9dd5, which reads left-handed cylinders (RE-301), both with `--saved-meshes`:

| file | bodies drawn from saved meshes, before → after | kept as boxes, before → after |
|---|---|---|
| BIM4LCA office as built (2024) | 10,881 → 10,924 | 322 → 279 |
| BIM4LCA office at permit (2024) | 10,998 → 11,041 | 322 → 279 |
| RE1 Electrical (2025) | 45 → 47 | 4 → 2 |
| Snowdon Towers electrical (2024 sample) | 907 → 1,324 | 556 → 139 |
| Snowdon Towers architectural (2024 sample) | 1,971 → 2,303 | 646 → 314 |
| rme basic and advanced (2024 samples) | 5,096 → 5,506 and 5,099 → 5,509 | 551 → 141 |
| rac advanced and basic (2024 samples) | 4,935 → 4,971 and 284 → 294 | 69 → 33 and 49 → 39 |
| Core Interior, RE1 Architecture, Mechanical, Plumbing, the other samples | unchanged | unchanged |

Against Revit's own export (`mesh_bodies_vs_ifc`):

| file | scored, before → after | exact (0.001 ft) | within 0.01 ft | volume within 1% |
|---|---|---|---|---|
| office as built | 1,452 → 1,495 | 1,360 → 1,403 | 1,438 → 1,481 | 1,434 → 1,477 |
| office at permit | 1,540 → 1,583 | 1,444 → 1,487 | 1,522 → 1,565 | 1,521 → 1,564 |
| RE1 Electrical | 45 → 47 | 29 → 31 | 45 → 47 | 45 → 47 |

Every new body scored is exact; they are the office's "LEX Exit Sign" fixtures (wall and ceiling mounted) and RE1 Electrical's `265100-LIGHT-LED`. Measured before RE-301 (Measure 41547683034, against a2cec9dc), the same rule drew 39 more office bodies, all exact. The IFC4 schema check changes only in its count of swept solids (the boxes): 2,404 to 2,361 on the office. Snowdon Towers electrical's `--saved-meshes` export now passes 30 MB, so IfcOpenShell's full validation skips it, as it does any export over that size. The meshes cost size: the office as built goes from 447,784 to 473,681 triangles, Snowdon Towers electrical from 41,376 to 815,408.

The samples have no Revit export, so their new bodies are not scored. That they are light fixtures whose light source is now left out is consistent with their counts, not measured: on no file do the new bodies outnumber its `IfcLightFixture`s.

## 5. Not claimed

- The `GPolyMesh` normal binding 2 elsewhere than in a light source's filter: not met on these files, and not read.
- The 139 Snowdon Towers electrical bodies still kept as boxes, and the other kinds of RE-301 §1 (non-rectangular cylinder trims, `GEllipse` profiles, `GBiFlipControl`).

## 6. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=6689533d -f base=a8ad9dd5 -f ifc_flags=--saved-meshes \
  -f nordic=true -f samples=true -f sample_years="2024 2024" \
  -f models="core-interior re1-* nordic-office-* sample-2024-*" -f scorers="mesh_bodies_vs_ifc schema"
```

The probe: `-f probe=probe_re300_graphics_imposters -f scorers=none` on the same models, at this change.
