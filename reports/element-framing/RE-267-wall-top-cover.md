# RE-267 — Revit cuts a wall's top only where the floor covers the wall

**Date:** 2026-10-30
**Issues:** #1067 (walls drawn taller than Revit's body), #1016 (W7: walls' `Height`), W6 of [`docs/release-1.0-criteria.md`](../../docs/release-1.0-criteria.md); corrects RE-258 §3 and follows RE-257 (a wall's `Height` is its body's span), RE-228 (`JoinToRoofGStep.m_baseOrTop`) and RE-266
**Artefacts:** the BIM4LCA office as built and at building permit (Revit 2024, Revit 24.0.5's IFC4 exports), Core Interior (Revit 2024), RE1 Architecture and puzzbobb's R2025 models (Revit 2025), as Measure fetches them
**Runs:** Measure [41075127575](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41075127575) (both offices: `probes/examples/probe_1067_native_dump.rs` and `tools/re/wall_top_cover_vs_ifc.py`), [41076305345](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41076305345) (every model, the scorer also printing each wall's horizontal top faces), [41077948567](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41077948567) (the probe on Core Interior, RE1 and the R2025 models) and [41079569865](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41079569865) (every model, the scorer also reading each floor's level across the wall); for the shipped rule [41092512565](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41092512565) (every model and scorer, against `main`), [41092527900](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41092527900) (`wall_top_cover_vs_ifc` on its export) and [41091288700](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41091288700) (both offices, with `Height` written on the cut walls)
**Result:** positive, shipped for the walls a named floor covers whole. The 5 walls per stage that RE-258 found Revit "leaves whole" are not whole: Revit cuts them at the floor's underside over the part of their plan the floor covers and leaves the rest at full height, so the highest point does not move. A rule that cuts a top only where the named floor covers the wall's whole plan gives 64 of the office's 65 cut tops as built and 70 of 70 at permit, and cuts no wall that Revit leaves whole; rvt-rs now draws those walls so, and no other wall's base or top moves on any model. One wall as built is cut by Revit although a sliver of its plan lies outside the floor, and nothing read explains it; the walls a floor covers in part need a stepped top. Both keep their full height.

## 1. What RE-258 measured, and what it missed

`wall_heights_vs_ifc.py` compares each wall's lowest and highest point with Revit's. A floor that covers only part of a wall's plan cannot move the highest point when the rest of the wall stays at full height, and the scorer then reads the wall as left whole. `wall_top_cover_vs_ifc.py` (new) meshes both bodies, maps Revit's into rvt-rs's frame with the same fit, reads the top at a station every 0.25 ft along the wall's long axis, and prints the plan area of each horizontal face in the upper half of the body:

| Revit's top along the wall's centre line | as built | at permit |
|---|---:|---:|
| full height at every station | 512 | 493 |
| one height below the full height | 72 | 74 |
| ... the highest point is lower too (RE-258's 65 and 70) | 65 | 70 |
| ... the highest point is not (a strip beside the centre line stays at full height) | 7 | 4 |
| stepped along the wall | 22 | 22 |
| no body on the centre line | 1 | 0 |

So 94 office walls as built and 96 at permit are cut somewhere on their centre line, not 65 and 70. A floor that stops short of a wall's end, or of one of its faces, cuts the wall where it covers it and leaves the stub at full height.

## 2. The top is the floor's underside where the floor covers, the wall's own top elsewhere

Take each wall with exactly one named floor whose underside is level over its centre line and between its base and top: 83 walls as built and 87 at permit. The plan area of Revit's horizontal face at the wall's full height equals the area of Revit's wall plan outside that floor's plan to 0.002 ft² on 79 and 83 of them. The others are 3 foundation walls per file beside the ground slab's edge (several have more than one height across their thickness, and one has a sliver under 0.001 ft wide), the wall in §5 as built, and at permit a roof-edge wall that differs by 0.018 ft². Along the centre line, taking the named floors' undersides where they cover and the wall's own top elsewhere, the prediction is Revit's height at every station on 591 of 607 walls as built and 575 of 589 at permit. The other 16 and 14 walls differ from it somewhere. Eight in each file have a top that slopes along their length (183.10 to 183.34 ft over the first 2 ft of one). As built, 2 are lower than predicted at the underside of a floor the scorer reads, at stations that floor does not reach (the larger, a foundation wall, over 19 ft of its length), and 6 at a height none of the floors, roofs and footings the scorer reads has. At permit, 5 are lower at such a height, and 1 is higher than predicted. These are measured and unexplained.

The walls RE-258 §3 could not explain, with what stays at full height in Revit's body:

| Wall | Floor | What the floor does not cover | Full-height area in Revit |
|---|---|---|---:|
| 1630966, both stages | the one its top join names | the last 1.25 ft of its centre line (of 40.2 ft), and 16% of its plan in all | 9.954 ft² as built, 9.819 at permit |
| 1457415 as built | 1861193 | a strip 0.033 ft wide along its outer face, 18 ft long | 0.580 |
| 1892033 as built | 1861193 | its end, 0.032 ft past the floor's edge | 0.021 |
| 3123954 as built | 2765485 | its start, 0.033 ft past the floor's edge | 0.013 |
| 1892039, both stages | 1474132 | a 3 to 4 ft stretch at one end, where the floor's outline steps back (a recess 4.6 ft wide) | 2.476 as built, 2.045 at permit |
| 2984235, 2984610 and 2984730 at permit | the roof slab over them | 44% of their plan (48% of 2984610's) | 18.475, 20.775 and 18.475 |

In each row the full-height area is the area of Revit's wall plan outside Revit's floor plan, to 0.002 ft² (2984610: 0.018). rvt-rs's plan against rvt-rs's floor gives the same where both walls have the same ends; it differs for 1457415 by 0.05 ft², and for 1892039 and 1630966 at permit, where rvt-rs's wall is longer. Revit's top over the rest of each wall is the floor's underside, level. The walls never were left whole: the part of the body the floor covers is cut.

1892039's top join names a wall (3363820) beside 1474132. That wall's base is at 1892039's top, and it is the only wall in either file whose top join names a wall. It explains nothing: the recess in 1474132's outline does.

## 3. The rule and its tallies

A wall's top is the underside of a floor, roof or footing that its top join (`JoinToRoofGStep` with `m_baseOrTop` 1) or one of its `CutPairGStep`s names, when that underside is level across the wall (the floor's lowest and highest underside over three rows of stations, on the centre line and 0.45 thicknesses off it on each side, within 0.001 ft), lies between the wall's base and top, and the floor's plan (rvt-rs's body) covers the whole of the wall's plan (the wall's plan outside it under 1e-6 ft²). The two walls RE-258 found cut with no top join, 1746099 and 1795193, are cut this way, by the floor a Join Geometry pair names. A wall's `CleanPairGStep` names the same floors as its `CutPairGStep` on every wall measured, and the tallies count either.

| | as built | at permit |
|---|---:|---:|
| tops Revit cuts (highest point) | 65 | 70 |
| ... cut by the rule, by the top join | 62 | 68 |
| ... cut by the rule, by a cut pair | 2 | 2 |
| ... not cut by the rule | 1 | 0 |
| walls the rule cuts, in all | 64 of 607 | 70 of 589 |
| ... that Revit leaves whole | 0 | 0 |

At permit the rule is exact. Every wall with a named floor whose plan lies wholly inside the floor's is cut, and every one with a part outside is not: the smallest area outside among the walls Revit does not cut is 2.05 ft² of Revit's plan (2.48 of rvt-rs's), and no wall it cuts has any.

The RE-258 rule, which cuts where the floor covers only part of the wall, is wrong on the 14 and 13 walls it selects that the new rule leaves: it draws the uncovered part too low.

The rule, top join and cut pair together, cuts none of Core Interior's 360 walls, RE1 Architecture's 7 or the 138 walls of the R2025 models. The across-plan level condition is what keeps it off the R2025 roofs: along the centre line alone, 2 walls in 3 models each (a slope-arrow roof in three, a hip roof in the other three) sit under a roof whose underside is level there but slopes across the wall's 0.656 ft thickness. Revit's body on those has no horizontal top face and its highest point is the record box's, so they are not cut.

## 4. What was compared and does not tell a cut wall from one with a part outside the floor

Among the 77 office walls as built (81 at permit) whose top join names a floor with its underside between the wall's base and top at some station of the centre line, 14 (13) of which Revit does not cut at the highest point:

- every saved parameter of the wall (top constraint, unconnected height, top offset, structural usage, room bounding and the rest) and every field of its root record (phases, design option, cross-section, flip, structural usage): the only values no cut wall shares are one per wall (its ElementId, a comment text);
- the top join's fields: `m_flags` 761725 and `m_version` 16 on all, `m_leadingIdx` -1, 0 or 1 in both groups, the count of named elements, and the lengths of its face and edge history tables, which take 12 and 8 different lengths (faces) and 21 and 14 (edges) in the two groups, with lengths in common;
- the floors' records: class (the office's "floors" are mostly `ProfileRoof` natively, with 5 or 6 `Floor`), room bounding, design option, phase, `m_analyzesAs`, and whether the floor's record lists the wall (it does for 1 and 2 walls);
- how far the wall rises above the floor.

RE-258 §3 compared the history tables from a probe that printed arrays of up to four numbers, so tables of more than four entries were absent from it; the lengths above come from the new probe. None of these separates the groups, and none needs to: the plan does.

The floors' fields above were compared as the probe read them up to dfdb1025, which read a `ProfileRoof` floor (most of the office's) only through its named-id pass and not as a slab, so those floors' saved parameters were not among them; 911674f4 reads them as slabs (174 more records and 177 more saved parameters on the office as built, Measure run [41102735279](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41102735279)). The same run re-scores every wall with level faces taken only where a triangle's corners lie within 0.001 ft in height: its output is byte-identical to run 41079569865's on both offices, so every count in this report stands. Whether a `ProfileRoof` floor's saved parameters separate the walls is not measured; the plan does.

## 5. The one wall

As built, one wall (5.0 ft long, 0.328 ft thick, its top joined to the same floor as the wall it abuts) ends 0.085 ft past the named floor's edge, inside the abutting wall's plan. rvt-rs's plan and Revit's agree (0.0243 ft² of it lies outside the floor in both), and Revit's body has one horizontal top face over its whole plan, at the floor's underside, stub included. At permit its end meets the abutting wall's face, the wall is covered, and it is cut. The three other slivers as built (0.032 to 0.033 ft) lie outside every wall and stay at full height. Why Revit cuts the stub is not read: the abutting wall lies wholly outside the floor and is not cut, and counting a part of a wall's plan that lies inside an abutting wall's plan as covered would make the rule exact as built, but it rests on this one wall.

## 6. The two-stage pair

RE-258 §3 said 1457415 and 1892033 have the same records and floors in both files. They do not.

- 1457415's location line moves 0.050 ft (its curve's origin 162.1997 to 162.2496 in x). Its body is 0.656 ft thick as built and 0.492 ft at permit, and its partners in `CleanPairGStep` and `CutPairGStep` are different elements.
- 1892033 has no `CleanPairGStep` or `CutPairGStep` as built and gains partners at permit.
- 3123954's curve ends 0.115 ft earlier at permit, and the history tables of its top join are 16 and 5 entries as built and 4 and 1 at permit.
- 1861193's plan is the same in both files; 2765485's edge beside the wall is at the same place.

The wall's end is 0.032 ft past the floor's edge as built and flush with it at permit, and that is the whole difference: a change in plan geometry, not a hidden flag.

## 7. What rvt-rs does

A wall's top is cut at the underside of the floor, roof or footing that its top join or one of its `CutPairGStep`s names, where all of this holds in rvt-rs's own export. The pass is `ifc::export_content::wall_tops`, run once every element is in the model, since the floor is another element; the ElementIds come from the wall's native record (`native_walls::WallRecords::top_cutters`), read in the pass that already reads `JoinToRoofGStep` for RE-228.

- The element is an `IfcSlab`, `IfcRoof` or `IfcFooting` drawn as a plain extrusion, or as several such pieces at one height. Its underside is then level across the wall, which is §3's condition by construction. A roof drawn from its slopes is a solid and is left out, so the R2025 roofs that slope across a wall's thickness cut nothing.
- The underside lies between the wall's base and top, 0.001 ft in.
- The element's outline covers the whole of the wall's plan, clear of the element's holes and of the openings that void it: the area of the wall's plan outside the outline and the area of it inside a hole or opening are each under 1e-4 ft². The areas are worked out on triangles of rvt-rs's own plans, the outline and each hole one by one, since a floor of nine holes did not triangulate whole (the storey slabs 1457693 and 1457785 as built). A hole that does not triangulate to its own area counts as its bounding box; an outline that does not leaves the wall as it was.

The wall is one plain extrusion with no hole in its plan and no edited elevation profile; a wall drawn as anything else keeps its body (17 of the office's walls as built that name a floor, none of which Revit cuts at its underside). Where several elements qualify the wall is cut at the lowest underside (no office wall has two at different undersides). The extrusion's height becomes the underside less the base; the placement and `BoundingBoxHeight` stay, and the wall carries `BodyHeightSource` `wall_top_cut_at_covering_floor` and `TopCoverElementId`.

The tolerance. §3 measures 1e-6 ft² on IfcOpenShell's meshes of the written IFC. In memory a covered wall measures at most 4.1e-6 ft² outside its floor (1727582 as built, against 1498471; every other covered wall under 1e-9), and the walls a floor covers in part at least 0.0127 (3123954), then 0.0210, 0.0243 (3004978, §5) and 0.0317, so 1e-4 keeps a factor of 24 above the one and 127 below the other.

Measured by Measure run [41092512565](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41092512565) against `main`, every model and scorer (the cover scorer on the shipped export is [41092527900](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41092527900)):

| BIM4LCA office | as built | at permit |
|---|---:|---:|
| walls with base and top Revit's, before | 522 | 495 |
| ... after | 586 | 565 |
| walls with base Revit's and top not, before | 65 | 70 |
| ... after | 1 | 0 |
| walls cut | 64 | 70 |

The one wall left is §5's. Exactly the cut walls' bodies change, their tops only (no base and no plan moves), and each lands on Revit's top. `ifc.sha256` changes on the two office models only: the exports of Core Interior, RE1 Architecture and the R2025 models are byte-identical, and on the offices every scorer's output is unchanged but `wall_heights_vs_ifc` and `frames_vs_ifc`, whose fit of rvt-rs's frame onto Revit's reads the walls' heights (as built, the vertical shift is 0.009 ft where it was -0.016 and the median residual 0.0096 ft where it was 0.0165; at permit 0.010 ft where it was -0.025 and 0.0111 ft where it was 0.0258).

On the shipped export `wall_top_cover_vs_ifc` ([41092527900](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41092527900)) reads rvt-rs's top along the whole wall equal to Revit's on 472 of 607 walls as built (416 before) and 419 of 589 at permit (367), and no wall that was equal differs now. The top of each of the 64 and 70 cut walls is Revit's height along its length; 56 and 52 of them are equal to the station, and the rest differ only where Revit's body has no point (a gap across it, or its end where the two plans differ). Of the 135 and 170 walls still differing, 128 and 167 have more than one run in Revit's top: a step or a gap.

Left as they were: the walls a floor covers in part, which need a stepped top that an extrusion cannot draw (22 office walls in each file are stepped along their centre line, and 7 and 4 more have a strip at full height beside it, §1: 29 as built and 26 at permit), and §5's wall.

`Height`. A cut wall's `Qto_WallBaseQuantities.Height` is the underside less its base. Written on all 64 and 70 it is Revit's on 62 and 66 and never different ([41091288700](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41091288700): 143 and 136 equal on the offices), but Revit's export writes none on the other 2 (1746099 and 1767978) and 4 (1488456, 1746099, 1795193 and 3363820), the walls it draws as a mesh, and no field read tells them from the rest: not the join steps (1767978 holds a top join and nothing else), the cut pairs, the cutouts (27 and 29 of the cut walls hold one and have a `Height`) or the classes of the partners. Written on all, rvt-rs would write 2 and 4 values Revit's export omits (the maintainer, 2026-10-26: no quantity Revit omits), so a cut wall gets no `Height`, as before.

Stepping the rest needs a body that is not one extrusion, which this export draws a wall as, and the wall's plan after its joins (rvt-rs's differs from Revit's by up to 1.3 ft² on walls of 4 to 13 ft²). `wall_top_cover_vs_ifc.py` is the measurement for each.

## 8. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=dfdb1025159243f41ae876b731f2912dae22ea98 -f base=none -f nordic=true -f r2025=true \
  -f scorers=wall_top_cover_vs_ifc -f probe=probe_1067_native_dump -f probe_scorer=-
```

The shipped rule, whose `--ref` must carry the scorer (`main` once it lands):

```bash
gh workflow run measure.yml --ref main -f ref=555dbb358203c0ad77f35a030f54d821253e1fbd -f base=main -f nordic=true -f r2025=true
gh workflow run measure.yml --ref main -f ref=555dbb358203c0ad77f35a030f54d821253e1fbd -f base=main -f nordic=true -f r2025=true -f scorers=wall_top_cover_vs_ifc
```

The scorer and the probe are this change's, and `main` has them once it lands (until then use `--ref re/1067-wall-top-cuts`); the runs above measured the branch's commits c0fa0953, e7d8d172 and dfdb1025 (the scorer and probe as they stand), kept under `refs/archive/2026-10-30/re/1067-wall-top-cuts` once it lands. The tallies join the probe's `wall <id> relation` lines (`JoinToRoofGStep top`, `CutPairGStep`) with the scorer's `wall`, `top`, `levels`, `pair` and `gap` lines.
