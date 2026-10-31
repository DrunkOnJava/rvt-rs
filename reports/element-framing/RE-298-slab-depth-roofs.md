# RE-298 — A flat roof's slab quantities, and a level floor's Depth

**Date:** 2026-10-30
**Issues:** #1016 (quantities), W7 of [`docs/release-1.0-criteria.md`](../../docs/release-1.0-criteria.md); follows RE-249 (a floor's GrossArea), RE-295 (a floor's NetVolume and Width) and RE-238 (a level slab is its layers)
**Artefacts:** the BIM4LCA office as built and at building permit (Revit 2024, Revit 24.0.5's IFC4 exports), Core Interior and RE1 Architecture (whose exports have base quantities off, RE-242) and puzzbobb's R2025 models, as Measure fetches them
**Runs:** Measure [41138429994](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41138429994) (`probes/examples/probe_re295_slab_records.rs`), [41141494766](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41141494766) (the quantity census), [41181103022](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41181103022) (the first version, against `main`, with the held-out models) and [41200434886](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41200434886) (this change, unfiltered, with the held-out models)
**Result:** positive, narrowed: a slab's `GrossVolume` is its `GrossArea` times its `Depth` on all 42 of the office's slabs that have one. On a flat roof written as a slab, `Depth` is the layers' total, and rvt-rs writes it with the roof's `GrossArea`, `GrossVolume` and `NetVolume`, none different. On a level floor of several layers the office's export takes the last layer's thickness, but Revit 26.4's export of the held-out Revit 2026 house writes another value, so rvt-rs writes no `Depth` or `GrossVolume` there (§3). A roof's `Width` stays unwritten.

## 1. Revit's values

| slabs of the office | as built | at permit | `Depth` |
|---|---:|---:|---|
| level floors of one layer (`TK-1`, `VP-3`, `AP-3` ×2, `AP-2` landings) | 13 | 13 | the layer = the extrusion (RE-249) |
| level floors of several layers (`VP-1`, `VP-2`, `AP-1` flat) | 10 | 10 | the last layer: 0.32 of 0.02, 0.08, 0.03 and 0.32 m; 0.57 of 0.02, 0.08, 0.03 and 0.57 m; 0.1 of 0.08, 0.1 and 0.1 m |
| flat roofs written as slabs (`ProfileRoof`) | 10 | 10 | the layers' total = the extrusion: 0.72 m of 0.055, 0.4 and 0.265 m (`YP-3`), 0.89 m of five layers (`YP-2`), 0.973 m of ten (`YP-1`) |
| sloped floors (`AP-1` ×4, `AP-3` ×4) and the `Non-Monolithic Landing` | 9 | 9 | not a layer: the thickness times the cosine of the slope, which rvt-rs draws level (RE-238) |

`GrossVolume` is `GrossArea` times `Depth` on all 42, sloped floors included. A roof's `GrossArea` is the area of the profile its sketch gives on all 10 (the 163-point `KA-1` to 5e-6), and its `NetVolume` is that times the depth on 9: the tenth, `YP-1`, is 596.788 m3 against 598.189 m3 as built and its native record holds four `WallCutoutGStep`s, the record of RE-295 §2.

revit-ifc's `RoofExporter.ExportRoofOrFloorAsContainer` (CLEANROOM.md source 5, read at v24.2.0) reuses one creation-data object while it loops over a floor's subcomponents, so a floor of several of them reads as the last one's depth, which is how the office's compound floors read; a roof goes through the extrusion analyzer whole. That is a reading of why, not the rule: the rule is the 10 and the 10.

## 2. What rvt-rs does

`add_slab_gross_area` writes `Depth` and `GrossVolume` on a level floor of one layer, as before (RE-249), and on a roof written as a slab (`IFCSLAB`), drawn as one extrusion of its sketch's outline, level and with its layers adding up to its depth: `GrossArea`, `Depth` as the extrusion's, `GrossVolume` and, when the record holds no cutting step (`m_slab_uncut`, RE-295), `NetVolume` as the area times the depth. A roof that Revit writes as an `IfcRoof` is not a slab and gets none.

## 3. Measured

A first version also wrote a level floor of several layers its last layer's thickness as `Depth`, and `GrossVolume` from it: Revit's value on all 10 such floors of the office, as built and at permit. Its run with the held-out models ([41181103022](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41181103022)) found that value different from Revit's on 2 slabs of the held-out Revit 2026 house, whose export is Revit 26.4's: `Depth` 0.018 ft larger, and the `GrossVolume` from it. Nothing in a `.rvt` says which exporter will read it, and the maintainer's standing rule (2026-10-31) is that a value Revit's exporters disagree on is not written. The floors of several layers therefore get neither, and the house is no longer a held-out test of a slab's `Depth`.

This change against `main` (Measure [41200434886](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41200434886), unfiltered, `-f nordic=true -f r2025=true -f holdout=true`):

| model | `Depth` equal | `GrossVolume` equal | `GrossArea` equal | `NetVolume` equal | different |
|---|---:|---:|---:|---:|---:|
| office as built | 13 → 23 of 42 | 13 → 23 of 42 | 31 → 41 of 42 | 31 → 40 of 63 | 0 |
| office at permit | 13 → 23 of 42 | 13 → 23 of 42 | 31 → 41 of 42 | 31 → 39 of 63 | 0 |

The quantity census goes from 28,329 to 28,368 equal as built and from 28,254 to 28,292 at permit. The one different value (a mullion's) is unchanged, and so is "rvt-rs quantities Revit does not write", 0. Only the two office exports move. The R2025 models, Core Interior, RE1 and the three held-out models are byte-identical, and the held-out census is unchanged.

## 4. What stays open

- A roof's `Width` (and `Length`): its profile's extent, 12.35 m for `YP-3` of three layers.
- The 10 level floors of several layers' `Depth` and `GrossVolume` (§3).
- The 8 sloped floors' `Depth`, and the single `Non-Monolithic Landing`'s slab quantities.
- The 21 stair landings drawn as a mesh (RE-295 §5).

## 5. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=b0a1d997911d1633337f61d89e7e13dfd2b77c23 -f base=6aaef6aa089a6bc0cdb1092255ac95ad58cc2871 -f nordic=true -f r2025=true -f holdout=true
```
