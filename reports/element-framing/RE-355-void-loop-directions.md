# RE-355 — The direction of a void's first line says which line Revit tags

**Date:** 2026-11-05
**Issues:** #1408 (the void 3147725), follows [RE-339](RE-339-void-tags.md) (#1165, #1402)
**Artefacts:**

| file | sha256 |
|---|---|
| `ARK_NordicLCA_Office_Concrete_As-Built.rvt` (Revit 2024, CC BY-SA 4.0) | `747120005b4b35c7cb66ec61158aab50ded95e052e172cb387b86cd15b31d906` |
| its Revit IFC4 export, `..._As-Built_Revit.ifc` | `b2f530eba8b4424c4542b98e9a2c7bbcc726367f21b2740f3074e4a6ce917382` |
| `ARK_NordicLCA_Office_Concrete_BuildingPermit.rvt` | `1c12c2058128315edbd65a8bf97f60da971aee8625f21233b8477d9c6165c1d1` |
| its Revit IFC4 export, `..._BuildingPermit_Revit.ifc` | `bc3bd3366a3c413ce74840300a590f59102b740118ef37898e906253b6b7c08d` |
| `2024_Core_Interior.rvt` (magnetar-io/revit-test-datasets, MIT) | `c805df445d613b408e37337765572021265e3f5dfdc7d1fa53b22ba1600b8014` |
| its Revit export as Measure's scorers read it, `2024_Core_Interior_slim.ifc` | `bfdf36ffb0bb768f3409d818403990e64d4c262c6780603be87f8077387ad86d` |
| `Snowdon Towers Sample Architectural.rvt` (Autodesk's sample, Revit 2024) | `33271010919acb2a0d0d040851393a511d86ea7fd9a1f4c054797ae03e5e44a1` |
| Revit's IFC4 export of it, `Snowdon Towers Sample Architectural_IFC4.ifc` (bldrs-ai/test-models, no licence: read locally, never committed) | `ecfcb04e3e818090cf818873fe76fba8b447742bbf4d19dad6d590f09cbb985a` |
| `Snowdon Towers Sample Structural.rvt` (Autodesk's sample, Revit 2024; no Revit export of it is held) | `6dc833f9fe9fa702607265f2e3d581a6d2a928ea56133db8225c9b5e0f79a44f` |

**Probe:** `probes/examples/probe_re355_void_loop_directions.rs`
**Status:** positive, and written. One reading of one bit, the direction a void's first line is stored in, gives Revit's tag on 133 of 133 voids on three files, 3147725 included, and on the one Snowdon file that has a Revit export to check against it; `void_tag` gives way to it where the direction is read and keeps its own tag everywhere else.

## 0. Result

For a void whose loop is straight sketch lines that each meet two others in one cycle, take the line with the lowest ElementId (the first drawn), `a`, and walk the loop counter-clockwise. If `a` is stored running along that walk (its start-to-end direction follows the counter-clockwise order), Revit tags the line that follows `a` on the walk. If it is stored running against it, Revit tags `a`. That is Revit's tag on every one of the 133 voids Revit's own export can check:

| file | voids | `a` stored against the walk (tag `a`) | `a` stored along it (tag the next line) | rule right |
|---|---:|---:|---:|---:|
| office as built | 36 | 27 | 9 | 36 |
| office at building permit | 35 | 27 | 8 | 35 |
| Core Interior | 62 | 62 | 0 | 62 |

Of the 9 and 8 on the office, the next line is the highest ElementId on 8 and 7 (the clockwise loops of RE-339 §1, four lines each and two notch loops of six whose notch was drawn at the closing corner) and is the fourth lowest on 1 and 1: 3147725, the loop of six with its notch drawn in the middle. Its lines 3147722 to 3147725 are the rectangle, stored one after another along the walk, and the notch lines 3147758 and 3147788 are stored against it. Revit's tag, 3147725, is the line after 3147722 on the counter-clockwise walk.

RE-339's rule gives Revit's tag on 69 of the office's 71 floor voids (35 of 36 and 34 of 35): all but 3147725 in both files. This one gives it on all 71, and on all 62 of Core Interior.

Against another file with a Revit export, Autodesk's Snowdon Towers architectural sample (Revit's IFC4 export of it, read locally, no licence, nothing of it committed): rvt-rs writes 12 floor voids whose host is in that export, 11 of them with a tag Revit's export has on that host before this change and 12 of 12 after it; one tag moved and it now equals Revit's, none left Revit's. The tag moved in Measure run 42122776008 (6d3f6c73 against a clean `main`); the 11 of 12 and 12 of 12 were scored locally, on the exports Measure run 42125396606 kept, against Revit's export. On Snowdon Towers structural, 3 moved, with no export to check them.

## 1. What was read

Measure runs of `probe_re355_void_loop_directions` (it printed `probe_re352_...` in the first), each with `-f base=none -f scorers=none -f probe=<probe> -f probe_scorer=-`:

- [42115473363](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42115473363) at 03d8f459, `-f nordic=true -f models="nordic-office-as-built nordic-office-permit core-interior"`: the office pair. Its Core Interior leg took `2024_Core_Interior.ifc` before the slim export and found one void; the probe now takes `_slim.ifc`, `_Revit.ifc`, then `.ifc`.
- [42116875940](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42116875940) at ff87e5a2, `-f models=core-interior`: Core Interior from the slim export, 63 unfilled openings, 62 off a wall (42 floor and 20 shading-device voids), Revit's tag the lowest id on all 62.
- [42118790534](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42118790534) at 8d28136e, the three models again with each loop printed line by line: the rank of each line by ElementId and whether it is stored along (`f`) or against (`b`) the counter-clockwise walk, from the lowest line on. The table of section 0 is read from it (the `cycle` column).

A loop is the sketch lines of the void's host that meet end to end (their `GLine` ends within 0.002 ft); it is read only where each line meets exactly two others in one cycle of three or more.

## 2. The readings that were tried

Three readings of the direction, each against Revit's tag:

Right of the voids the reading reads:

| reading | office as built | office at permit | Core Interior |
|---|---:|---:|---:|
| the lowest line's stored direction (this report) | 36 of 36 | 35 of 35 | 62 of 62 |
| every line stored one way round (`stored`) | 33 of 33 | 32 of 32 | 42 of 42 |
| the second-lowest line next to the lowest, which side (`ids`) | 35 of 35 | 35 of 35 | 42 of 62 |

- `stored` is never wrong and reads no loop that has a line stored against the rest: not the notch loops (3147725 among them) and not Core Interior's 20 loops of 27 lines. Where it reads, stored clockwise gives tag `a` on every void and stored counter-clockwise the line after `a` (the highest on a four-line loop).
- `ids` reads the direction in which the first two lines were drawn. It is right on all 70 office voids it reads, 3147725 included, and wrong on the 20 loops of 27 lines of Core Interior (the shading devices): it says clockwise there and predicts the line after the lowest, and Revit tags the lowest. It does not ship.
- The lowest line's stored direction is what `stored` reads on the loops that chain, and on a loop that does not it is still one bit of one line. On the 20 loops of 27 lines the lowest line is stored against the walk, as all 62 of Core Interior's are, and Revit tags it.

## 3. Exercised on how many models

From Measure run 42122779912 (6d3f6c73), counts of loops read (a void's loop of straight lines, each meeting two others) by branch, on the models that have a Revit export to check the tag against:

| model | loops read | first line against the walk (tag `a`) | along it (the next line) | direction not read | tag right |
|---|---:|---:|---:|---:|---:|
| BIM4LCA office as built | 36 | 27 | 9 | 0 | 36 |
| BIM4LCA office at building permit | 35 | 27 | 8 | 0 | 35 |
| Core Interior | 62 | 62 | 0 | 0 | 62 |
| BIM4LCA office terrain, concrete and timber | 0 | 0 | 0 | 0 | 0 |

So the against branch is exercised on 3 models and the along branch on 2 (the office's two saves), of the 5 with a Revit export in that run (the two terrain offices hold no void off a wall); and no loop read ended with the direction unread. Snowdon architectural's one moved tag and the 3 on Snowdon structural are not split by branch: those runs kept no per-void reading. The 19 held-out and spent exports are byte for byte `main`'s with the rule (Measure run 42122776008), so it moved no held-out tag.

The held-out and spent models, as counts (Measure run 42129836920, db0bb07b, dispatched from the branch so that its scrub script keeps the probe's count line; 18 held-out models and 1 spent pair): 11 were read, and each holds 0 loops (no void off a wall in its Revit export whose sketch lines the probe read), so the rule is exercised on no held-out model, 0 right of 0 read where an export can say; 8 were not read, because the probe's native reader is not measured on Revit 2018 to 2022 (7 held-out models and the spent pair).

Measured on 2024 only. Every export that moved is a Revit 2024 file (the two office files and the two Snowdon samples). No Revit 2025 export holds a floor void: the 23 puzzbobb models with a Revit export hold no unfilled opening in it, and RE1 Architecture's three are not floor voids. No export of 2025 to 2027 moved (the samples of those years are byte for byte `main`'s, Measure run 42131336084), so nothing here says what the rule does on those releases.

What the exporter exercises. The table above is the probe's reading, from each line's `GLine` with a join tolerance of 0.002 ft. The exporter reads a subset of it: it takes the first line's ends from the line's own data (`scan_sketch_curves`), which declines a line whose start parameter is not below its end parameter or whose direction is not a unit vector (`bounded_line_at`) and gives no line where the first record is an arc (`first_sketch_curve`); it holds those ends against the ring to 0.005 ft; and a ring whose collinear lines were merged (an edge with two lines) does not read. Where it declines, `void_tag`'s tag stands. A read that agrees with `void_tag` leaves no trace in the export, so what the exporter visibly exercises is the 6 voids whose tag moved (3147725 on each office file, 1 on Snowdon architectural, 3 on Snowdon structural).

## 4. What is not shown

- Why is unread: nothing here says why Revit begins the loop where it does. The finding restates RE-339 §1 with the direction taken from one line instead of from all of them.
- The notch drawn in the middle of a loop is one void in two saves of one file. The rule for a first line stored along the walk rests on 9 loops of the office (six of four lines, two notch loops of six with the notch at the closing corner and 3147725).
- Core Interior exercises only the other branch (`a` stored against the walk): 42 four-line loops and 20 loops of 27 lines.
- Every line read is a straight `GLine`; a curve in a loop is not read. A loop where two lines lie on one edge is not in the data. A line of zero length has no direction, and the probe does not say what it would do.
- The held-out models exercise neither branch: 11 read hold no loop, 8 are not read (§3), and the rule moved none of their 19 exports. What the rule does on a held-out void is not measured.
- 3 voids on Snowdon structural moved, with no export to check them; none of the moved tags there is shown right or wrong.

## 5. What changed in the exporter

`first_line_tag` ([`element_record_plan_profiles.rs`](../../src/research/element_record_plan_profiles.rs)) is the rule, and `tag_voids` uses it where it reads and `void_tag` (RE-339) everywhere else. It reads only where the loop's ring has one straight line on every edge (three or more) and the first line's stored ends lie at the two vertices of its edge, in one order only, to within 0.005 ft; a curve, two lines on one edge, an edge with none or no stored ends keep `void_tag`'s tag. The first line's ends are read from its own data (`scan_sketch_curves`, the same start and end the probe's `GLine` gives) once the floor profiles are solved, in `slabs.rs`, and only where the exact sketch reads are on.

What moved: in Measure run 42122776008 (6d3f6c73 against a clean `main`, 54 exports) and again in run 42131336084 (e77ef0dd, which merges `main`, against `main`, every scorer, 111 exports: the 24 Revit 2025 models, the held-out and spent models and every sample among them), 4 exports moved and 107 are byte for byte `main`'s. The 4 are 3147725 on the office as built and at building permit, 1 void on Snowdon architectural and 3 on Snowdon structural. The scorer diff of the unfiltered run is in the two office files only: `none in IfcSlab` written with Revit's tag goes from 35 of 36 to 36 of 36 as built and from 34 of 35 to 35 of 35 at permit, with box, name, `PredefinedType` and `Reference` equal on all of them, the one `none in IfcSlab` extra (Tag 3147722 on 1457785) is gone and the openings written that Revit's export does not go from 9 to 8 in each file; the census gains the matched opening's quantity, which Revit's export writes and rvt-rs does not (one more missing `Qto_OpeningElementBaseQuantities.Area` in each file), as RE-339 §4 said of its matches.

A moved tag moves GlobalIds. `step_writer.rs` keys the opening's id on the tag (`gid(["void-opening", host_gid, tag])`), and the ids of its `IfcRelVoidsElement`, its `Pset_OpeningElementCommon` and its defining relation follow it, so 4 ids move on each of the 6 moved voids; RE-339 said the same of its tags. The id is not redesigned here.

The tagging also runs for the other elements whose sketches `sketch_plan_profiles` solves, roofs (`roofs.rs`), ceilings and pads (`slabs.rs`), where only the decoded field changes: only a `Floor` exports its voids' tags (`export_content.rs`, the `Floor` filter on `void_openings`), so no export of theirs moves.

Counted: before this change 3147725 was the one floor void in each office file whose Tag is not Revit's (1 of 36 as built, 1 of 35 at permit; `extra: none in IfcSlab` in `unfilled_openings_vs_ifc`, RE-339 §4), and on Core Interior none of 62; after it, none of the 133 and Snowdon architectural 12 of 12.

## 6. Reproduce

`<sha>` is a commit, never a branch.

```bash
gh workflow run measure.yml --ref main -f ref=<sha> -f base=none -f nordic=true \
  -f models="nordic-office-as-built nordic-office-permit core-interior" -f scorers=none \
  -f probe=probe_re355_void_loop_directions -f probe_scorer=-
```

Each void prints one line, `void <tag> host <host> lines <n> tag <lowest|highest|other> ids <ccw|cw|?> stored <ccw|cw|nochain> pred_ids <id|-> pred_stored <id|-> cycle <rank><f|b> ...`, and the last line of each model holds the counts (`void tags:`) and no id. The first token of `cycle` is rank 0, the lowest line: `0b` is tag `a`, `0f` is the line of the second token.
