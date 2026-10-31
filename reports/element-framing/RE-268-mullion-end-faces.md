# RE-268 — Which end faces of a curtain mullion Revit's solid keeps

**Date:** 2026-10-30
**Issues:** #1046 (step 2), #1016 (quantities), W7 of [`docs/release-1.0-criteria.md`](../../docs/release-1.0-criteria.md); follows RE-248 (what a mullion's `NetSurfaceArea` counts), RE-243 (mullions drawn as an extrusion), RE-226 (`Span`), RE-241, RE-72
**Artefacts:** the BIM4LCA office as built and at building permit (Revit 2024, Revit's IFC4 exports), as Measure fetches them
**Runs:** Measure [41079057668](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41079057668) (`probes/examples/probe_1046_curtain_graphs.rs`, as built), [41080188877](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41080188877) (`probes/examples/probe_1046_mullion_joins.rs`, both), [41094260414](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41094260414) (that probe with `tools/re/mullion_joins_vs_ifc.py`), [41091498749](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41091498749) (this change against `main`: `qtos_vs_ifc`, `psets_vs_ifc`, `tools/re/mullion_ends_vs_boxes.py`; the same counts in [41087652091](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41087652091) on the commit before the size bound) and [41091518481](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41091518481) (unfiltered, every scorer, with the R2025 models and `timing`)
**Result:** positive, narrowed: which of a mullion's two ends Revit's solid keeps follows from where the boxes of its curtain wall's mullions meet, and `perimeter x length + kept ends x section area` equals Revit's `NetSurfaceArea` on every mullion read, 6,606 of 10,003 as built and 6,636 of 10,031 at permit, none different. The mullions on curved and edited curtain walls (2,208) and 1,189 where an end is not read stay missing.

## 1. The question

RE-248 left the count of kept ends (0, 1 or 2) open: neither the mullions' records nor a test of whether another mullion's box covers a point just past an end reproduced it, and the next signals named were the curtain wall's grid, its type's join condition and each mullion's own Make Continuous or Break at Join setting. The type's join condition (`NewCurtainWallType.m_autoJoinCond`) is 0 on every wall (RE-248 §3).

## 2. What the file holds about joins

The probes list the records of the office as built (and, for the second, at permit too). A mullion's own record names its host (`m_hostId`), its type and an edge tag (`MullionFamRef.m_edgeTag`), and `MullionFamRef.m_driverId` is -1 on every mullion. The curtain wall's record holds the grid:

| object in the host's record | what it holds |
|---|---|
| `WallCGDriver.m_mullionJoins` | one entry per join: `m_gridJoinSet`, the mullions that meet at a grid vertex as `{m_edgeTag, m_end}` (the edge tag of a mullion and which end of it, 0 or 1), `m_id`, `m_constULinesButt` |
| `CGDriverMullionData.m_mullionsByEdge` | edge tag to the mullion's ElementId, on 218 of the 220 walls |
| `CurtainGrid` | `m_uLineIds`, `m_panelIds`, `m_joinTypes` |
| `CGridDivideFaceGStep.m_cellMap` | per grid cell its four edge tags, its panel and its grid lines |
| `CurtainGridLine` records (3,122; 3,014 owned by one of the 220 walls that hold mullions) | the line's `GLine` (origin, direction, extent) |

`tools/re/mullion_joins_vs_ifc.py` against Revit's export (run 41094260414; at permit 4,911 joins on 151 walls, 9,217 of 9,824 mullions agree):

- There are 4,909 joins on 149 of the 220 curtain walls: 4,908 of three mullions and 1 of two. `m_constULinesButt` is false on all. `CurtainGrid.m_joinTypes` is empty on 218 walls; two walls' records hold neither the driver's joins nor a `CurtainGrid` object. 108 members name an edge that has no mullion in the export.
- `m_end` 0 is the far end of the mullion's exported extrusion (where its extrusion ends) and `m_end` 1 its start: 6,943 + 7,439 members, no exception.
- In each of the 4,794 joins of three that were located, the one mullion running across the other two is upright, and the two others run along one another: a join lists a T, the upright that ends at a vertex and the horizontal that runs through it. It does not say which of them is continuous.

The joins alone are not the answer. Reading "an end is hidden when it is one of the two mullions of a join that run along one another" agrees with Revit's count on 9,201 of 9,796 mullions drawn as one extrusion:

| ends the joins leave counted | ends Revit's `NetSurfaceArea` counts | mullions |
|---|---:|---:|
| 0 | 0 | 4,372 |
| 0 | 2 | 43 |
| 1 | 0 | 36 |
| 1 | 1 | 694 |
| 1 | 2 | 28 |
| 2 | 0 | 8 |
| 2 | 1 | 480 |
| 2 | 2 | 4,135 |

A join lists a vertex where an upright ends against a horizontal that runs through. Vertices where two mullions meet end to end, or an upright continues into the next, are in none, which fits the 480 + 8 mullions the joins leave with both ends counted and Revit counts 1 or 0 of; and a join's presence does not settle a count either: 43 horizontals that are in joins at both ends count both. So the rest of this report reads the ends from the boxes the joins produced.

## 3. The ends follow from the boxes

RE-248 §2 counted an end as covered when a point 2 mm past its centre lies in another mullion's box. That is not what Revit counts: an end that butts against the side face of a crossing mullion has such a point inside the crossing box and Revit counts it. What Revit does not count is an end face that coincides with the end face of the mullion's own continuation. At the end point `p`, outward along the mullion's axis, each other mullion of the same curtain wall (`m_hostId`) whose box holds `p` within 3 mm is:

| label | the other mullion |
|---|---|
| PARTNER | runs the same way, has an end at `p` and its body on the far side |
| PERP_CENTER | runs across, `p` on its axis |
| PERP_FACE | runs across, `p` on one of its side faces |
| PERP_IN | runs across, `p` elsewhere inside it |
| PAR_SAME, PAR_IN | runs the same way and lies over this one (an end at `p` with its body on the same side, or its box holding `p`) |
| OBLIQUE | runs neither along nor across |

An end is hidden when it has a PARTNER and nothing across covers `p`, or when a mullion across covers `p` (PERP_CENTER or PERP_IN) and it has no PARTNER; otherwise it is counted. The mullion's count of kept ends is its two ends' sum. rvt-rs's box of a mullion is its Revit solid (the `NetVolume` of RE-243 is equal on 7,795), so the rule runs on rvt-rs's boxes: `tools/re/mullion_ends_vs_boxes.py` takes the boxes from rvt-rs's IFC, the mullion's axis from rvt-rs's own `Length`, and Revit's count from its `NetSurfaceArea`. Against Revit's counts (run 41091498749, office as built):

| end labels | ends Revit's `NetSurfaceArea` counts | mullions |
|---|---:|---:|
| PARTNER, PARTNER | 0 | 3,891 |
| PARTNER, PERP_FACE | 1 | 574 |
| PARTNER, nothing within 3 mm | 1 | 429 |
| PERP_FACE, PERP_FACE | 2 | 2,795 |
| PERP_FACE, nothing | 2 | 348 |
| nothing, nothing | 2 | 231 |

The rule reads 8,404 mullions, drawn as their box's extrusion or not. 8,268 fall on the diagonal (the count it gives against Revit's: 0 on 3,891, 1 on 1,003, 2 on 3,374), and the other 136 have a `NetSurfaceArea` that is not a whole number of sections (cut mullions, §4). By type and orientation the written mullions are the painted 50 x 50 mm type's horizontal ones with 0 (2,844), 1 (511) or 2 (331) kept ends and its upright ones with 2 (1,801), the plain 50 x 50 mm type's with 0 (13), 1 (52 + 352 upright) or 2 (406 + 78) and the 50 x 150 mm type's with 1 (2 + 74) or 2 (118 + 22).

No mullion that is written has a PERP_CENTER or PERP_IN end (the table above lists every label pair among them): the covered branch of the rule decides only mullions that are left out, and no value that is written. Reading a covered end as unread instead, so that a mullion with one is left out, writes 44 fewer (6,560 equal as built in the scorer) and none different; the branch stays as the rule's reading of the mullions it leaves out.

## 4. What is left out, and what decides the rest

A mullion is not claimed when an end carries PERP_IN, PAR_SAME, PAR_IN or OBLIQUE, when its axis is not known, or when an end meets (within 6 cm) a mullion that is one of those or is shorter than its own section (such a mullion keeps its own count). The scorer also counts what the rule would have said for the mullions it leaves out among the 7,795 that Revit draws as one extrusion of their box (those with a `NetVolume`, office as built):

| left out because | mullions | the rule would be right | wrong |
|---|---:|---:|---:|
| an end lies inside a mullion across it, off its axis and faces (PERP_IN) | 9 | 4 | 5 |
| an end lies over another mullion, or another lies over it (PAR_SAME, PAR_IN) | 159 | 159 | 0 |
| an end meets a mullion left out, or one shorter than its section | 1,021 | 1,019 | 2 |

The 7 the rule would get wrong are why the others are left out: five have a PERP_IN end, and two are horizontal pieces of one wall, 0.1 m long, whose ends butt against an upright that lies over another upright (the count is 0 where the rule says 1). Leaving out the mullions whose end meets one that lies over another or is shorter than its section removes both of those two, and no field read tells them from the 1,019 others that meet the same kind of neighbour and are right. Oblique ends and unknown axes occur only on mullions outside the 7,795.

On the 2,208 mullions Revit does not draw as an extrusion of their box (RE-243 §2: mullions on curved or edited curtain walls), the rule reaches 1,800 and is right on 1,664 and wrong on 136, the cut mullions whose count is a fraction; RE-243 §2 found no field of the mullion's record, its type's or its host's that tells the mullions Revit draws as a mesh from the extrusions on the same walls, and a count that is not whole is a cut solid of the same kind, so none is written.

## 5. What rvt-rs does

`native_mullion_hosts::mullion_hosts` returns each mullion's host as well as the mullions on plain hosts. For each curtain wall, `attach_mullion_plain_hosts` builds the box of every mullion (its record box, along the model's axes and the axis it runs along, or its own axes where it is turned or tilted, as `Span` does), and `partition_schema_mvp::kept_ends` reads the ends. On a mullion on a plain host (the `NetVolume` gate) whose ends it reads, `add_mullion_span` writes `Qto_MemberBaseQuantities.NetSurfaceArea` as `2 (a + b) x length + kept ends x a x b`, `a` and `b` the box's two sides across the mullion. No other mullion gets one.

## 6. Measured

Run 41091498749, this change against `main` (`qtos_vs_ifc`, `psets_vs_ifc`):

| model | `NetSurfaceArea` equal | different | missing | quantities Revit writes that rvt-rs writes equal |
|---|---:|---:|---:|---|
| office as built | 6,606 of 10,003 | 0 | 3,397 | 21,146 → 27,752 of 39,721 |
| office at permit | 6,636 of 10,031 | 0 | 3,395 | 21,050 → 27,686 of 40,013 |

Only `Qto_MemberBaseQuantities.NetSurfaceArea` moves in the quantity census; the property census is identical. The scorer's own counts are 6,604 and 6,634: it groups mullions by the wall rvt-rs's IFC aggregates them under, and 2 mullions have none there, where the exporter groups them by the wall the mullion's record names. The permit file is the same building with the same Tags, so it is a repeat on the same walls, not an independent test. The 3,397 missing are the 2,208 of §4 and 1,189 left out.

Unfiltered run 41091518481 (every scorer, `nordic`, `r2025`, `timing`, this change against `main`): the exports that move are the two office models and RE1 Architecture; the exports of Core Interior, Einhoven, RE1 Electrical, Mechanical and Plumbing and of the R2025 models are byte-identical to `main`'s, and every scorer output apart from those three models' `qtos_vs_ifc` is identical. RE1 Architecture's 10 mullions now carry a `NetSurfaceArea` that Revit's export of it, made with Export base quantities off, does not write (RE-242): the census counts the quantities rvt-rs writes there apart, 68 to 78, so the value is not measurable on it. `rvt-ifc` takes 19.8 s to 20.4 s on the office as built and 19.2 s both ways at permit (one runner, three rounds); every other file is within 4%.

## 7. Reproduce

The scorers land with this change; before it lands, use `--ref re/1046-mullion-ends` for them.

```bash
gh workflow run measure.yml --ref main -f ref=e2cb89e499858857ec1ac7edae5f7b5508f09d2d -f base=main -f nordic=true \
  -f models="nordic-office-*" -f scorers="qtos_vs_ifc psets_vs_ifc mullion_ends_vs_boxes"
gh workflow run measure.yml --ref main -f ref=6e95d29728eb995946ee8489c860c0e5b74e062b -f base=none -f nordic=true \
  -f models="nordic-office-*" -f scorers=none -f probe=probe_1046_mullion_joins -f probe_scorer=mullion_joins_vs_ifc
gh workflow run measure.yml --ref main -f ref=e2cb89e499858857ec1ac7edae5f7b5508f09d2d -f base=main -f nordic=true -f r2025=true -f timing=true
```
