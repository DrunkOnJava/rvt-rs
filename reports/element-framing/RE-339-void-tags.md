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
**Status:** positive on the office, on one exporter's 8 and 7 clockwise voids; Core Interior's 62 floor and shading-device voids, which all take the lowest id, bound the rule to loops whose direction is read. The follow-up for the one void left wrong is [#1408](https://github.com/DrunkOnJava/rvt-rs/issues/1408).

## 0. Result

Revit's export tags a floor's void with the `Tag` of one sketch line of its loop, and RE-151 took the lowest ElementId of the lines on it. That is Revit's on 27 of the 36 voids of the office as built and 27 of the 35 at building permit, and on all 62 floor and shading-device voids of Core Interior. On 8 and 7 of the others it is the **highest** ElementId of the loop's lines, and what tells the two apart is the direction the lines run round the loop in the order they were drawn: counter-clockwise takes the first line, clockwise the last. That is what 8 voids of one exporter (Revit 24.0.5's IFC4 export of the BIM4LCA office) show; why Revit starts a clockwise loop at the line drawn last is not read. A sketch line's index in its sketch is its place in the order it was drawn, and it rises with its ElementId on all 216 lines of the 6 sketch groups read in the as-built file and on all of the permit file's.

The rule is written, with the lowest id kept wherever the direction is not read (section 2): the office as built goes from 27 to 35 of its 36 voids with Revit's tag (box within 0.01 ft, name, type and `Reference` equal on every one) and from 9 voids written under another line's Tag to 1; at permit from 27 to 34 of 35, and from 8 to 1. The GlobalId of such a void is rvt-rs's own (`gid(["void-opening", host GlobalId, tag])`, not Revit's hash, which the scorer does not compare), so the 8 voids as built and 7 at permit whose tag changes get new GlobalIds. Core Interior's export and RE1 Architecture's are byte for byte `main`'s (`ifc.sha256`), and so are the 19 held-out and spent models' (section 4). The one void left wrong on each file is 3147725, a loop of 6 lines with a notch drawn later ([#1408](https://github.com/DrunkOnJava/rvt-rs/issues/1408)).

## 1. The 9 voids that were not Revit's

On `main` at 77d71cbb the office as built writes 36 floor voids that the scorer reads as 27 matched and 9 extras (`none in IfcSlab`), and misses the 9 Revit tags instead; each of the 9 is written under another line's Tag. Of the 36 voids in Revit's export:

| Revit's tag is | as built | at permit | the rule writes it |
|---|---:|---:|---|
| the lowest ElementId of the loop's lines | 27 | 27 | 27 and 27 |
| the highest ElementId of the loop's lines | 8 | 7 | 8 and 7 |
| neither (3147725, the fourth of 6 lines) | 1 | 1 | 0 and 0 |

The probe reads each void's sketch lines from the element records (category `OST_SketchLines`, owner the floor), orders them by their index in the sketch group's `ElementIdIndexPairSet` and chains them. Where the lines are axis-aligned (32 of the 36 as built) the chain gives the winding, and it is counter-clockwise on all 26 voids whose tag is the lowest and clockwise on all 5 whose tag is the highest; the 4 oblique loops of the rest are written by the same test on the ring, which is the result above. Every line read on the office is a straight `GLine`.

Five loops of 6 lines (3143563 and 3151525 as built; 3147917, 3148105 and 3151525 at permit) have lines that do not follow each other round the loop in id order, because the sketch was edited after it was drawn. No direction is read for them and they keep the lowest id, which is Revit's on all five. The sixth, 3147725, is the same kind and not the lowest: its lines are a rectangle, 3147722 to 3147725, with a notch cut in a corner by 3147758 and 3147788, drawn last, and Revit's tag is 3147725, neither the first nor the last. [#1408](https://github.com/DrunkOnJava/rvt-rs/issues/1408) holds that void and a reading to test.

## 2. The rule

For each void, among the sketch lines that lie on its loop (RE-151's test, a point of the line on an edge): take them in ElementId order and find the edge of the loop each lies on. The direction is read only if there are at least three lines, all straight, and every step from one line to the next in that order, the closing step from the last back to the first included, is exactly the next edge round the loop or exactly the previous one (so each line is on an edge of its own); the direction of the loop in that order, counter-clockwise or clockwise, is then the winding of the ring combined with the direction of the steps. Counter-clockwise takes the lowest ElementId and clockwise the highest. Anywhere else (fewer lines, a curve, two lines on one edge, a step over an edge, a closing step that disagrees) nothing says the loop runs clockwise and the tag is the lowest, RE-151's rule. A void with no line on it is left untagged, as before. `void_tag` is a pure function and has failure-first unit tests for these cases.

## 3. What is not shown

- The index of a sketch line in its sketch (`ElementIdIndexPairSet.m_data`) is not read by the exporter; the rule uses the ElementId order, which equals the index order on every line measured. A sketch whose lines were reordered would have the first and last lines elsewhere.
- The clockwise case rests on the office's 8 and 7 voids. Core Interior's export is byte for byte `main`'s, so the rule reads none of its 62 voids as clockwise, and Revit's tag is the lowest on all 62; no other model has a void the rule changes. The three-line minimum, the exact steps and the straight lines only narrow where the direction is read; they were not measured on more than the office.
- A loop with an arc or other curve keeps the lowest id until curves are measured: no void on the office has one.

## 4. Measured

Head e8ef33d3 against `main` at 77d71cbb. An earlier form of the rule (7bbadb50, Measure 41947422624) gave the same 35 of 36 and 34 of 35.

- Measure [41953415257](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41953415257) (`-f base=main -f nordic=true -f models="nordic-office-as-built nordic-office-permit core-interior re1-architecture"`, scorers `unfilled_openings_vs_ifc psets_vs_ifc qtos_vs_ifc`). As built: openings in Revit's export 220, written 75 (75 on `main`), both 66 (58); `none in IfcSlab` written 35 of 36 (27), box within 0.01 ft, name, `PredefinedType` and `Reference` equal on all 35; written that Revit's export does not 9 (17): 1 `none in IfcSlab` (3147725) and the 8 `none in IfcWall` that predate this change. At permit: written 69 (69), both 59 (52); `none in IfcSlab` 34 of 35 (27); written that Revit's export does not 9 (16), 1 of them 3147725, and one counted apart, RE-321's rectangular wall opening. The census (W7): property values on objects rvt-rs writes 45,601, equal 44,231, missing 1,369 and different 1 as built (45,585, 44,223, 1,361, 1 on `main`), values rvt-rs writes that Revit's export does not 2 (10); at permit 45,766, 44,394, 1,371, 1 and 9 (45,752, 44,387, 1,364, 1 and 16); quantities equal 29,546 and 29,833, missing 9,945 and 9,929 (9,931 and 9,917). The 8 and 7 new matches add 8 and 7 missing property values and 14 and 12 missing quantities, which Revit's export writes on those openings and rvt-rs does not. Core Interior (63 of 63) and RE1 Architecture (1 of 3): the scorer output and `ifc.sha256` are identical to `main`'s.
- Held-out and spent models, counts only: Measure [41953418616](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41953418616) (`-f holdout=true -f models="holdout-* spent-*"`, `unfilled_openings_vs_ifc`): the exports of all 19 are byte for byte `main`'s and the scorer diff is empty, so they hold no void the rule changes and say nothing of it.
- Measure [41943115366](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41943115366): the probe on both office files at c2afe0ce (a commit of #1400's branch; the probe's blob is this branch's), with the sketch group records.
- Not measured: Autodesk's samples and the 24 R2025 models (the unfiltered run is the main session's).

## 5. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=<sha> -f base=main -f nordic=true \
  -f models="nordic-office-as-built nordic-office-permit core-interior re1-architecture" \
  -f scorers="unfilled_openings_vs_ifc psets_vs_ifc qtos_vs_ifc"
gh workflow run measure.yml --ref main -f ref=<sha> -f base=none -f nordic=true \
  -f models="nordic-office-as-built nordic-office-permit" -f scorers=none \
  -f probe=probe_1165_void_tags -f probe_scorer=-
```

The probe's `open` lines are Revit's unfilled openings with a host that is not a wall, its `line` lines the sketch lines of those hosts and its `group` lines their sketch groups' records.
