# RE-383: Revit's body keeps a profile wall's top line on to the wall's end, measured at its stub ends

**Date:** 2026-11-09
**Issues:** #1473 (the far-end stub of a profile wall's sloped top: closed by this, section 5); follows RE-358 (the rule that carries the cut-out on to the wall's end, whose volumes matched Revit's and whose done-when stayed open for the stub's top itself), RE-380 (the pockets against the convex hull) and RE-348 (the wall ends)
**Artefacts:** the BIM4LCA office as built and at building permit (Revit 2024, Revit 24.0.5's IFC4 exports, CC BY-SA 4.0, as Measure fetches them) and Autodesk's Snowdon Towers Architectural with Revit's own IFC4 export (local only, no licence, never committed; counts and equalities only here)
**Tool and runs:** `tools/re/wall_profile_tops_vs_ifc.py`, which reads the output of `probes/examples/probe_1607_wall_profile_lines.rs` (commit `80808447`, the sketch lines of every wall with their ends) and the IFC it is given, takes the file out of its IfcSite placement (`ifc_common.site_frame`) and prints, for each wall whose loop lies in a plane along the model's x or y axis, the highest point of the body at the ends of the wall's box and of its loop and at six points between, the loop's top at the same places, and the loop's top edge at each end carried on to the stub's end. Measure [42658044225](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42658044225) (the scorer on commit `01ecb7c4`, as the probe scorer, on Revit's export of the two office files), the same scorer on the two office files' rvt-rs export from Measure [42655806838](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42655806838) (`main` at `060e8f0a`, `keep_ifc`), and, locally, on Snowdon Towers' Revit export and on rvt-rs's export of it made by the CI-built binary of Measure [42649417280](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42649417280)
**Status:** positive: at all 20 stub ends of the office's 9 walls with one, on both files, Revit's top is the loop's top edge carried on, and rvt-rs's is Revit's.

## 1. The question

A profile-edited wall can stand longer than its sketch lines: its body goes on past an end of the profile by the stretch RE-358 calls a stub (0.4101 ft on 8 of the office's walls, half their thickness). rvt-rs drew the stub at the top of the wall's record box, 0.65 ft too high at the wall's end (#1473). RE-358 carries the cut-out on, along the sloped line's slope or level, and measured it by the walls' volumes (Revit's to 1e-5 ft3 on the four walls of the issue) and by plan rows; it did not read Revit's top over the stub, and said that no byte of the file states the rule. This report reads it.

## 2. Measured

For each wall with sketch lines, the scorer's samples at the end of the stub are against the loop's top edge there, carried on to the stub's end (`extension`):

| File | Walls with a stub | Stub ends | Revit's top against the carried line, worst | rvt-rs's top against Revit's, worst |
|---|---|---|---|---|
| office as built | 9 (6 sloped at the high end, 2 sloped at the low end, 1 level at both ends) | 10 | 0.0001 ft | 0.0000 ft |
| office at permit | 9 (the same) | 10 | 0.0000 ft | 0.0000 ft |
| Snowdon Towers Architectural (local) | 3 read (4 of its walls with sketch lines are angled and not read) | 4 (a level cut-out carried on at one end of wall 1996082, three flat tops) | 0.0000 ft | 0.0000 ft |

The four office walls of #1473 (2057668, 2057670, 2058499 and 2058501) are at 183.0852 ft as built and 183.0810 at permit at the wall's end, where the carried line gives 183.0851 and 183.0810. Along the loop itself (5 to 8 points a wall) Revit's top is the loop's to 0.0001 ft on every wall but one pair: walls 2056705 and 2056770 as built, whose end Revit steps above 182.087 ft (4.4545 ft at the worst sample, #1474's walls and not the stub's).

The same scorer on `main`'s own export of the two office files (Measure 42655806838) puts rvt-rs's top at each of the 20 ends within 0.0000 ft of Revit's: RE-358's rule is Revit's at every end it carries.

## 3. What it says

Revit's body keeps the top line of the profile across the stretch the profile does not reach, along its slope where the line slopes and level where its edge is level: the line is carried on to the end of the wall's body. Everything the rule needs is in the file: the sketch's lines (RE-97, RE-151) and the record's box that holds the wall's end. No byte says why the body goes on past the profile at an end (RE-358 section 1: the record lists no joined wall at the office's ends, and an exposed neighbour is no discriminator), and the stubs are half the wall's thickness on 11 of the 12 walls measured here (the office's 0.4101 ft on walls 0.8202 ft thick and wall 1429569's 0.7136 on 1.4272, Snowdon's wall 736143's 0.5261 ft on 1.0521 and wall 1263678's 0.1770 on 0.3542; the exception is Snowdon's wall 1996082, whose stubs are 0.4375 and 0.3006 ft on a wall 1.1354 thick), which is why RE-358 bounds the stretch by the thickness. The mechanism RE-380 reads from the pockets, Revit's clipping by the planes of the profile's sloped hull edges, would carry a sloped top line on to the end by itself. It would not carry a notch's level edge, which Snowdon's wall 1996082 shows Revit does at its low end: Revit's top at the stub's end is the notch's level line, 54.5833 ft, where a plane through the hull's edge that leaves the same corner (it rises 1.16 ft for each foot) carried on would give 54.08 ft and the box's top is 63.5 ft. So the rule stays RE-358's: each opening's vertical edge at the end of the profile moves out to the wall's end.

## 4. Not measured

- **A stretch longer than the wall is thick:** no wall of any file has one, so `profile_stub_longer_than_the_wall_is_thick` has no instance and its bound stays a stated heuristic (RE-358 section 7).
- **The end RE-358 refuses on `rac_basic`'s wall 493612** (the line ends at the rectangle's top corner): no Revit export of it in CI, and the local Revit 2018 export of the project does not match the wall for reasons the rule does not touch (RE-358 section 5). The record's box is the line carried to the end there (3.529048 m, RE-358 section 1), which is the same fact from another side.
- **The two angled Snowdon walls (698311, 698364)** and any wall whose profile plane is not along an axis: the scorer reads walls along x or y only.
- **An arc or curved profile.**

## 5. Closing #1473

#1473 asked that the four walls' volumes match Revit's on both office files with no other wall moving, and that the stub's top be read from the bytes, or that a measured negative say what was tested. The volumes did since RE-358 (the four walls to 1e-5 ft3 on both files). The top is now Revit's at every stub end measured, 24 of 24 (section 2), and what it is read from is the sketch's lines and the record's box: no byte states the rule, which RE-358 and section 3 say with what was tested. The cases the measurements do not reach (section 4) have no instance in any file with a Revit export, so they are listed here and in RE-358 section 7 and need no issue of their own until a file has one.

## 6. Reproduce

```bash
# Revit's export of the two office files; the scorer is on a commit on top of the probe's commit (Measure looks a probe scorer up in the tree of -f ref):
gh workflow run measure.yml --ref main -f ref=01ecb7c414921f6206ecb42f44074d1cbee14250 -f base=none -f nordic=true \
    -f models="nordic-office-as-built nordic-office-permit" -f scorers=none \
    -f probe=probe_1607_wall_profile_lines -f probe_scorer=wall_profile_tops_vs_ifc
# rvt-rs's own export of the same files:
gh workflow run measure.yml --ref main -f ref=main -f base=none -f nordic=true \
    -f models="nordic-office-as-built nordic-office-permit" -f scorers=none -f keep_ifc=true
python3 tools/re/wall_profile_tops_vs_ifc.py probe.txt model.ifc     # probe.txt from the first run, model.ifc from the second
```

Snowdon Towers needs the local copies (RE-100): the probe's output from `-f samples=true -f sample_years="2024 2024"` and `python3 tools/re/wall_profile_tops_vs_ifc.py probe.txt "Snowdon Towers Sample Architectural_IFC4.ifc"`.
