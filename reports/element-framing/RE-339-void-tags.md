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
**Status:** positive on the office; Core Interior's 62 floor and shading-device voids, which all take the lowest id, bound the rule to loops whose direction is read. The runs are in section 4.

## 0. Result

Revit's export tags a floor's void with the `Tag` of one sketch line of its loop, and RE-151 took the lowest ElementId of the lines on it. That is Revit's on 27 of the 36 voids of the office as built and 27 of the 35 at building permit, and on all 62 floor and shading-device voids of Core Interior. On 8 and 7 of the others it is the **highest** ElementId of the loop's lines, and what tells the two apart is the direction the lines run round the loop in the order they were drawn: counter-clockwise takes the first line, clockwise the last, as if Revit turned a clockwise loop round before writing the opening. A sketch line's index in its sketch is its place in the order it was drawn, and it rises with its ElementId on all 216 lines of the 6 sketch groups read in the as-built file and on all of the permit file's.

The rule is written, with the lowest id kept wherever the direction is not read: the office as built goes from 27 to 35 of its 36 voids with Revit's tag (box within 0.01 ft, name, type and `Reference` equal on every one) and from 9 voids written under another line's Tag to 1; at permit from 27 to 34 of 35, and from 8 to 1. Core Interior's 62, RE1 Architecture and every other scorer output are byte for byte `main`'s. The one void left wrong on each file is 3147725, a loop of 6 lines drawn out of order (section 1).

## 1. The 9 voids that were not Revit's

On `main` at 240c7d7c the office as built writes 36 floor voids that the scorer reads as 27 matched and 9 extras (`none in IfcSlab`), and misses the 9 Revit tags instead. Each of the 9 is written with the loop and the GlobalId Revit hashes from the host's and the tag's, under another line's Tag. Of the 36 voids in Revit's export:

| Revit's tag is | as built | at permit | the rule writes it |
|---|---:|---:|---|
| the lowest ElementId of the loop's lines | 27 | 27 | 27 and 27 |
| the highest ElementId of the loop's lines | 8 | 7 | 8 and 7 |
| neither (3147725, the fourth of 6 lines) | 1 | 1 | 0 and 0 |

The probe reads each void's sketch lines from the element records (category `OST_SketchLines`, owner the floor), orders them by their index in the sketch group's `ElementIdIndexPairSet` and chains them. Where the lines are axis-aligned (32 of the 36 as built) the chain gives the winding, and it is counter-clockwise on all 26 voids whose tag is the lowest and clockwise on all 5 whose tag is the highest; the 4 oblique loops of the rest are written by the same test on the ring, which is the result above.

Five loops of 6 lines (3143563 and 3151525 as built; 3147917, 3148105 and 3151525 at permit) have lines that do not follow each other round the loop in id order, because the sketch was edited after it was drawn. No direction is read for them and they keep the lowest id, which is Revit's on all five. The sixth, 3147725, is the same kind and not the lowest: its lines are a rectangle, 3147722 to 3147725, with a notch cut in a corner by 3147758 and 3147788, drawn last, and Revit's tag is 3147725, neither the first nor the last. A reading that fits all six, not tested because the probe does not print a line's own direction: Revit starts the loop at the first line drawn, runs it in that line's own direction, and turns it round if that runs clockwise, so the tag is the lowest where it runs counter-clockwise and the line before the first one round the loop (3147725 on 3147722) where it runs clockwise.

## 2. The rule

For each void, among the sketch lines that lie on its loop (RE-151's test, a point of the line on an edge): take them in ElementId order and find the edge of the loop each lies on. If the lines step the same way round the loop at every step (one edge on, or one edge back, around the closed loop, two lines on one edge counting for nothing), the direction of the loop in that order, counter-clockwise or clockwise, is the winding of the ring combined with the direction of the steps; counter-clockwise takes the lowest ElementId and clockwise the highest. Where they do not step the same way, or a step is half the loop (two lines on opposite sides), nothing says the loop runs clockwise and the tag is the lowest, RE-151's rule. A void with no line on it is left untagged, as before.

## 3. What is not shown

- The index of a sketch line in its sketch (`ElementIdIndexPairSet.m_data`) is not read by the exporter; the rule uses the ElementId order, which equals the index order on every line measured. A sketch whose lines were reordered would have the first and last lines elsewhere.
- Core Interior's voids are the lowest id on all 62 (Measure 41947422624, the scorer output unchanged); whether any of them runs clockwise in id order is not read, since the rule reads the direction only where the lines step the same way round the loop, so the office is the only evidence for the clockwise case. The held-out set is in section 4.
- Why Revit turns a clockwise loop round is not read.

## 4. Runs

- Measure [41943115366](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41943115366): the probe on both office files at c2afe0ce, with the sketch group records.
- Measure [41947422624](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41947422624) (`-f base=main -f nordic=true -f models="nordic-office-as-built nordic-office-permit core-interior re1-architecture"`, `unfilled_openings_vs_ifc`, 7bbadb50 against `main` at 77d71cbb). As built: openings in Revit's export 220, written 75 (75 on `main`), both 66 (58); `none in IfcSlab` written 35 of 36 (27), box within 0.01 ft, name, `PredefinedType` and `Reference` equal on all 35; written that Revit's export does not 9 (17): 1 `none in IfcSlab` (3147725) and the 8 `none in IfcWall` that predate this change. At permit: written 69 (69), both 59 (52); `none in IfcSlab` 34 of 35 (27) written and equal; written that Revit's export does not 9 (16), 1 of them 3147725, and one counted apart, the rectangular wall opening of RE-321. Core Interior (63 of 63) and RE1 Architecture (1 of 3): the scorer output is identical to `main`'s.
- Held-out models, counts only: Measure [41947057835](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41947057835) (7e2e7c87, the first form of the rule, against `main`): see the result in the pull request.
