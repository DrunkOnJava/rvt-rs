# RE-257 — A wall's Height

**Date:** 2026-10-27
**Issues:** #1016 (quantities), W7 of [`docs/release-1.0-criteria.md`](../../docs/release-1.0-criteria.md), #1067 (walls drawn taller than Revit's body); follows RE-251 (a wall's record keeps none of Revit's wall quantities; its saved Unconnected Height is Revit's `Height` on 496 of 579 office walls) and RE-228 (walls whose tops join a roof or floor)
**Artefacts:** the BIM4LCA office as built and at building permit (Revit 2024, Revit 24.0.5's IFC4 exports) and puzzbobb's R2025 models (Revit 2025, Apache-2.0, #356), as Measure fetches them
**Runs:** Measure (`examples/probe_re251_wall_numbers.rs` with `tools/re/wall_height_vs_qtos.py`) [40490349469](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40490349469) (office and R2025) and [40501460421](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40501460421) (office, with Revit's own bodies)
**Result:** negative for a rule, positive for what the value is. Revit's `Qto_WallBaseQuantities.Height` is the vertical span of Revit's own body on every wall measured. rvt-rs's wall body, as tall as the wall's record box, is that tall on 501 of the office's 579 walls as built and on all 117 R2025 walls. The other 78 are drawn taller than Revit's body (#1067). Nothing read tells them apart, so no wall gets a `Height`.

## 1. What Revit writes

revit-ifc's `WallExporter` (CLEANROOM.md source 5, `ExportWallBase`) sets a wall's `Height` to the span in z of the wall's solids, or of its bounding box where it has meshes. Where walls are split by level, it clips that span to the level's range. The office's exports write each wall once (no wall Tag appears twice among their `IfcWall`s), so no wall is split there.

## 2. The two readings

The probe now prints, beside every number of each wall's native record and saved parameters, the depth of the extrusion rvt-rs draws the wall as (`B.depth`). The scorer sets that depth and the saved Unconnected Height (-1001101) against Revit's `Height`, in feet, to a relative 1e-6:

| | as built | at permit |
|---|---:|---:|
| walls rvt-rs writes | 607 | 589 |
| Revit writes a `Height` on | 579 | 565 |
| depth and -1001101 agree, and are Revit's | 496 | 478 |
| depth and -1001101 agree, and are not Revit's | 77 | 82 |
| they differ, and the depth is Revit's | 5 | 3 |
| they differ, and neither is Revit's | 1 | 2 |
| rvt-rs's depth is Revit's | 501 | 481 |

On the 23 R2025 models, the depth is Revit's on all 117 walls. -1001101 is Revit's on 85 of them; the other 32 are walls whose tops are attached to a roof, where the record box follows the roof and the parameter does not.

Requiring the two readings to agree, as RE-252 does for a wall's `Length`, does not narrow the office's misses: 77 of its 78 have the two agreeing.

## 3. The walls rvt-rs draws too tall

For each office wall whose `Height` rvt-rs's depth does not give, the scorer takes the vertical span of Revit's own body from IfcOpenShell's geometry:

| | as built | at permit |
|---|---:|---:|
| walls | 78 | 84 |
| Revit's body exactly as tall as its `Height` | 78 | 84 |
| ... drawn as `SweptSolid` | 32 | 33 |
| ... drawn as `Tessellation` | 46 | 51 |

So Revit's `Height` is right, and rvt-rs's body is too tall: 19.587 ft against 10.344 on four walls (1632093 and three others, a plain extrusion in Revit's export), and 14.173 against 11.253 on three (1457513, 1536395, 1656495, meshed). The record box and the saved Unconnected Height both describe the uncut wall.

## 4. What does not tell them apart

Ranking every field of the native record and every saved parameter's value against whether the depth is Revit's `Height` finds no clean separation. The best single value is the roof join's `JoinToRoofGStep.m_leadingIdx = -1`: it is on 61 of the 78 walls, but also on 30 of the walls whose depth is right. Its presence alone is on 77 of the 78 and on 383 of the 501. RE-251's §2 found the same field the best, and not exact, for -1001101. Of the 28 walls Revit gives no `Height` as built, 20 have the two readings agreeing and 8 not.

## 5. What rvt-rs does

Nothing changes: no wall gets a `Height`. Writing the depth would be wrong on 78 walls as built and 84 at permit. #1067 tracks reading what cuts these walls' solids, which would make both the body and its `Height` Revit's.

## 6. Reproduce

```bash
gh workflow run measure.yml --ref re/wall-height -f ref=8904889d80fb22a1c8a37e0eb59e62a319ac5f73 -f base=none -f nordic=true \
  -f models="nordic-office-*" -f scorers=none \
  -f probe=probe_re251_wall_numbers -f probe_scorer=wall_height_vs_qtos
gh workflow run measure.yml --ref re/wall-height -f ref=efc4fcb8ae18ed8924225a8d351f184f50ba3391 -f base=none -f nordic=true -f r2025=true \
  -f models="nordic-office-* r2025-*" -f scorers=none \
  -f probe=probe_re251_wall_numbers -f probe_scorer=wall_height_vs_qtos
```

Once this lands, `--ref main` with the merge commit as `ref` runs the same.
