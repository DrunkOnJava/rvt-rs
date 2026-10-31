# RE-250 — A ceiling's Width

**Date:** 2026-10-26
**Issues:** #1016 (quantities), W7 of [`docs/release-1.0-criteria.md`](../../docs/release-1.0-criteria.md); follows RE-57 (ceilings' layers), RE-238 (a slab whose body is its layers) and RE-249 (a floor's GrossArea)
**Artefacts:** the BIM4LCA office as built and at building permit (Revit 2024, Revit's IFC4 exports), RE1 Architecture (whose export has no base quantities, RE-242) and puzzbobb's R2025 models, as Measure fetches them
**Runs:** Measure [40376669754](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40376669754) (`tools/re/qto_candidates_list.py --misses`), [40377441925](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40377441925) (`tools/re/covering_qtos_list.py`), [40378309356](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40378309356) (`Width` and `GrossArea` against `main`) and [40379026534](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40379026534) (`Width` alone, unfiltered)
**Result:** positive, narrowed: a compound ceiling's `Qto_CoveringBaseQuantities.Width` is its type's layers' thickness, which rvt-rs's body depth is wherever the two agree; its `GrossArea` is not its sketch's area on every ceiling, and is left out.
**Update (RE-294, 2026-10-31):** The ceiling `Width` rule is withdrawn: Revit 2024's exporter writes it and the exporters of later releases do not, and where Revit's exporters disagree rvt-rs leaves a value out. See [RE-294](RE-294-slab-covering-width-exporters-disagree.md).


## 1. Revit's values

`covering_qtos_list.py` tallies each covering of the office by Revit's predefined type and type name against rvt-rs's body:

| coverings | as built | `Width` = body depth | `GrossArea` = profile area |
|---|---:|---:|---:|
| compound ceilings drawn as their record box | 78 | 78 | 57 |
| ceilings of a basic wall type (`AK-1.otsa`), drawn from wall data | 21 | 0 | 0 |
| roofing coverings (rooftop ventilation units) | 12 | 0 | 12 |

At permit the 75 compound ceilings have `Width` equal to the body's depth on all 75. Their depth is the type's thickness (0.052 m on most).

## 2. What rvt-rs does

`add_ceiling_width` writes a ceiling's `Width` (class `Ceiling`, drawn as one extrusion) where the extrusion's depth is its type's stacked layers' total thickness, the test RE-238 applies to a level slab, so a sloped ceiling, whose record box is deeper, gets none. Its `GrossArea` is not written: written where the ceiling's sketch gives its profile, it was Revit's on 57 as built and 46 at permit but differed on 2 and 9 (the sketch's area 0.7 to 1.1% larger than Revit's, run 40378309356), and no field told those apart yet.

## 3. Measured

This change against `main` (run 40379026534, `-f nordic=true -f r2025=true`, every scorer):

| model | `Width` equal | different | missing | Revit does not write |
|---|---:|---:|---:|---:|
| office as built | 59 of 111 | 0 | 52 | 0 |
| office at permit | 55 of 75 | 0 | 20 | 0 |

The office's quantities equal go from 20,188 to 20,247 as built (20,096 to 20,151 at permit). The 52 missing as built are the 19 compound ceilings whose layers are not read or do not sum to their body's depth, the 21 wall-type ceilings and the 12 roofing coverings. RE1 Architecture's 6 ceilings carry it too, counted apart since that export has no base quantities. The property census, IfcOpenShell's validation and every other output are as on `main`; Core Interior's export does not move.

## 4. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=2ee27bb22e24f77524ec413ee4a6e32ad8cda867 -f base=main -f nordic=true -f r2025=true
```
