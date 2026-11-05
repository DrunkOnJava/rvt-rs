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
**Status:** positive on the office, with three loops (four at permit) left untagged by a refusal; the runs are in section 4.

## 0. Result

Revit's export tags a floor's void with the `Tag` of one sketch line of its loop, and RE-151 took the lowest ElementId of the lines on it. That is Revit's on 27 of the 36 voids of the office as built and 27 of the 35 at building permit. On 8 and 7 of the others it is the **highest** ElementId of the loop's lines, and what tells the two apart is the direction the lines run round the loop in the order they were drawn: counter-clockwise takes the first line, clockwise the last, as if Revit turned a clockwise loop round before writing the opening. A sketch line's index in its sketch is its place in the order it was drawn, and it rises with its ElementId on all 216 lines of the 6 sketch groups read in the as-built file and on all of the permit file's.

The rule is written: the office as built goes from 27 to 33 of its 36 voids with Revit's tag (box within 0.01 ft, name, type and `Reference` equal on every one) and from 9 voids written under another line's Tag to none; at permit from 27 to 31 of 35, and from 8 to none. The other 3 (4 at permit) are loops of 6 lines drawn out of order round the loop, which the rule leaves untagged and unwritten, and the held-out models' exports are byte for byte `main`'s (section 4).

## 1. The 9 voids that were not Revit's

On `main` at 240c7d7c the office as built writes 36 floor voids that the scorer reads as 27 matched and 9 extras (`none in IfcSlab`), and misses the 9 Revit tags instead. Each of the 9 is written with the loop and the GlobalId Revit hashes from the host's and the tag's, under another line's Tag. Of the 36 voids in Revit's export:

| Revit's tag is | as built | at permit | the rule writes |
|---|---:|---:|---|
| the lowest ElementId of the loop's lines | 27 | 27 | 25 and 24 |
| the highest ElementId of the loop's lines | 8 | 7 | 8 and 7 |
| neither (3147725, the fourth of 6 lines) | 1 | 1 | none |

The probe reads each void's sketch lines from the element records (category `OST_SketchLines`, owner the floor), orders them by their index in the sketch group's `ElementIdIndexPairSet` and chains them. Where the lines are axis-aligned (32 of the 36 as built) the chain gives the winding, and it is counter-clockwise on all 26 voids whose tag is the lowest and clockwise on all 5 whose tag is the highest. The 4 oblique loops of the rest are written by the same test on the ring, which is the result above.

The loops the rule leaves out are all of 6 lines whose ids do not follow each other round the loop, because the sketch was edited after it was drawn: 3143563, 3147725 and 3151525 as built, and 3147725, 3147917, 3148105 and 3151525 at permit. On 3147725 the lines are a rectangle, 3147722 to 3147725, with a notch cut in a corner by 3147758 and 3147788, drawn last; Revit's tag is 3147725, neither the first nor the last. On the others it is the lowest. A reading that fits all four, not tested because the probe does not print a line's own direction: Revit starts the loop at the first line drawn, runs it in that line's own direction, and turns it round if that runs clockwise, so the tag is the lowest where it runs counter-clockwise and the line before the first one round the loop (3147725 on 3147722) where it runs clockwise.

## 2. The rule

For each void, among the sketch lines that lie on its loop (RE-151's test, a point of the line on an edge): take them in ElementId order, find the edge of the loop each lies on, and require the sequence to step the same way round the loop at every step (one edge on, or one edge back, around the closed loop). The direction of the loop in that order, counter-clockwise or clockwise, is the winding of the ring combined with the direction of the steps. Counter-clockwise takes the lowest ElementId, clockwise the highest. A void whose lines do not step the same way once round (two lines on one edge, a gap of half the loop) is left untagged and so not written, as one with no line on it already is.

## 3. What is not shown

- The index of a sketch line in its sketch (`ElementIdIndexPairSet.m_data`) is not read by the exporter; the rule uses the ElementId order, which equals the index order on every line measured. A sketch whose lines were reordered would have the first and last lines elsewhere.
- Core Interior holds one floor void in Revit's export, tagged with the lowest ElementId; RE1 Architecture holds none. The rule's cases are the office's only.
- Why Revit turns a clockwise loop round is not read.

## 4. Runs

- Measure [41943115366](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41943115366): the probe on both office files at c2afe0ce, with the sketch group records.
- Measure [41946217995](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41946217995) (`-f base=main -f nordic=true -f models="nordic-office-as-built nordic-office-permit"`, `unfilled_openings_vs_ifc`, 7e2e7c87 against `main` at 77d71cbb). As built: openings in Revit's export 220, written 72 (75 on `main`), both 64 (58); `none in IfcSlab` written 33 of 36, box within 0.01 ft 33, name, `PredefinedType` and `Reference` equal 33; written that Revit's export does not 8, all `none in IfcWall`, predating this change (they were 17 with the 9 voids). At permit: written 65 (69), both 56 (52); `none in IfcSlab` 31 of 35 written and equal; written that Revit's export does not 8, and one counted apart, the rectangular wall opening of RE-321.
- Held-out models, counts only: Measure [41947057835](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41947057835) (7e2e7c87 against `main`), result below.
