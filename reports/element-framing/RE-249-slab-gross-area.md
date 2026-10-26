# RE-249 — A floor's GrossArea

**Date:** 2026-10-26
**Issues:** #1016 (quantities), W7 of [`docs/release-1.0-criteria.md`](../../docs/release-1.0-criteria.md); follows RE-247 (base quantities a body gives) and RE-95, RE-96 and RE-151 (floor sketches)
**Artefacts:** the BIM4LCA office as built and at building permit (Revit 2024, Revit's IFC4 exports), Core Interior and RE1 Architecture (whose exports have no base quantities, RE-242) and puzzbobb's R2025 models, as Measure fetches them
**Runs:** Measure [40329358476](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40329358476) and [40330336007](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40330336007) (`tools/re/qto_candidates_list.py` by `BodySource`), [40372979423](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40372979423) (this change against `main`, the quantity census) and [40373985565](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40373985565) (unfiltered)
**Result:** positive: a floor drawn as one extrusion has Revit's `Qto_SlabBaseQuantities.GrossArea` as the area of the profile it extrudes, its outline less its holes; rvt-rs writes it there, none different.

## 1. Revit's value

`qto_candidates_list.py --by-source` finds Revit's `GrossArea` equal to the area of rvt-rs's slab profile on all 40 of the office's slabs drawn from their record box that Revit gives one, as built and at permit; the 21 stair landings drawn as their slab (RE-214) get only a `NetVolume` from Revit's export, and the one landing drawn as its tread (#824) none of these. The slab's other quantities are its body's on fewer: `NetVolume` 32 of 40, `Depth` and `GrossVolume` 22, `Perimeter` 9 (the record box of a sloped or shape-edited slab is deeper than its layers, RE-238).

## 2. What rvt-rs does

`add_slab_gross_area` writes a floor's `GrossArea`, the area of its body's profile (its rectangle, or its sketch outline less its holes), where it is drawn as one extrusion. A floor whose sketch is several loops is several elements sharing its `Tag` (#331), and is left out, as is a floor drawn by another solid and a stair landing's slab.

## 3. Measured

This change against `main` (run 40372979423, the quantity census):

| model | `GrossArea` equal | different | missing | Revit does not write |
|---|---:|---:|---:|---:|
| office as built | 31 of 42 | 0 | 11 | 0 |
| office at permit | 31 of 42 | 0 | 11 | 0 |

The 11 missing are 10 slabs drawn otherwise or in several pieces and one opening Revit's export gives a slab's set. The office's quantities equal go from 20,157 to 20,188 as built (20,065 to 20,096 at permit). Core Interior's 79 floors and RE1 Architecture's 2 carry it too, counted apart since those exports have no base quantities; no R2025 export moves. The unfiltered run (40373985565, every scorer) moves only the quantity census: the property census, IfcOpenShell's validation and every other output are as on `main`. Core Interior's witness observations gain 79 `IFCELEMENTQUANTITY`, `IFCQUANTITYAREA` and `IFCRELDEFINESBYPROPERTIES` each, and both verdicts replay PASS unchanged.

## 4. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=99883ac9a0d8afb5f9486e0b8d382257877c74fb -f base=main -f nordic=true -f r2025=true
```
