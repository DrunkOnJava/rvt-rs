# RE-335: a wall that embeds a curtain wall is cut by it, and rvt-rs draws it uncut

**Date:** 2026-11-04
**Issues:** #1067 (walls drawn taller than Revit's body); follows RE-258 (the base), RE-267 and RE-320 (the top), RE-270 (`WallEmbedWallGStep`)
**Artefacts:** the BIM4LCA office as built and at building permit (Revit 2024, Revit 24.0.5's IFC4 exports, CC BY-SA 4.0, `research/bim4lca-measure.tsv`), as Measure fetches them
**Probe, tool:** `probes/examples/probe_1067_native_dump.rs` (unchanged), `tools/re/wall_heights_vs_ifc.py`, `tools/re/wall_top_cover_vs_ifc.py`
**Runs:** Measure [41906631624](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41906631624) (`main` at 5026d863: both offices, Core Interior and RE1 Architecture, the probe and the two scorers)
**Status:** measured, not shipped. After RE-267 and RE-320 the office's walls whose lowest point is not Revit's are 20 as built and 24 at permit. Six of the 20 as built (five of the 24 at permit, and three more that Revit raises 9.514 ft) are walls that embed a curtain wall 8.2021 ft tall from their base; Revit's body is the wall less that curtain wall's prism, and rvt-rs draws it whole. The record says that a wall embeds a curtain wall (57 walls as built do) but not which of them Revit's lowest point is raised on, and the rest of the rule is a boolean the walls' ends complicate (section 3). Nothing is drawn differently; the diagnostics do not yet count these walls.

## 1. What is left of #1067

`wall_heights_vs_ifc.py` on `main` (run 41906631624), against Revit's IFC4 exports:

| | as built | at permit |
|---|---:|---:|
| walls | 607 | 589 |
| base and top Revit's | 586 | 565 |
| top Revit's, base not (Revit raises the base) | 20 | 24 |
| base Revit's, top not | 1 (3004978, the stub of RE-320 section 4) | 0 |

The 20 bases as built are the walls of RE-258 section 4: 14 in the Basement, raised 0.656 ft, 2.9 to 3.28 ft where Revit's footings under them slope (a footing's own body, not read), and six on the upper levels, each raised exactly 8.2021 ft (2098938, 2105250, 2177212, 2293116, 2294996 and 3450274). RE-258 found no base join on those six and no floor top at that height.

## 2. The six are the hosts of an embedded curtain wall

Every one of the six has a `WallEmbedWallGStep` (`m_bCutSomething` 1, `m_bIsTrueBoolean` true) whose `m_instId` is a curtain wall (an `IfcCurtainWall` in both exports, which the scorers do not list because they read `IfcWall` only): 1746112, 2105069, 1746117, 2293862, 2295004 and 3450547. Read from each curtain wall's own record (its `GLine` and the envelope of its vertical `Plane`s), each

- stands on the same line as its host (0 to 0.112 ft off it) and begins at the host's base level (the same z to 1e-9 ft),
- is **8.2021 ft (2.5 m) tall**, the raise,
- spans 0.90 to 1.0 of the host's length (from 0.05 and 0.95, or from 0 to 0.98, 0.99 or 1.0).

Revit's lowest point on each host is the curtain wall's top. 57 walls as built embed one: 72 embedded walls in all, every one a curtain wall. The other 51 are not raised. Some embed a curtain wall that is shorter than the host (0.1 to 0.9 of its length, so the rest stays at full height) or lower, and others one that spans it to within 2% and leave a stub of the host at an end (2177158, 2178914, 2178961, 2103576 as built). The embed step alone separates nothing, as the walls' other records did not (RE-257).

## 3. Why the rule is not shipped

What Revit's body is, measured on two walls with the same curtain wall (8.2021 ft tall, spanning 0.051 to 0.949 of a 20.54 ft host, 0.387 ft thick):

| Wall | Revit's horizontal faces | Revit's volume | Reading |
|---|---|---:|---|
| 2103576 as built | 7.8922 ft² at the top, 7.1382 ft² at 3.28 ft below it | 32.08 ft³ | the host less the curtain wall's prism: the full-length strip above it (25.89 ft³) and the two end stubs down to the base (6.19 ft³) |
| 2098938 as built | 7.1382 ft² at the top | 23.42 ft³ | only the strip over the curtain wall's span (7.1382 x 3.2808): the end stubs are gone, and its lowest point is the strip's |

The two have the same curtain wall, and Revit's lowest point differs. 2098938 has two `CutPairGStep`s and `CleanPairGStep`s with 1927726 and 1927727, whose records hold `ColumnAttachment` and `AttachmentData`, as columns' do: the end stubs, 1.05 ft each, may lie inside them, and a Join Geometry cut would remove them. That is an inference, not read. 2103576 as built has no such partner and keeps its stubs (at permit it is raised, so the stubs went, which was not examined). So the body is the host less the curtain wall's prism less the volumes of its cut partners, and the lowest point depends on the second.

A rule that draws the host less the curtain wall's prism alone (a stepped base, as RE-320 draws a stepped top) would give 2103576's body above and leave 2098938 and the others with stubs Revit does not have: the lowest point still the base, the volume closer (90.6 ft³ to about 30 against Revit's 23.4) and not Revit's. Raising the base to the curtain wall's top for every host the curtain wall spans to within 5% would be wrong on 2103576 (as built) and 2177158, which keep their stubs.

## 4. What the diagnostics could count

Not done here. The walls that name an embedded curtain wall are 57 as built, and Revit's lowest point is raised on 6 of them (11%); a count of them would be an upper bound of the walls drawn taller than Revit's body, not a count of errors. The 14 Basement walls on sloped footings and the stub of 3004978 are the other counted causes of the 21.

## 5. Not measured

- The cut partners of the six (columns?): whether the volume of each, taken off the host, gives Revit's end.
- The three walls at permit that Revit raises 9.514 ft (1378125, 1416796 and 2294996): whether their embedded curtain walls are 9.514 ft tall was not read.
- A host whose embedded curtain wall begins above its base or reaches its top.
- The Basement's footings.

## 6. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=5026d8639e144578b615af19ea96e25e0aed6729 -f base=none -f nordic=true \
  -f models="nordic-office-as-built nordic-office-permit core-interior re1-architecture" \
  -f scorers="wall_heights_vs_ifc wall_top_cover_vs_ifc" -f probe=probe_1067_native_dump -f probe_scorer=-
```

The curtain walls' lines and heights come from the probe's `wall <id> step GLine` and `step Plane` lines, and the embedded walls from its `relation WallEmbedWallGStep` lines.
