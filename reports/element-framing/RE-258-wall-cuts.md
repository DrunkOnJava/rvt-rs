# RE-258 — Where Revit cuts a wall's top and base

**Date:** 2026-10-27
**Issues:** #1067 (walls drawn taller than Revit's body), #1016 (W7: walls' `Height`), W6 of [`docs/release-1.0-criteria.md`](../../docs/release-1.0-criteria.md); follows RE-257 (a wall's `Height` is its body's span) and RE-228 (`JoinToRoofGStep.m_baseOrTop`)
**Artefacts:** the BIM4LCA office as built and at building permit (Revit 2024, Revit 24.0.5's IFC4 exports), Core Interior (Revit 2024), RE1 Architecture and puzzbobb's R2025 models (Revit 2025), as Measure fetches them
**Runs:** Measure (`tools/re/wall_heights_vs_ifc.py`, with `examples/probe_re251_wall_numbers.rs` for the walls' records) [40509396187](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40509396187) (every model), [40515952214](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40515952214) (the floors' own heights) and [40523970600](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40523970600) (both offices, both files' floors sampled over each wall); [40556720844](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40556720844) re-ran every model with the scorer taking the pieces an exporter writes under one Tag as one body, and [40720727607](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40720727607) with it meshing only each product's Body representation, as it lands: its output is byte-identical to 40556720844's on every model, nothing is left out, and every count here is unchanged
**Update (2026-10-30):** RE-267 explains the 5 walls per stage that §3 could not: Revit cuts them over the part of their plan the floor covers, and the records and floors are not the same in the two files. rvt-rs now cuts the walls a named floor covers whole (RE-267 §7); §5 below is as it stood.
**Result:** positive for the top, narrowed, not shipped; negative for the base. Revit cuts a wall's top at the underside of a floor its top join names (`JoinToRoofGStep` with `m_baseOrTop` 1). That gives 63 of the office's 65 top cuts as built and 68 of 70 at permit, but it also cuts 5 walls at each stage that Revit leaves whole, and nothing in either file tells those apart. No wall is cut yet.

## 1. Which end moves

`wall_heights_vs_ifc.py` meshes every `IfcWall` in rvt-rs's IFC and in Revit's with IfcOpenShell and compares each body's lowest and highest point by Tag. The two files' heights agree; their plans differ by a turn of 40.33 degrees and a shift, which the scorer fits to the walls' middles (residual 0.016 ft as built, 0.011 ft at permit).

| Model | Walls | Base and top Revit's | Only the base Revit's | Only the top Revit's |
|---|---:|---:|---:|---:|
| office as built | 607 | 522 | 65 | 20 |
| office at permit | 589 | 495 | 70 | 24 |
| Core Interior | 360 | 360 | 0 | 0 |
| R2025 (22 models) | 117 | 117 | 0 | 0 |

No wall has both ends wrong, and every miss is in Revit's storey: these are cut walls, not misplaced ones. The floors, roofs and footings rvt-rs draws stand where Revit's do: 42 of the office's 63 at base and top, the other 21 at their top, their base within 0.02 to 0.28 ft. Sampled over each wall's middle, Revit's floors and roofs have rvt-rs's underside and top on 1,006 of 1,018 samples as built (974 of 986 at permit). The other 12 are footings that rvt-rs draws as their box and Revit draws sloped (§4).

## 2. The top: the floor the wall's top join names

A wall's native record holds a `JoinToRoofGStep` for each end joined to a roof or floor. Its `m_baseOrTop` is 1 for the top (RE-228) and its `m_ids` name the elements. Take a wall's top as the lowest underside, over the wall's middle, of a floor or roof its top join names, wherever that underside lies between the wall's base and top:

| | as built | at permit |
|---|---:|---:|
| tops Revit cuts | 65 | 70 |
| ... cut there by the rule | 63 | 68 |
| ... cut by an element no top join names | 2 | 2 |
| tops Revit leaves | 542 | 519 |
| ... left by the rule | 537 | 514 |
| ... cut by the rule | 5 | 5 |

Every floor involved is level, with the same underside over the wall in both files. A plan box is not enough: floors whose box overlaps a wall without covering its middle are beside it, as a roof slab is beside the parapets along its edge. The rule needs the floor's body over the wall's middle. The two walls cut with no top join (1746099 and 1795193) are cut by the floor that a Join Geometry pair (`CutPairGStep`) of their record names.

## 3. What the rule gets wrong

The five walls the rule cuts and Revit leaves are not explained by either file:

- 1630966 (both stages) and 2984235, 2984610 and 2984730 (at permit) rise above the joined floor's top (by 0.919 and 0.098 ft). But walls that rise 0.646 and 7.382 ft above theirs are cut, so how far a wall rises does not tell them apart.
- 1892039's top join also names a wall (3363820).
- 1457415 and 1892033 are joined to floors 1474132 and 1861193. Revit leaves them at 1474132's underside as built and cuts them at 1861193's at permit, with the same records and the same floors in both files.
- 3123954 (as built) is joined to floor 2765485, whose underside lies 0.656 ft below the wall's top over its middle, in both files.

The face history of the join (`m_faceHistTable`), its flags (761725 on all), `m_leadingIdx` (-1 on all), and the count of elements it names were compared between the walls the rule gets right and these: none separates them.

## 4. The base

The same rule for the base, using a base join (`m_baseOrTop` 0) and a floor's or footing's top, gives 4 of the 20 raised bases as built, 10 wrong, and 2 bases raised that Revit leaves. The joined elements are footings that rvt-rs draws as their box, 173.950 to 176.837 ft. Revit draws them sloped: over these walls' middles they run about 174.93 to 175.85 ft or 175.75 to 176.67 ft. Revit raises the walls on them to 174.869 or about 176.5 ft, and two not at all. So the base waits on the footings' own bodies. Six walls Revit raises by 8.202 ft (2098938, 2105250, 2177212, 2293116, 2294996, 3450274) have no base join, and no floor's top is there.

## 5. What rvt-rs does

Nothing changes. Cutting the tops by the rule would make 63 walls Revit's as built but 5 wrong, and a body that is wrong where it was a box is not an improvement under the fail-closed rule. #1067 stays open for what tells those walls apart. Since a wall's `Height` is its body's span (RE-257), it follows once the bodies are right. `wall_heights_vs_ifc.py` is the measurement for both.

## 6. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=5a81e74587c1cc8c71f8d2dacb9fada7baef88f4 -f base=none -f nordic=true \
  -f models="nordic-office-*" -f scorers=wall_heights_vs_ifc \
  -f probe=probe_re251_wall_numbers -f probe_scorer=-
```

The export and the probe are `main`'s as this lands (5a81e745), and the scorer comes from `main` once it has. The runs above measured the branch's commits, kept under `refs/archive/2026-10-27/re/wall-cuts` once it lands. The rule's tallies come from joining `probe.txt`'s `JoinToRoofGStep` objects (`m_baseOrTop`, `m_ids`) with the scorer's `wall` and `over` lines.
