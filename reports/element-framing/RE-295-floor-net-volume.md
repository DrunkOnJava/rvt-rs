# RE-295 — A floor's NetVolume and Width

**Date:** 2026-10-30
**Issues:** #1016 (quantities), W7 of [`docs/release-1.0-criteria.md`](../../docs/release-1.0-criteria.md); follows RE-249 (a floor's GrossArea), RE-238 (a level slab is its layers) and RE-225 (the cuts a hosted symbol makes)
**Artefacts:** the BIM4LCA office as built and at building permit (Revit 2024, Revit 24.0.5's IFC4 exports), Core Interior and RE1 Architecture (whose exports have base quantities off, RE-242) and puzzbobb's R2025 models, as Measure fetches them
**Runs:** Measure [41138429994](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41138429994) (`probes/examples/probe_re295_slab_records.rs`), [41139584512](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41139584512) (the quantity census) and [41140153113](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41140153113) (this change against `main`, unfiltered)
**Result:** positive, narrowed: a floor's `Qto_SlabBaseQuantities.NetVolume` is its `GrossArea` times its type's layers' total thickness where nothing cuts it, and a compound floor's `Width` is that thickness; both are written, none different. A single-layer floor's `Width`, the volumes of the stair landings Revit draws as a mesh and of the roofs written as slabs stay unwritten.
**Update (RE-294, 2026-10-31):** The compound floor `Width` rule is withdrawn: Revit 2024's exporter writes it and a Revit 2026 export of a held-out model does not, and where Revit's exporters disagree rvt-rs leaves a value out. The `NetVolume` stands. See [RE-294](RE-294-slab-covering-width-exporters-disagree.md).


## 1. Revit's values

revit-ifc's `NetVolumeCalculator` (CLEANROOM.md source 5, read at v24.2.0; the office's exporter is 24.0.5.432) takes a slab's `NetVolume` from a parameter, else from the volume of the solids and meshes of the element's geometry. On the 31 floors of the office that rvt-rs writes a `GrossArea` on and that nothing cuts, it is that `GrossArea` times the total thickness of the floor's type's layers:

| floors | as built | `NetVolume` = `GrossArea` × the layers' total thickness |
|---|---:|---:|
| `VP-1` and `VP-2` (four layers, 0.45 and 0.70 m) | 6 | 6 |
| `AP-1` base slabs (three layers, 0.28 m), level | 4 | 4 |
| `AP-1` base slabs, sloped | 4 | 4 |
| `AP-3` gutter floors (one layer, 0.05 m), 4 of them sloped | 6 | 6 |
| `TK-1`, `VP-3` and `AP-2` landings (one layer) | 11 | 11 |

The sloped floors are the 8 (4 `AP-1`, 4 `AP-3`) whose record boxes rvt-rs draws level (RE-238; Revit gives them 1.9 to 24 degrees). For them Revit's value is still the area it writes as `GrossArea` times the thickness as the type states it, and its `Depth` is that thickness times the cosine of the slope (0.2603 m for the 0.28 m base slab whose extrusion direction has a vertical component of 0.93). At building permit the same 31 floors hold.

`Width` is that thickness on all 14 floors of more than one layer (`VP-1`, `VP-2` and `AP-1`): 0.45, 0.70 and 0.28 m. On a floor of one layer it is the profile's extent along the extrusion's Y axis (a landing of 6.025 by 2.76 m has `Width` 6.025 and `Length` 2.7605, a floor of 1.5 by 6.47 m has `Width` 1.5), and some floors with the same kind of profile get none: the rectangular `AP-2` landing 1440353 has neither as built and both at permit.

## 2. What cuts a floor

The roof `YP-1` is the one slab of the office whose `NetVolume` is not its profile's area times its depth: 596.788 m3 against 598.189 m3 as built (594.948 and 596.345 at permit). Its native record holds four `WallCutoutGStep`s, the cuts of the four hatches RE-225 reads, and it is the only slab record that does; no slab record holds a `CutPairGStep`. The floors joined to walls hold `CleanPairGStep`s, and their volumes are their areas times their thickness: a `CleanPairGStep` does not cut the floor.

`probes/examples/probe_re295_slab_records.rs` lists the steps of every slab's record. The `CutPairGStep`, the join that cuts one element with another, marks a cut ceiling: the 2 ceilings as built and the 9 at permit whose `GrossArea` RE-250 found 0.7 to 1.1 percent short all hold one (Measure [41138942086](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41138942086)). It is read on floors too.

## 3. What rvt-rs does

`attach_slab_uncut` marks each floor whose native record was read and holds neither a `CutPairGStep` nor a `WallCutoutGStep` (`m_slab_uncut`). `attach_slab_layers` gives each floor its type's total thickness and its layer count (`m_slab_type_thickness`, `m_slab_type_layer_count`) whether or not its box is that deep, which a sloped floor's is not. `add_slab_gross_area` writes `NetVolume` as `GrossArea` times that thickness on a floor so marked, and `Width` as that thickness where the type has more than one layer. On a release the native record path does not read nothing is marked and no `NetVolume` is written.

## 4. Measured

This change against `main` (Measure 41140153113, unfiltered, `-f nordic=true -f r2025=true`, every scorer):

| model | `NetVolume` equal | `Width` equal | different | Revit does not write |
|---|---:|---:|---:|---:|
| office as built | 0 → 31 of 63 | 0 → 14 of 34 | 0 | 0 |
| office at permit | 0 → 31 of 63 | 0 → 14 of 35 | 0 | 0 |

The office as built goes from 28,020 to 28,065 of the 39,388 quantities equal and at permit from 27,966 to 28,011 of 39,679, with the mullion's one difference and none Revit's export leaves out. Core Interior's 79 floors carry a `NetVolume` and one a `Width`, and RE1 Architecture's 2 floors carry both; their exports have no base quantities, so the census counts them apart (2,188 → 2,268 and 82 → 86). Core Interior's witness observations gain 79 `IFCQUANTITYVOLUME` and 1 `IFCQUANTITYLENGTH`, and both verdicts replay PASS unchanged. No R2025 export moves, and every other scorer's output (properties, geometry, validation) is as on `main`. The run is of the commit before the cut marker was made to read ceilings and roofs too; Measure [41141311742](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41141311742) at the commit after it gives the same census.

## 5. What stays open

- The 21 stair landings Revit draws as a mesh (`300 mm Thickness`, 11 or 12 faces): their `NetVolume` is 0.5028 to 0.5121 m3, where rvt-rs's body, its slab (RE-214), has 0.5268 or 0.5307. The stair's runs cut them.
- The 10 roofs written as slabs (`ProfileRoof`) and the one `Non-Monolithic Landing`: rvt-rs writes none of their `Qto_SlabBaseQuantities`, `GrossArea` included. Revit's `NetVolume` is their profile's area times their depth on 9 of the 10 roofs; the tenth is `YP-1`, cut.
- A single-layer floor's `Width` (20 as built, roofs included): Revit's value is the profile's extent along the extrusion's Y axis, which turns with the plane Revit picks for each floor.
- A compound floor's `Depth` and `GrossVolume` (the 10 level ones): Revit's `Depth` is one layer's thickness, the last of the type's layers in the order rvt-rs reads them (0.32 of 0.45 m, 0.57 of 0.70 m, 0.1 of 0.28 m), and `GrossVolume` is the area times it.

## 6. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=eb191f78ae5bd1ca3511fdc865a470acef7b17d7 -f base=main -f nordic=true -f r2025=true
```
