# RE-238 — A slab's `PitchAngle`

**Date:** 2026-10-26
**Issues:** #35 (parameters), W7 of [`docs/release-1.0-criteria.md`](../../docs/release-1.0-criteria.md)
**Artefacts:** the BIM4LCA office as built and at building permit (Revit 2024, Revit's IFC4 exports) and RE1 Architecture (Revit 2025, Revit's IFC2X3 export), as Measure fetches them
**Runs:** Measure [40264178401](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40264178401) (`tools/re/slab_pitch_list.py`), [40263066749](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40263066749) (a first rule, §3) and [40269902756](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40269902756) (this change against `main`)
**Result:** positive where a slab's top is known to be level; a sloped slab is left out.

## 1. Revit's values

revit-ifc's `PitchAngleCalculator` is its `SlopeCalculator` (CLEANROOM.md source 5). A slab has no beam ends, no ramp slope and no location curve, so unless a parameter named `Slope` gives it, its pitch is the slope of the largest upward face of its geometry. On the office Revit's export writes `Pset_SlabCommon.PitchAngle` on 62 slabs: 0 on 54 (its 33 level floors and roofs and its 21 monolithic stair landings), and 1.9 to 4.4 degrees on four edge curbs (`Floor:AP-3.Reunakoroke`) and 21.6 to 24.0 degrees on four foundation slabs (`Floor:AP-1`, `BASESLAB`).

## 2. What rvt-rs does

`add_level_slab_pitch` writes `PitchAngle` 0 on a slab whose top it knows to be level:

- a stair landing drawn from its top down its type's thickness (RE-214);
- a slab drawn as an extrusion whose depth equals its type's layers' total thickness, within 0.001 ft. A sloped or shape-edited slab's record box is deeper than its layers, so it is left out.

Any other slab gets nothing.

## 3. A rule from the body alone

A first rule wrote 0 on every slab drawn as an extrusion of its outline. It was right on 33 and wrong on the eight sloped slabs, whose record boxes rvt-rs draws level: their bodies are not their shape either. The layer-thickness test leaves those eight out.

`tools/re/psets_vs_ifc.py` now compares a plane angle in radians from each file's own angle unit; the office's export writes degrees.

## 4. Measured

`psets_vs_ifc`, this change against `main`:

| file | `PitchAngle` equal | different | missing |
|---|---:|---:|---:|
| office as built | 54 of 62 | 0 | 8 (the sloped slabs) |
| office at permit | 54 of 62 | 0 | 8 |
| RE1 Architecture | 2 of 2 | 0 | 0 |

Core Interior's 80 slabs carry it too; its reference export writes no property sets. Einhoven and the R2025 models export byte for byte as before.

## 5. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=main -f base=none -f nordic=true -f keep_ifc=true \
  -f probe=basics_inspect -f probe_scorer=slab_pitch_list
```
