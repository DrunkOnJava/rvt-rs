# RE-251 — Whether a wall's record keeps Revit's wall quantities

**Date:** 2026-10-26
**Issues:** #1016 (quantities), W7 of [`docs/release-1.0-criteria.md`](../../docs/release-1.0-criteria.md); follows RE-207 (saved parameters), RE-228 (walls joined to a roof or floor) and the wall quantities RE-247 leaves missing
**Artefacts:** the BIM4LCA office as built and at building permit (Revit 2024, Revit's IFC4 exports), as Measure fetches them
**Runs:** Measure [40382390875](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40382390875) and [40383222141](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40383222141) (`examples/probe_re251_wall_numbers.rs` with `tools/re/wall_numbers_vs_qtos.py`), on `main` after v0.9.0
**Result:** negative: a wall's native record and saved parameters hold none of Revit's wall areas and volumes; its `Height` is the saved parameter -1001101 on 496 of 579 walls, and nothing read tells the other 83 apart. Nothing is written.

## 1. What the record holds

The probe prints every number of every object of each wall's native record and of its saved parameters (RE-207), and the scorer looks among them for each of Revit's `Qto_WallBaseQuantities`, in feet, square feet or cubic feet:

| quantity | walls | best field equal |
|---|---:|---|
| `Length` | 607 | a face plane's envelope along the wall, 421 (the walls whose length is their location line's) |
| `Height` | 579 | saved parameter -1001101 and the face planes' envelope upward, 496 |
| `GrossSideArea`, `NetSideArea` | 579 | none |
| `GrossVolume` | 579 | none |
| `NetVolume` | 607 | none |
| `GrossFootprintArea` | 279 | none |
| `Width` | 607 | 67 (it is written from the type's thickness, #1025) |

At permit `Height` is -1001101 on 478 of 565 walls. Every wall saves -1001101, -1001105, -1001108, -1001109, -1001111, -1012100, -1012101, -1012828 and -1012829; none is an area or a volume, so Revit computes those when it exports and does not keep them.

## 2. What does not tell the 83

The scorer ranks every field against whether -1001101 is the wall's `Height`. The best is whether the wall's record holds a `JoinToRoofGStep` and its leading index: right on 529 of 579 (walls with none: 115 of 119 equal; leading index 0: 348 of 360; -1: 29 of 91). A wall whose top is attached to a roof or floor (RE-228) has the height its attachment gives, which the parameter does not always follow, and no field read says which.

## 3. What rvt-rs does

Nothing changes: a wall's `Height`, side areas and volumes stay unwritten. Writing -1001101 as the `Height` would be wrong on 83 walls as built and 87 at permit.

## 4. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=6a254a09bc832c537be2ad6c61c392923d47fcfd -f base=none -f nordic=true \
  -f models="nordic-office-*" -f scorers=none \
  -f probe=probe_re251_wall_numbers -f probe_scorer=wall_numbers_vs_qtos
```
