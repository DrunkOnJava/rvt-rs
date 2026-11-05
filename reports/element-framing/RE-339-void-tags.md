# RE-339 — Which sketch line tags a floor's void

**Date:** 2026-11-04
**Issues:** #1165 (unfilled openings), W4 and W7 of [`docs/release-1.0-criteria.md`](../../docs/release-1.0-criteria.md); corrects the tag rule of [RE-151](RE-151-unfilled-openings.md) (#584), which took the lowest ElementId of the sketch lines on a void's loop
**Artefacts:** the BIM4LCA office as built and at building permit (Revit 2024, Revit 24.0.5's IFC4 exports, CC BY-SA 4.0), as `research/bim4lca-measure.tsv` pins them:

| file | sha256 |
|---|---|
| `ARK_NordicLCA_Office_Concrete_As-Built.rvt` | `747120005b4b35c7cb66ec61158aab50ded95e052e172cb387b86cd15b31d906` |
| `ARK_NordicLCA_Office_Concrete_As-Built_Revit.ifc` | `b2f530eba8b4424c4542b98e9a2c7bbcc726367f21b2740f3074e4a6ce917382` |
| `ARK_NordicLCA_Office_Concrete_BuildingPermit.rvt` | `1c12c2058128315edbd65a8bf97f60da971aee8625f21233b8477d9c6165c1d1` |
| `ARK_NordicLCA_Office_Concrete_BuildingPermit_Revit.ifc` | `bc3bd3366a3c413ce74840300a590f59102b740118ef37898e906253b6b7c08d` |

**Probe and scorer:** `probes/examples/probe_1165_void_tags.rs`, `tools/re/unfilled_openings_vs_ifc.py`
**Status:** positive on the office; the runs are listed in section 4.

## 0. Result

Revit's export tags a floor's void with the `Tag` of one sketch line of its loop, and RE-151 took the lowest ElementId of the lines on it. That is Revit's on 27 of the 36 voids of the office as built and 27 of the 35 at building permit. On the others it is the **highest** ElementId of the loop's lines, and what tells the two apart is the direction the loop runs in the order the lines were drawn: counter-clockwise takes the first line, clockwise the last, as if Revit turned a clockwise loop round before writing the opening. A sketch line's index in its sketch is its place in the order it was drawn, and it rises with its ElementId on all 216 lines of the 6 sketch groups read in the as-built file and on all of the permit file's.

## 1. The 9 voids that were not Revit's

On `main` at 240c7d7c the office as built writes 36 floor voids that the scorer reads as 27 matched and 9 extras (`none in IfcSlab`), and misses the 9 Revit tags instead. The 9 pairs are the same pairs on both sides: each void is written, with the loop and the GlobalId Revit hashes from the host's and the tag's, under another line's Tag. Of the 36 voids in Revit's export:

| winding of the lines in ElementId order | the tag is | as built | at permit |
|---|---|---:|---:|
| counter-clockwise (as built 26 computed and 1 oblique loop, whose winding the probe does not compute; at permit 27 computed) | the lowest ElementId | 27 | 27 |
| clockwise (as built 5 computed and 3 oblique loops; at permit 7 computed) | the highest ElementId | 8 | 7 |
| a join component that holds a second, separate loop | the highest of the loop's own 4 lines | 1 | 1 |

The probe reads each void's sketch lines from the element records (category `OST_SketchLines`, owner the floor), chains them in the order of their index in the sketch group's `ElementIdIndexPairSet`, and computes the winding from the chain. The last row is a loop of 4 lines joined at a corner to two lines of another loop; taken as the loop's own, its lines run clockwise and Revit's tag is the highest of the 4.

## 2. The rule

For each void, among the sketch lines that lie on its loop (RE-151's test, a point of the line on an edge): take them in ElementId order, find the edge of the loop each lies on, and require the sequence to step the same way round the loop at every step (one edge on, or one edge back, around the closed loop). The direction of the loop in that order, counter-clockwise or clockwise, is the winding of the ring combined with the direction of the steps. Counter-clockwise takes the lowest ElementId, clockwise the highest. A void whose lines do not step the same way once round (two lines on one edge, a gap of half the loop) is left untagged and so not written, as one with no line on it already is.

## 3. What is not shown

- The index of a sketch line in its sketch (`ElementIdIndexPairSet.m_data`) is not read by the exporter; the rule uses the ElementId order, which equals the index order on every line measured. A sketch whose lines were reordered would have the first and last lines elsewhere.
- Core Interior holds one floor void in Revit's export, tagged with the lowest ElementId; RE1 Architecture holds none. The rule's cases are the office's only.
- Why Revit turns a clockwise loop round is not read.

## 4. Runs

Measure [41943115366](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41943115366) (the probe on both office files at c2afe0ce). The runs of the writer are filled in as they come.
