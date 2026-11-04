# RE-322: A wall's end at a corner follows the order its record joins the walls in

**Date:** 2026-11-03
**Issues:** #1194 (checklist item "Wall ends at corners"). Follows RE-70, RE-71, RE-73 and RE-74 (butt joins, layered joins, T joints and angled joins from the join lists), RE-127 (joined-wall lists), RE-222 and RE-223 (a wall's native joins and T joints) and RE-252 (the native join step that names the wall a wall runs through).
**Artefacts:**

| file | sha256 | licence |
|---|---|---|
| `ARK_NordicLCA_Office_Concrete_As-Built.rvt` (Revit 2024) | `747120005b4b35c7cb66ec61158aab50ded95e052e172cb387b86cd15b31d906` | CC BY-SA 4.0, VTT (`research/bim4lca-measure.tsv`) |
| `ARK_NordicLCA_Office_Concrete_As-Built_Revit.ifc` (Revit's IFC4 export) | `b2f530eba8b4424c4542b98e9a2c7bbcc726367f21b2740f3074e4a6ce917382` | CC BY-SA 4.0 |
| `ARK_NordicLCA_Office_Concrete_BuildingPermit.rvt` (Revit 2024) | `1c12c2058128315edbd65a8bf97f60da971aee8625f21233b8477d9c6165c1d1` | CC BY-SA 4.0 |
| `ARK_NordicLCA_Office_Concrete_BuildingPermit_Revit.ifc` (Revit's IFC4 export) | `bc3bd3366a3c413ce74840300a590f59102b740118ef37898e906253b6b7c08d` | CC BY-SA 4.0 |
| `2024_Core_Interior.rvt` (Revit 2024) | `c805df445d613b408e37337765572021265e3f5dfdc7d1fa53b22ba1600b8014` | MIT |
| `RE1-Architecture.rvt` (Revit 2025) | `4a78bdd68e7f4dd7806060db07c222da9a810e20b3f303c17a443294b91c8ce4` | MIT |
| `R2025_Models_wRVTSets.zip`, 23 Revit 2025 models, among them `18_WallJoins` (`research/r2025-models.tsv`) | `ff9b3946eddcaff52ba42e28d77ea556aa5df905a13402d7f25692758bf59559` | Apache-2.0, puzzbobb (#356) |

Measure fetches all of them and stores none. The reference export is Revit's own IFC4 export of each file, as RE-222 lists them (Core Interior's is `2024_Core_Interior_slim.ifc`, RE1's and the R2025 models' are in their repositories).
**Probe:** `probes/examples/probe_re322_wall_corners.rs` (each wall's curve, root fields, join lists, steps and planes from its native record, and each wall type's layers). **Scorer:** `tools/re/wall_corner_ends_vs_ifc.py` (each end of each wall against Revit's body, by what the join says, and the rule below over the same data).
**Runs:**
- [41753373378](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41753373378): the probe and scorer on the office, `main` at 8da11dab (the baseline of section 1);
- [41753475029](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41753475029): `wall_bodies_vs_ifc`, `wall_ends_cv_vs_ifc` and `wall_ends_list` on `main`;
- [41754748710](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41754748710): the probe on Core Interior, the four RE1 models and the R2025 models;
- [41756871771](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41756871771) (360b458f, first rule), [41758238706](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41758238706) (bf6103c8, with the mitre) and [41759839869](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41759839869) (edc7bb08, the rule as it stands), each against `main`;
- the final head, unfiltered, with the held-out models: RUN_FINAL.

**Status:** positive, narrowed: the order the record joins the walls in is a priority, and decides the end of a wall at a corner of three or more walls and at a mitre; read only where it is right on every measured end. Negative for layered corners of two walls, for a layered wall at layered walls and for every corner a column cuts (section 5).

## 1. What was left

RE-222 section 4 measured a corner rule from the join order and dropped it: it was Revit's end on fewer ends than the join lists' reach already gave. Its table counted every wall the record lists at a point. The list also names the walls on other storeys that end at the same point (a join at a corner of a four-storey building lists a wall per storey, up to 16 walls), and a wall that does not overlap another in elevation does not take part in the join. Counting only the walls that overlap in elevation, the ends rvt-rs had wrong on `main` (8da11dab), by the walls meeting there (this one included):

| class of end | as built: ends | right | off | permit: ends | right | off |
|---|---:|---:|---:|---:|---:|---:|
| T (another wall meets its middle) | 398 | 381 | 17 | 384 | 324 | 60 |
| free (no wall) | 102 | 100 | 2 | 140 | 125 | 15 |
| two walls, square, one layer each | 274 | 273 | 1 | 288 | 265 | 23 |
| two walls, square, a layered wall | 199 | 162 | 37 | 202 | 167 | 35 |
| two walls, straight on or angled | 14 | 13 | 1 | 6 | 5 | 1 |
| three walls, single layers | 49 | 35 | 14 | 45 | 21 | 24 |
| three walls, a layered wall | 100 | 67 | 33 | 93 | 57 | 36 |
| four walls or more, single layers | 23 | 11 | 12 | 12 | 4 | 8 |
| four walls or more, a layered wall | 54 | 29 | 25 | 8 | 5 | 3 |

(The table leaves out one end as built, right, that no wall meets end to end.) Walls with both ends right: 482 of 607 as built, 422 of 589 at permit, so 125 and 167 walls are not. Of those, 14 and 55 are off only at T or free ends; 111 and 112 have a corner end that is off. The two-wall single-layer corners RE-70 and RE-127 read are right except where a column cuts the wall (23 at permit, section 5).

## 2. The join order is a priority

`VWallDriver.m_controlJoinsSet` lists, for each join, its walls with the end each joins by, in an order the record keeps. At a corner of three or more walls that order decides which wall stops and which runs on, once the walls that do not overlap in elevation are left out. Take a corner where one wall (the stem, C) meets two collinear walls (the bar, B and S) and read the end of S:

| order | S's end | by |
|---|---|---|
| C before S (CBS, BCS, CSB) | at C's near face, half C's thickness short of the joint | C runs through; S stops against it as at a T joint |
| S first, then C (SCB) | at C's far face, half C's thickness past the joint | S runs through |
| B before C, S before C (SBC, BSC) | at the joint, flush with B | S and B are one bar, C stops against it |

Over every end the rule reads (section 4) this is Revit's end, on both sides of the wall, with no exception. A wall that is first among the perpendicular walls runs through all of them; a wall with earlier perpendicular walls stops at their near face. Where two earlier walls disagree about where S stops (a bar whose halves differ in thickness and are not aligned, so S meets two different faces), Revit's end is ragged, one side to each face, and no single reach is right: the rule leaves the end alone (taking the first wall listed instead is wrong on 1 of 98 ends as built and 2 of 37 at permit, each with the two sides at different faces).

## 3. Faces are where each wall's own line ends

The faces a wall stops at are the other wall's faces where its own line ends, not where the joint is. Walls of different thickness are often aligned on a face, not a centreline: the office has bars of 0.656 ft and 0.492 ft walls whose lines end 0.082 ft apart. The unguarded rule, taking half the other wall's thickness from the joint, is Revit's end on 51 of 64 single-layer ends of three walls or more as built; 12 of the 13 misses are bars of unequal thickness (the wall stops at -0.4101 ft, not -0.3281). Reading the face from where the other wall's line ends is exact on all of them (-0.082 - 0.328 = -0.4101).

## 4. The rule

`element_record_wall_joins::corner_joins` reads an end where three or more walls meet end to end at a butt join (`m_wallJoinType` 0). Walls are counted if they overlap the wall in elevation (the record box, as `butt_joins` does). The end is left alone unless:
- every wall of the join is a straight wall with a line, and meets this one at a right angle or straight on;
- no element but a wall cuts the wall (`CleanPairGStep`, `CutPairGStep`, `WallEmbedWallGStep`, as RE-252 reads them);
- no layered wall that continues this one has its line end off the joint;
- the faces of the walls that decide agree to 1e-3 ft;
- the wall has one layer, or has several and stops against walls of one layer each.

Then the end is, every layer alike: at the near face of the earlier perpendicular walls; else at the joint if a wall that continues the wall is listed before the first perpendicular wall after it (and every continuing wall's line ends on the joint); else at the far face of the perpendicular walls after it. `attach_wall_corner_reaches` writes it as the wall's join reach (and as each layer's, for a layered wall), in place of the reach a join list or a T joint gave the end, and names no partner: a corner of three has none. So an end it reads loses the `JoinStartWallElementId` and `JoinStartRunsThrough` that RE-70's lists gave it (the viewer's "stops at" link), also where one perpendicular wall decides it.

The tally of the scorer's reading of the rule, over the same data as section 1 (the scorer leaves out a corner with a wall that has no body in Revit's export, which the exporter does not):

| | fires | right | wrong | ends it fixes | ends it breaks |
|---|---:|---:|---:|---:|---:|
| office as built | 95 | 95 | 0 | 30 | 0 |
| stops at an earlier wall | 67 | 67 | 0 | 25 | 0 |
| runs through the later walls | 22 | 22 | 0 | 2 | 0 |
| flush with the wall that continues it | 6 | 6 | 0 | 3 | 0 |
| office at permit | 34 | 34 | 0 | 9 | 0 |
| stops at an earlier wall | 26 | 26 | 0 | 8 | 0 |
| runs through the later walls | 7 | 7 | 0 | 0 | 0 |
| flush | 1 | 1 | 0 | 1 | 0 |
| Core Interior | 8 | 8 | 0 | 0 | 0 |
| RE1 and the R2025 models | 0 | | | | |

"Right" is Revit's end within 0.001 ft on the whole body and on each side of the wall. What it leaves alone (as built / permit): 731 / 740 ends with fewer than three walls, 25 / 14 where a layered wall that continues the wall ends off the joint, 21 / 0 where a wall meets it at another angle, 10 / 45 where an element other than a wall cuts it, 7 / 7 where a layered wall stops against layered walls, 6 / 0 a layered wall with no earlier wall to stop at, 4 / 3 where the faces of the walls that decide differ, 1 / 0 where a wall that continues it ends off the joint.

What separates the unguarded rule's misses (section 3, and as built 13 of 64): 12 are bars of unequal thickness and 1 is a continuing wall off the line. At permit the unguarded rule is right on 24 of 57; 31 of the 33 misses have a column or a slab cutting the wall.

## 5. Measured negatives

- **Two walls, one layered, square (199 ends as built, 37 off).** The order says which wall runs through, and with the layers alike from the corner and the heights equal RE-71's staircase then gives the end (on this office 64 of 65 through ends and 61 of 65 stopping ends). Where the heights differ, Revit draws a staircase on 12 of 22 ends and a clean butt on 10. A join tweak step that names the partner is on all 12 staircase ends and on 4 of the 10 clean ones (the other 6 have none), with the edge count 3 to 6 on both: it narrows, but it does not reach zero wrong. Where the layers are three structure layers of different widths (the 0.0853, 0.2165, 0.0853 ft and 0.0427, 0.2428, 0.0427 ft types, 3347182 and 460609), RE-71's staircase says +0.1214 ft and Revit stops the core at the other wall's core, -0.1214 ft, on 4 ends of equal height. A layered wall against a single-layer wall, or the reverse, ends flat at the other wall's face on some ends and as a mitre-like end (±0.2461 ft) on others (3 ends as built, 5 at permit). RE-71's rule stays as it is.
- **Three walls or more, a layered wall at layered walls (7 / 7 ends declined).** RE-71's nested layers, with the order saying which wall runs through, give the union of the end right on 7 of 12 ends and each side on none; a flat stop at the other wall's face is right on 8 of 17 and wrong, the two sides at different reaches, on 9. Left as it is. (A layered wall that stops against single-layer walls is exact, and read.)
- **A column cutting the wall (10 as built and 45 at permit ends declined; 47 of 57 and 147 of 158 distinct cutters are `IfcColumn`, the rest slabs and proxies).** The walls around a column stop at its face (-0.9843 ft where the wall's thickness is 0.6562 ft at permit), and the rule would put them at the other wall's face. Columns' joins are RE-202's. The permit's closed loops of four walls (walls 1450965 to 1450972) are of this kind: every one has a column among its cutters.
- **A wall meeting the end's middle** (T) is RE-222's and RE-223's; 17 as built and 60 at permit are still off (44 of the permit's are 0.5 ft or more too long).

## 6. Mitres

The join type (`m_JoinInfo.m_wallJoinType`) is 0, a butt join, on every join of the office and Core Interior. R2025 `18_WallJoins` has one join of type 1, walls 4511 and 4567, which Revit mitres: each wall's end is the line through the outer corner and the inner corner, so the union of its body reaches half the other wall's thickness past its line's end on the outer face and as far short on the inner. rvt-rs drew them as butt joins (7 of the model's 9 walls had both ends right). `corner_joins` reads a mitre of two single-layer walls whose lines end at one point at right angles, and writes the slanted end as RE-74 does for an angled join (the reach along the exterior-side and the interior-side edge): 9 of 9, every end within 0.001 ft. It is the only mitre in the measured corpora, two ends of two walls: the rule is the geometry of a mitre, not a fit, but it is measured on that pair only.

## 7. Measured

Run [41759839869](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41759839869) (edc7bb08) against `main` (8da11dab, run 41753475029), filtered to the office, Core Interior, RE1 and `18_WallJoins`; the unfiltered run on the final head is RUN_FINAL.

| | office as built | | office at permit | | `18_WallJoins` | |
|---|---:|---:|---:|---:|---:|---:|
| | `main` | RE-322 | `main` | RE-322 | `main` | RE-322 |
| walls with both ends within 0.001 ft (`wall_bodies_vs_ifc`) | 482 of 607 | 510 | 422 of 589 | 430 | 7 of 9 | 9 |
| ends within 0.001 ft (`wall_ends_cv_vs_ifc`) | 1,073 of 1,212 | 1,103 | 978 of 1,178 | 986 | 16 of 18 | 18 |
| Revit quantities equal (`qtos_vs_ifc`) | 29,448 | 29,472 | 29,762 | 29,777 | | |

End for end (`wall_corner_ends_vs_ifc` DATA, runs 41753373378 and 41759839869): as built 30 ends go from off to right and none from right to off; 28 walls gain both ends and none loses them. Six ends that were right as built move by 3e-6 to 1.3e-5 ft, inside the tolerance (walls 2114504, 3098955 at both ends, 3121295, 3121883 and 3128081): their face now comes from where the other wall's line ends, not from the joint, and the last digits differ. At permit 8 ends go from off to right, none the other way, and 8 walls gain both ends. The scorer's reading of the rule fixes 9 permit ends and the exporter 8: the start of wall 2639982, a bar of unequal thickness with a layered wall off the line, is read by the scorer only. The one quantity that differs was there before. Layer sets and layer bands are unchanged. Core Interior's 360 walls stay 360 of 360, and its IFC and RE1's are byte for byte as before; `18_WallJoins` changes at walls 4511 and 4567. The other R2025 models were byte for byte as before at the first rule (run 41756871771); the unfiltered run on the final head repeats that.

## 8. What it does not cover

- Layered corners (section 5), and every corner a column cuts.
- A corner whose walls meet at other than a right angle or straight on.
- A T joint (RE-222, RE-223).
- Snowdon Towers (724 of 1,078 walls with both ends right in RE-70 to RE-74) is local only and was not measured again.

## 9. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=edc7bb082eb9e3300c1607a5072e235697d0f4f3 -f base=none -f nordic=true -f r2025=true \
  -f models="nordic-office-as-built nordic-office-permit core-interior re1-architecture r2025-18_WallJoins" \
  -f scorers="wall_bodies_vs_ifc wall_ends_cv_vs_ifc wall_ends_list wall_layers_vs_ifc layer_sets_vs_ifc qtos_vs_ifc" \
  -f probe=probe_re322_wall_corners -f probe_scorer=wall_corner_ends_vs_ifc -f keep_ifc=true
```

`probe_score.txt` of each model holds the class table, the rule's tally and a `DATA` line per end. The baseline of section 1 is run 41753373378: the same probe and scorer on 5b762a0f, which is `main` (8da11dab) and nothing else but the probe and scorer.
